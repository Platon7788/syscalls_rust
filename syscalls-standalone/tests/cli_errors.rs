use std::{fs, process::Command};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_syscalls-standalone"))
}

#[test]
fn help_and_invalid_options_have_documented_exit_codes() {
    let help = binary().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("usage:"));
    for args in [vec!["--out"], vec!["--out", "out", "--lib"], vec!["--typo"]] {
        let output = binary().args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error:"), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
    }
}

#[test]
fn invalid_source_never_creates_output_directory() {
    let root = std::env::temp_dir().join(format!(
        "generator-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("input.rs");
    let out = root.join("output");
    for (input, message) in [
        ("", "no supported syscall declarations"),
        (
            "pub unsafe fn nt_example(value: Missing) -> i32 { 0 }",
            "unknown type Missing",
        ),
        (
            "pub type A = u32; pub type A = u16;",
            "C name collision X_A",
        ),
        (
            "#[repr(C)] pub struct Example { #[cfg(feature = \"extra\")] pub value: u32, }",
            "unsupported attributes on field Example.value",
        ),
    ] {
        fs::write(&source, input).unwrap();
        let output = binary()
            .arg("--lib")
            .arg(&source)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(message), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
        assert!(!out.exists());
    }
    fs::remove_file(source).unwrap();
    fs::remove_dir(root).unwrap();
}

#[test]
fn cli_recovers_a_prepared_transaction_and_reports_positions() {
    let root = std::env::temp_dir().join(format!(
        "generator-recovery-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let stage = root.join(".syscalls-write-lock");
    fs::create_dir(&stage).unwrap();
    fs::write(
        stage.join("manifest"),
        "SYSCALLS-TRANSACTION-1\nexample.h\t1\n",
    )
    .unwrap();
    fs::write(stage.join("old-0"), "original").unwrap();
    fs::write(stage.join("new-0"), "replacement").unwrap();
    fs::write(root.join("example.h"), "replacement").unwrap();
    let result = binary().arg("--recover").arg(&root).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("example.h")).unwrap(),
        "original"
    );
    assert!(!stage.exists());
    let source = root.join("input.rs");
    fs::write(&source, "\n\npub type Example = Missing;").unwrap();
    let result = binary()
        .arg("--out")
        .arg(root.join("unused"))
        .arg("--lib")
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("3:10:"));
    for name in ["example.h", "input.rs", ".syscalls-writer.lock"] {
        fs::remove_file(root.join(name)).unwrap();
    }
    fs::remove_dir(root).unwrap();
}
