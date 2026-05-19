//! Tauri IPC command handlers for interacting with frontend.

use crate::project::ProjectMetadata;

#[tauri::command]
pub fn create_project(name: String, path: String) -> Result<ProjectMetadata, String> {
    // TODO: Initialize a new HOI4 mod project structure on disk
    Ok(ProjectMetadata {
        name,
        version: "0.1.0".to_string(),
        path: std::path::PathBuf::from(path),
        target_hoi4_version: "1.14".to_string(), // Default fallback version
    })
}
