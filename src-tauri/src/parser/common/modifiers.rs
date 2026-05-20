//! Группа 10: Модификаторы (modifiers/, modifier_definitions/, opinion_modifiers/,
//! dynamic_modifiers/, resistance_activity/, resistance_compliance_modifiers/).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
