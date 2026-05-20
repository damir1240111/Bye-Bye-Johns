//! Группа 7: Войска и снаряжение (units/, unit_tags/, ai_equipment/, ai_navy/,
//! ai_templates/, equipment_groups/, combat_tactics.txt, acclimatation.txt).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
