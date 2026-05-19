//! Focus Tree definition models for Hearts of Iron IV.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Focus {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub cost: f32,
    pub relative_to_focus: Option<String>,
    // TODO: Add prerequisites, bypasses, effects, icon, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusTree {
    pub id: String,
    pub focuses: Vec<Focus>,
}
