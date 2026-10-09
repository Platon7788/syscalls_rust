#[path = "transaction.rs"]
mod transaction;
pub use transaction::{recover, write_bundle};
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io};
    use transaction::publish;
    fn sandbox() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "generator-output-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    #[test]
    fn replaces_bundle_and_preserves_unrelated_files() {
        let root = sandbox();
        fs::write(root.join("keep"), "user").unwrap();
        for value in ["old", "new"] {
            write_bundle(&root, &[("a", value.into()), ("b", value.into())]).unwrap();
        }
        assert_eq!(fs::read_to_string(root.join("a")).unwrap(), "new");
        assert_eq!(fs::read_to_string(root.join("b")).unwrap(), "new");
        assert_eq!(fs::read_to_string(root.join("keep")).unwrap(), "user");
        for name in ["a", "b", "keep"] {
            fs::remove_file(root.join(name)).unwrap();
        }
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn failed_install_restores_old_files_and_removes_new_files() {
        let root = sandbox();
        fs::write(root.join("old"), "original").unwrap();
        let mut calls = 0;
        let result = publish(
            &root,
            &[("new", "new".into()), ("old", "replacement".into())],
            |from, to| {
                calls += 1;
                if calls == 2 {
                    Err(io::Error::other("injected install failure"))
                } else {
                    fs::rename(from, to)
                }
            },
        );
        assert!(result.unwrap_err().contains("previous files restored"));
        assert!(!root.join("new").exists());
        assert_eq!(fs::read_to_string(root.join("old")).unwrap(), "original");
        assert!(!root.join(".syscalls-write-lock").exists());
        fs::remove_file(root.join("old")).unwrap();
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn existing_lock_and_directory_target_are_preserved() {
        let root = sandbox();
        fs::create_dir(root.join(".syscalls-write-lock")).unwrap();
        assert!(write_bundle(&root, &[("a", "new".into())]).is_err());
        fs::remove_dir(root.join(".syscalls-write-lock")).unwrap();
        fs::create_dir(root.join("a")).unwrap();
        assert!(write_bundle(&root, &[("a", "new".into())]).is_err());
        assert!(root.join("a").is_dir());
        fs::remove_dir(root.join("a")).unwrap();
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    #[ignore = "helper launched only by interrupted_process_is_recoverable"]
    fn interruption_child() {
        let root = std::env::var_os("SYSCALLS_TEST_TRANSACTION_DIR").unwrap();
        let after: usize = std::env::var("SYSCALLS_TEST_INTERRUPT_AFTER")
            .unwrap()
            .parse()
            .unwrap();
        let mut installed = 0;
        publish(
            std::path::Path::new(&root),
            &[("existing", "replacement".into()), ("added", "new".into())],
            |from, to| {
                fs::rename(from, to)?;
                installed += 1;
                if installed == after {
                    std::process::exit(73);
                }
                Ok(())
            },
        )
        .unwrap();
        panic!("child did not interrupt");
    }

    #[test]
    fn interrupted_process_is_recoverable() {
        for after in [1, 2] {
            let root = sandbox();
            fs::write(root.join("existing"), "original").unwrap();
            fs::write(root.join("keep"), "user file").unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "output::tests::interruption_child", "--ignored"])
                .env("SYSCALLS_TEST_TRANSACTION_DIR", &root)
                .env("SYSCALLS_TEST_INTERRUPT_AFTER", after.to_string())
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(73));
            assert!(root.join(".syscalls-write-lock/manifest").exists());
            // Recovery must refuse to overwrite an edit made after interruption.
            fs::write(root.join("existing"), "user edit").unwrap();
            assert!(
                recover(&root)
                    .unwrap_err()
                    .contains("modified after interruption")
            );
            assert_eq!(
                fs::read_to_string(root.join("existing")).unwrap(),
                "user edit"
            );
            fs::write(root.join("existing"), "replacement").unwrap();
            recover(&root).unwrap();
            assert_eq!(
                fs::read_to_string(root.join("existing")).unwrap(),
                "original"
            );
            assert_eq!(fs::read_to_string(root.join("keep")).unwrap(), "user file");
            assert!(!root.join("added").exists());
            assert_eq!(recover(&root).unwrap(), "no interrupted transaction");
            for name in ["existing", "keep", ".syscalls-writer.lock"] {
                fs::remove_file(root.join(name)).unwrap();
            }
            fs::remove_dir(root).unwrap();
        }
    }

    #[test]
    fn recovery_respects_active_writer_and_rejects_invalid_manifest() {
        let root = sandbox();
        let guard = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(root.join(".syscalls-writer.lock"))
            .unwrap();
        guard.try_lock().unwrap();
        assert!(recover(&root).unwrap_err().contains("active"));
        drop(guard);
        let stage = root.join(".syscalls-write-lock");
        fs::create_dir(&stage).unwrap();
        fs::write(
            stage.join("manifest"),
            "SYSCALLS-TRANSACTION-1\n../outside\t0\n",
        )
        .unwrap();
        assert!(recover(&root).unwrap_err().contains("invalid manifest"));
        assert!(stage.join("manifest").exists());
        fs::write(
            stage.join("manifest"),
            "SYSCALLS-TRANSACTION-1\nCON.txt\t0\n",
        )
        .unwrap();
        assert!(recover(&root).unwrap_err().contains("invalid manifest"));
        fs::remove_file(stage.join("manifest")).unwrap();
        fs::remove_dir(stage).unwrap();
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn interrupted_preparation_and_completed_cleanup_are_recoverable() {
        let root = sandbox();
        for terminal in [false, true] {
            let stage = root.join(".syscalls-write-lock");
            fs::create_dir(&stage).unwrap();
            fs::write(stage.join("preparing"), "SYSCALLS-TRANSACTION-1\n").unwrap();
            fs::write(stage.join("new-0"), "partial staging").unwrap();
            if terminal {
                fs::write(stage.join("committed"), "committed\n").unwrap();
                fs::write(root.join("existing"), "completed output").unwrap();
            }
            recover(&root).unwrap();
            assert!(!stage.exists());
        }
        assert_eq!(
            fs::read_to_string(root.join("existing")).unwrap(),
            "completed output"
        );
        fs::remove_file(root.join("existing")).unwrap();
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn partial_markers_can_be_recovered_and_metadata_names_are_reserved() {
        let root = sandbox();
        for name in [".SYSCALLS-WRITER.LOCK", ".SYSCALLS-WRITE-LOCK"] {
            assert!(write_bundle(&root, &[(name, "not metadata".into())]).is_err());
        }
        let stage = root.join(".syscalls-write-lock");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("preparing.tmp"), "partial").unwrap();
        recover(&root).unwrap();
        for marker in ["committed.tmp", "rolled-back.tmp"] {
            fs::create_dir(&stage).unwrap();
            fs::write(stage.join("manifest"), "SYSCALLS-TRANSACTION-1\nfile\t1\n").unwrap();
            fs::write(stage.join("old-0"), "old").unwrap();
            fs::write(stage.join("new-0"), "new").unwrap();
            fs::write(stage.join(marker), "partial").unwrap();
            fs::write(root.join("file"), "new").unwrap();
            recover(&root).unwrap();
            assert_eq!(fs::read_to_string(root.join("file")).unwrap(), "old");
        }
        fs::remove_file(root.join("file")).unwrap();
        fs::remove_file(root.join(".syscalls-writer.lock")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
