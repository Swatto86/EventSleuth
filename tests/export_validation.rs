//! Integration tests for export pre-flight validation.

use eventsleuth::export::csv_export::validate_export_path;
use std::path::PathBuf;

#[test]
fn validate_export_path_valid_directory() {
    let temp = std::env::temp_dir();
    let path = temp.join("eventsleuth_test_export.csv");
    let result = validate_export_path(&path);
    assert!(result.is_ok(), "Temp dir should be writable: {result:?}");
}

#[test]
fn validate_export_path_nonexistent_directory() {
    let path = PathBuf::from(r"C:\NonExistent_Dir_12345\output.csv");
    let result = validate_export_path(&path);
    assert!(result.is_err(), "Non-existent dir should fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("does not exist"),
        "Should indicate dir missing: {msg}"
    );
}

#[test]
fn validate_export_path_does_not_clobber_predictable_probe_file() {
    let dir = std::env::temp_dir().join("eventsleuth_probe_clobber_test");
    std::fs::create_dir_all(&dir).unwrap();
    let victim = dir.join(".eventsleuth_write_probe");
    std::fs::write(&victim, b"important").unwrap();

    let target = dir.join("out.csv");
    validate_export_path(&target).expect("temp dir should be writable");

    let content = std::fs::read(&victim).expect("pre-existing file must not have been deleted");
    assert_eq!(content, b"important", "pre-existing file must not be truncated");

    let _ = std::fs::remove_file(&victim);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_export_path_no_parent() {
    let path = PathBuf::from("just_a_filename.csv");
    // On Windows this resolves to CWD which should exist, or the parent is ""
    // The function should handle this gracefully
    let result = validate_export_path(&path);
    // Either succeeds (CWD is writable) or fails with a clear message
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("parent") || msg.contains("directory"),
            "Error should mention directory: {msg}"
        );
    }
}
