//! Группа 11: Скрипты и DSL (scripted_effects/, scripted_triggers/, scripted_guis/,
//! scripted_localisation/, scripted_diplomatic_actions/, script_constants/,
//! defines/, script_enums.txt).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
