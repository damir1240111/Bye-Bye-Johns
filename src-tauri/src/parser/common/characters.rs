//! Группа 2: Персонажи и лидеры (characters/, country_leader/, unit_leader/,
//! scientist_traits/, medals/, unit_medals/, ribbons/, names/).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
