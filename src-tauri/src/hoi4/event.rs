//! Event definition models (Country events, State events) for Hearts of Iron IV.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub desc: String,
    // TODO: Add triggers, options, effects, is_triggered_only, etc.
}
