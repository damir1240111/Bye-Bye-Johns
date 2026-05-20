//! Группа 6: Технологии и доктрины (technologies/, doctrines/,
//! technology_sharing/, technology_tags/, special_projects/,
//! military_industrial_organization/).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
