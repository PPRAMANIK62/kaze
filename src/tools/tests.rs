use super::*;
use serde_json::json;

#[tokio::test]
async fn test_registry_with_builtins() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    assert_eq!(registry.len(), 5);
    assert!(!registry.is_empty());
    let defs = registry.definitions();
    assert_eq!(defs.len(), 5);
    assert_eq!(defs[0].name, "read_file");
    assert_eq!(defs[1].name, "glob");
    assert_eq!(defs[2].name, "grep");
    assert_eq!(defs[3].name, "write_file");
    assert_eq!(defs[4].name, "edit");
}

#[tokio::test]
async fn test_read_file_cargo_toml() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("read_file", json!({"path": "Cargo.toml"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("[package]"));
}

#[tokio::test]
async fn test_read_file_nonexistent() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("read_file", json!({"path": "nonexistent_file_xyz.txt"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_path_escape() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("read_file", json!({"path": "../../../etc/passwd"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_glob_rs_files() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("glob", json!({"pattern": "src/**/*.rs"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("main.rs"));
}

#[tokio::test]
async fn test_glob_no_matches() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("glob", json!({"pattern": "**/*.zzzzzzz_impossible"}))
        .await
        .unwrap();
    assert!(result.content.contains("No files matched"));
}

#[tokio::test]
async fn test_unknown_tool() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("nonexistent_tool", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_grep_fn_main() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("grep", json!({"pattern": "fn main", "include": "*.rs"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("main.rs"));
}

#[tokio::test]
async fn test_grep_no_matches() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("grep", json!({"pattern": "^\\d{50}$"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("No matches found"));
}

#[tokio::test]
async fn test_grep_invalid_regex() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry
        .execute("grep", json!({"pattern": "[invalid"}))
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("Invalid regex"));
}

#[tokio::test]
async fn test_write_file_basic() {
    let dir = std::env::temp_dir().join(format!("kaze_test_write_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "write_file",
            json!({"path": "hello.txt", "content": "hello world"}),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("11 bytes"));

    let written = std::fs::read_to_string(dir.join("hello.txt")).unwrap();
    assert_eq!(written, "hello world");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_write_file_creates_parents() {
    let dir = std::env::temp_dir().join(format!("kaze_test_parents_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "write_file",
            json!({"path": "a/b/c/deep.txt", "content": "nested"}),
        )
        .await
        .unwrap();
    assert!(!result.is_error);

    let written = std::fs::read_to_string(dir.join("a/b/c/deep.txt")).unwrap();
    assert_eq!(written, "nested");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_write_file_path_escape() {
    let dir = std::env::temp_dir().join(format!("kaze_test_escape_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "write_file",
            json!({"path": "../../../tmp/evil.txt", "content": "bad"}),
        )
        .await;
    assert!(result.is_err());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_edit_basic() {
    let dir = std::env::temp_dir().join(format!("kaze_test_edit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("file.txt"), "hello world").unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "edit",
            json!({"path": "file.txt", "old_text": "hello", "new_text": "goodbye"}),
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("Edited"));

    let content = std::fs::read_to_string(dir.join("file.txt")).unwrap();
    assert_eq!(content, "goodbye world");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_edit_replace_all() {
    let dir = std::env::temp_dir().join(format!("kaze_test_edit_all_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("repeat.txt"), "aaa bbb aaa bbb aaa").unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "edit",
            json!({"path": "repeat.txt", "old_text": "aaa", "new_text": "ccc", "replace_all": true}),
        )
        .await
        .unwrap();
    assert!(!result.is_error);

    let content = std::fs::read_to_string(dir.join("repeat.txt")).unwrap();
    assert_eq!(content, "ccc bbb ccc bbb ccc");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_edit_text_not_found() {
    let dir = std::env::temp_dir().join(format!("kaze_test_edit_nf_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("file.txt"), "hello world").unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "edit",
            json!({"path": "file.txt", "old_text": "nonexistent", "new_text": "replacement"}),
        )
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("Text not found"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_edit_path_escape() {
    let dir = std::env::temp_dir().join(format!("kaze_test_edit_esc_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "edit",
            json!({"path": "../../../tmp/evil.txt", "old_text": "a", "new_text": "b"}),
        )
        .await;
    assert!(result.is_err());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_edit_multiline() {
    let dir = std::env::temp_dir().join(format!("kaze_test_edit_ml_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("multi.txt"),
        "line one\nline two\nline three\nline four\n",
    )
    .unwrap();

    let registry = ToolRegistry::with_builtins(dir.clone());
    let result = registry
        .execute(
            "edit",
            json!({
                "path": "multi.txt",
                "old_text": "line two\nline three",
                "new_text": "LINE 2\nLINE 3"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error);

    let content = std::fs::read_to_string(dir.join("multi.txt")).unwrap();
    assert_eq!(content, "line one\nLINE 2\nLINE 3\nline four\n");

    std::fs::remove_dir_all(&dir).unwrap();
}
