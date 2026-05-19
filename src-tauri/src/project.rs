//! Project management models and functions for managing mod workspaces.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub target_hoi4_version: String,
}

// TODO: Define project load/save states and configurations
