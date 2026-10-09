//! Recoverable file publication. Snapshots survive process interruption.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

const STAGE: &str = ".syscalls-write-lock";
const GUARD: &str = ".syscalls-writer.lock";

fn regular(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_file() => Ok(true),
        Ok(_) => Err(io::Error::other(format!(
            "non-regular file: {}",
            path.display()
        ))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn lock(out: &Path) -> Result<(PathBuf, fs::File), String> {
    let out = fs::canonicalize(out).map_err(|e| e.to_string())?;
    let path = out.join(GUARD);
    regular(&path).map_err(|e| e.to_string())?;
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| e.to_string())?;
    guard
        .try_lock()
        .map_err(|e| format!("another writer/recovery is active: {e}"))?;
    Ok((out, guard))
}

fn valid_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    !name.is_empty()
        && !reserved
        && name != "."
        && name != ".."
        && !name.eq_ignore_ascii_case(STAGE)
        && !name.eq_ignore_ascii_case(GUARD)
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn durable(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(data)?;
    file.sync_all()
}

fn mark(stage: &Path, name: &str, data: &[u8]) -> io::Result<()> {
    let temporary = stage.join(format!("{name}.tmp"));
    if regular(&temporary)? {
        fs::remove_file(&temporary)?;
    }
    durable(&temporary, data)?;
    fs::rename(temporary, stage.join(name))
}

pub fn write_bundle(out: &Path, files: &[(&str, String)]) -> Result<(), String> {
    publish(out, files, |from, to| fs::rename(from, to))
}

pub(super) fn publish(
    out: &Path,
    files: &[(&str, String)],
    mut install: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), String> {
    let mut names = std::collections::HashSet::new();
    for (name, _) in files {
        if !valid_name(name) || !names.insert(name.to_ascii_lowercase()) {
            return Err(format!("invalid or duplicate output filename: {name}"));
        }
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let (out, _guard) = lock(out)?;
    let stage = out.join(STAGE);
    fs::create_dir(&stage).map_err(|e| {
        format!(
            "cannot reserve {}: {e}; inspect it and use --recover <out> after interruption",
            stage.display()
        )
    })?;
    let result = (|| -> io::Result<()> {
        mark(&stage, "preparing", b"SYSCALLS-TRANSACTION-1\n")?;
        let mut manifest = String::from("SYSCALLS-TRANSACTION-1\n");
        for (i, (name, contents)) in files.iter().enumerate() {
            let old = regular(&out.join(name))?;
            if old {
                durable(&stage.join(format!("old-{i}")), &fs::read(out.join(name))?)?;
            }
            durable(&stage.join(format!("new-{i}")), contents.as_bytes())?;
            manifest.push_str(&format!("{name}\t{}\n", u8::from(old)));
        }
        // No destination is changed before the complete manifest is published.
        durable(&stage.join("manifest.tmp"), manifest.as_bytes())?;
        fs::rename(stage.join("manifest.tmp"), stage.join("manifest"))?;
        for (i, (name, _)) in files.iter().enumerate() {
            let source = stage.join(format!("install-{i}"));
            durable(&source, &fs::read(stage.join(format!("new-{i}")))?)?;
            install(&source, &out.join(name))?;
        }
        mark(&stage, "committed", b"committed\n")?;
        Ok(())
    })();
    if let Err(error) = result {
        return match recover_locked(&out) {
            Ok(_) => Err(format!("write failed; previous files restored: {error}")),
            Err(recovery) => Err(format!(
                "write failed: {error}; recovery failed: {recovery}; snapshots retained at {}",
                stage.display()
            )),
        };
    }
    recover_locked(&out).map(|_| ())
}

pub fn recover(out: &Path) -> Result<String, String> {
    let (out, _guard) = lock(out)?;
    recover_locked(&out)
}

fn recover_locked(out: &Path) -> Result<String, String> {
    recover_io(out).map_err(|e| e.to_string())
}

fn recover_io(out: &Path) -> io::Result<String> {
    let stage = out.join(STAGE);
    match fs::symlink_metadata(&stage) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok("no interrupted transaction".into());
        }
        Ok(m) if m.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "transaction path is not a regular directory",
            ));
        }
        Err(e) => return Err(e),
    }
    // Reject unexpected entries before modifying anything. Cleanup is never recursive.
    let entries: Vec<_> = fs::read_dir(&stage)?.collect::<Result<_, _>>()?;
    for entry in &entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| io::Error::other("invalid transaction filename"))?;
        let numbered = ["old-", "new-", "install-", "restore-"]
            .iter()
            .any(|prefix| {
                name.strip_prefix(prefix)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            });
        if !numbered
            && ![
                "preparing",
                "preparing.tmp",
                "manifest",
                "manifest.tmp",
                "committed",
                "committed.tmp",
                "rolled-back",
                "rolled-back.tmp",
            ]
            .contains(&name.as_str())
        {
            return Err(io::Error::other(format!(
                "unexpected transaction entry {name}; manual inspection required"
            )));
        }
        if !regular(&entry.path())? {
            return Err(io::Error::other("transaction entry disappeared"));
        }
    }
    let committed = regular(&stage.join("committed"))?;
    let rolled_back = regular(&stage.join("rolled-back"))?;
    for (present, marker, expected) in [
        (committed, "committed", b"committed\n".as_slice()),
        (rolled_back, "rolled-back", b"rolled-back\n".as_slice()),
    ] {
        if present && fs::read(stage.join(marker))? != expected {
            return Err(io::Error::other("invalid transaction completion marker"));
        }
    }
    if !regular(&stage.join("manifest"))?
        && !committed
        && !rolled_back
        && entries
            .iter()
            .any(|entry| entry.file_name() != "preparing.tmp")
        && (!regular(&stage.join("preparing"))?
            || fs::read(stage.join("preparing"))? != b"SYSCALLS-TRANSACTION-1\n")
    {
        return Err(io::Error::other(
            "unrecognized/legacy transaction; preserve backups and inspect manually",
        ));
    }
    if regular(&stage.join("manifest"))? && !committed && !rolled_back {
        let text = fs::read_to_string(stage.join("manifest"))?;
        let mut lines = text.lines();
        if lines.next() != Some("SYSCALLS-TRANSACTION-1") {
            return Err(io::Error::other("unsupported transaction manifest"));
        }
        let records: Vec<_> = lines.collect();
        for entry in &entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            for prefix in ["old-", "new-", "install-", "restore-"] {
                if let Some(index) = name.strip_prefix(prefix)
                    && !index.parse::<usize>().is_ok_and(|i| i < records.len())
                {
                    return Err(io::Error::other(
                        "transaction snapshot index is outside the manifest",
                    ));
                }
            }
        }
        let mut names = std::collections::HashSet::new();
        let mut restore = Vec::new();
        for (i, line) in records.into_iter().enumerate() {
            let (name, had_old) = line
                .split_once('\t')
                .ok_or_else(|| io::Error::other("invalid manifest record"))?;
            if !valid_name(name)
                || !names.insert(name.to_ascii_lowercase())
                || !["0", "1"].contains(&had_old)
            {
                return Err(io::Error::other("invalid manifest filename or state"));
            }
            let new = fs::read(stage.join(format!("new-{i}")))?;
            let old = if had_old == "1" {
                Some(fs::read(stage.join(format!("old-{i}")))?)
            } else {
                None
            };
            let path = out.join(name);
            let current = if regular(&path)? {
                Some(fs::read(&path)?)
            } else {
                None
            };
            if current
                .as_ref()
                .is_some_and(|bytes| bytes != &new && Some(bytes) != old.as_ref())
            {
                return Err(io::Error::other(format!(
                    "{name} was modified after interruption; preserving it and all snapshots"
                )));
            }
            restore.push((i, path, old, current.is_some()));
        }
        // All snapshots and targets are checked before the first restore operation.
        for (i, path, old, exists) in restore {
            if let Some(bytes) = old {
                let temporary = stage.join(format!("restore-{i}"));
                if regular(&temporary)? {
                    fs::remove_file(&temporary)?;
                }
                durable(&temporary, &bytes)?;
                fs::rename(temporary, path)?;
            } else if exists {
                fs::remove_file(path)?;
            }
        }
        mark(&stage, "rolled-back", b"rolled-back\n")?;
    }
    // With no manifest, installation never began. With a terminal marker,
    // interrupted cleanup is safe to resume even if some snapshots are gone.
    // Keep terminal markers until all other files are gone.
    for entry in fs::read_dir(&stage)? {
        let entry = entry?;
        if entry.file_name() != "committed"
            && entry.file_name() != "rolled-back"
            && entry.file_name() != "preparing"
        {
            fs::remove_file(entry.path())?;
        }
    }
    for marker in ["committed", "rolled-back", "preparing"] {
        if regular(&stage.join(marker))? {
            fs::remove_file(stage.join(marker))?;
        }
    }
    fs::remove_dir(stage)?;
    Ok(if committed {
        "completed transaction cleanup"
    } else {
        "previous files restored; staging removed"
    }
    .into())
}
