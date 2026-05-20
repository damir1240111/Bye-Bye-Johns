//! Группа 14: Дипломатия и спецсистемы (wargoals/, peace_conference/, factions/,
//! collections/, intelligence_agencies/, intelligence_agency_upgrades/).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
