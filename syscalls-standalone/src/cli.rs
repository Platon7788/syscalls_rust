use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "usage: syscalls-standalone --out <dir> [--lib <lib.rs>]\n       syscalls-standalone --recover <dir>\n       syscalls-standalone --help";

#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Recover { out: PathBuf },
    Generate { out: PathBuf, source: PathBuf },
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter().peekable();
    let mut out = None;
    let mut source = None;
    while let Some(arg) = args.next() {
        if arg == "--recover" {
            let path = args.next().ok_or("--recover requires a directory")?;
            if out.is_some()
                || source.is_some()
                || path.is_empty()
                || path.to_string_lossy().starts_with('-')
                || args.next().is_some()
            {
                return Err("--recover <dir> must be used alone".into());
            }
            return Ok(Command::Recover { out: path.into() });
        }
        if arg == "--help" || arg == "-h" {
            if out.is_some() || source.is_some() || args.peek().is_some() {
                return Err("--help must be used alone".into());
            }
            return Ok(Command::Help);
        }
        let slot = if arg == "--out" || arg == "-o" {
            &mut out
        } else if arg == "--lib" {
            &mut source
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        };
        if slot.is_some() {
            return Err(format!("duplicate option: {}", arg.to_string_lossy()));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{} requires a path", arg.to_string_lossy()))?;
        if value.is_empty() || value.to_string_lossy().starts_with('-') {
            return Err(format!(
                "{} requires a path (prefix paths starting with '-' with ./)",
                arg.to_string_lossy()
            ));
        }
        *slot = Some(PathBuf::from(value));
    }
    Ok(Command::Generate {
        out: out.ok_or("missing --out <dir>")?,
        source: source.unwrap_or_else(|| PathBuf::from("lib.rs")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(OsString::from))
    }
    #[test]
    fn accepts_paths_and_both_option_orders() {
        for args in [
            vec!["--lib", "исходники/lib.rs", "-o", "output folder"],
            vec!["--out", "output folder", "--lib", "исходники/lib.rs"],
        ] {
            assert_eq!(
                command(&args).unwrap(),
                Command::Generate {
                    out: "output folder".into(),
                    source: "исходники/lib.rs".into(),
                }
            );
        }
        assert_eq!(
            command(&["--out", "out"]).unwrap(),
            Command::Generate {
                out: "out".into(),
                source: "lib.rs".into(),
            }
        );
    }
    #[test]
    fn rejects_missing_duplicate_and_unknown_arguments() {
        for args in [
            vec![],
            vec!["--out"],
            vec!["--out", "--lib", "lib.rs"],
            vec!["--out", "out", "--lib"],
            vec!["--out", ""],
            vec!["--out", "out", "--typo"],
            vec!["--out", "a", "-o", "b"],
            vec!["--lib", "a", "--lib", "b", "--out", "out"],
        ] {
            assert!(command(&args).is_err(), "{args:?}");
        }
    }
    #[test]
    fn help_does_not_require_output() {
        assert_eq!(command(&["--help"]).unwrap(), Command::Help);
        assert!(command(&["--help", "--out", "out"]).is_err());
    }

    #[test]
    fn recovery_is_an_exclusive_command() {
        assert_eq!(
            command(&["--recover", "output folder"]).unwrap(),
            Command::Recover {
                out: "output folder".into()
            }
        );
        for args in [
            vec!["--recover"],
            vec!["--recover", ""],
            vec!["--recover", "--out"],
            vec!["--recover", "out", "--lib", "lib.rs"],
            vec!["--out", "out", "--recover", "out"],
        ] {
            assert!(command(&args).is_err(), "{args:?}");
        }
    }
}
