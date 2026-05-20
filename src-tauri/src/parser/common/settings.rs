//! Группа 12: Интерфейс и настройки (bookmarks/, game_rules/,
//! difficulty_settings/, frontend/, map_modes/, profile_backgrounds/,
//! profile_pictures/, alerts.txt, weather.txt, graphicalculturetype.txt,
//! achievements.txt).
//!
//! Заглушка. Семантическая валидация будет добавлена отдельным изменением.

use crate::parser::common::{CommonError, CommonTyped};
use crate::parser::pdx_script::KeyValuePair;

pub fn validate(_path: &str, _ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    (None, Vec::new())
}
