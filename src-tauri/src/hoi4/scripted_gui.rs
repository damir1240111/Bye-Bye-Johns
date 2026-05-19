//! Scripted GUI definition models for Hearts of Iron IV.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptedGui {
    pub id: String,
    // TODO: Add scripted GUI properties, window declarations, custom scripts
}
