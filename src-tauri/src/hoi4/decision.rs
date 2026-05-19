//! Decision category and decision definition models for Hearts of Iron IV.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub name: String,
    // TODO: Add cost, trigger, visible, complete_effect, remove_effect, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCategory {
    pub id: String,
    pub decisions: Vec<Decision>,
}
