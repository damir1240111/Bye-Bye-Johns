//! Группа 9: ИИ и стратегия (ai_areas/, ai_focuses/, ai_strategy/,
//! ai_strategy_plans/, scorers/, ai_faction_theaters/, generation/,
//! ai_attitudes.txt, ai_personalities.txt).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
