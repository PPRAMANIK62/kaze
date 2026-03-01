use super::*;
use serde_json::json;

#[tokio::test]
async fn test_registry_with_builtins() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    assert_eq!(registry.len(), 3);
    assert!(!registry.is_empty());
    let defs = registry.definitions();
    assert_eq!(defs.len(), 3);
    assert_eq!(defs[0].name, "read_file");
    assert_eq!(defs[1].name, "glob");
    assert_eq!(defs[2].name, "grep");
}

#[tokio::test]
async fn test_read_file_cargo_toml() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("read_file", json!({"path": "Cargo.toml"})).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("[package]"));
}

#[tokio::test]
async fn test_read_file_nonexistent() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("read_file", json!({"path": "nonexistent_file_xyz.txt"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_path_escape() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("read_file", json!({"path": "../../../etc/passwd"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_glob_rs_files() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("glob", json!({"pattern": "src/**/*.rs"})).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("main.rs"));
}

#[tokio::test]
async fn test_glob_no_matches() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("glob", json!({"pattern": "**/*.zzzzzzz_impossible"})).await.unwrap();
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
    let result = registry.execute("grep", json!({"pattern": "fn main", "include": "*.rs"})).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("main.rs"));
}

#[tokio::test]
async fn test_grep_no_matches() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("grep", json!({"pattern": "^\\d{50}$"})).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("No matches found"));
}

#[tokio::test]
async fn test_grep_invalid_regex() {
    let registry = ToolRegistry::with_builtins(PathBuf::from("."));
    let result = registry.execute("grep", json!({"pattern": "[invalid"})).await.unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("Invalid regex"));
}
