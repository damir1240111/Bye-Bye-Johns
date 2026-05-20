pub mod states;
pub mod countries;
pub mod general;
pub mod units;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HistoryError {
    pub file: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String, // "Error" | "Warning" | "Info"
}
