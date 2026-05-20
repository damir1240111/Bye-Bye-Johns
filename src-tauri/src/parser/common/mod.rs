use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use crate::parser::pdx_script::{self, KeyValuePair};

pub mod countries;
pub mod characters;
pub mod ideas;
pub mod focus;
pub mod events;
pub mod tech;
pub mod units;
pub mod world;
pub mod ai;
pub mod modifiers;
pub mod scripted;
pub mod settings;
pub mod activities;
pub mod diplomacy;

/// Семантическая группа, в которую входит файл common/.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommonGroup {
    Countries,    // 1
    Characters,   // 2
    Ideas,        // 3
    Focus,        // 4
    Events,       // 5
    Tech,         // 6
    Units,        // 7
    World,        // 8
    Ai,           // 9
    Modifiers,    // 10
    Scripted,     // 11
    Settings,     // 12
    Activities,   // 13
    Diplomacy,    // 14
    /// Опознанный, но осознанно не парсимый файл (например, не PDX-формат).
    Skipped,
    /// Файл лежит в common/, но не относится ни к одной из известных групп.
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CommonError {
    pub file: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String, // "Error" | "Warning" | "Info"
    pub group: CommonGroup,
}

/// Типизированная доменная модель, заполняемая группой по мере её реализации.
/// На этапе ядра остаётся пустой; варианты добавляются вместе с каждой
/// семантической группой по отдельности.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum CommonTyped {
    /// Заглушка, пока ни одна группа не вернула типизированные данные.
    None,
    /// Группа 1: данные одного из трёх форматов в countries/, country_tags/,
    /// country_tag_aliases/.
    Countries(countries::CountriesData),
}

/// Результат разбора одного файла common/.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CommonParseResult {
    pub group: CommonGroup,
    pub source_path: String,
    pub ast: Vec<KeyValuePair>,
    pub typed: Option<CommonTyped>,
    pub errors: Vec<CommonError>,
}

/// Карта «имя подпапки common/» → группа. Источник правды о принадлежности.
fn folder_routing() -> &'static HashMap<&'static str, CommonGroup> {
    static MAP: OnceLock<HashMap<&'static str, CommonGroup>> = OnceLock::new();
    MAP.get_or_init(|| {
        use CommonGroup::*;
        let mut m = HashMap::new();

        // 1. Страны и теги
        for f in ["countries", "country_tags", "country_tag_aliases"] {
            m.insert(f, Countries);
        }
        // 2. Персонажи и лидеры
        for f in ["characters", "country_leader", "unit_leader", "scientist_traits",
                  "medals", "unit_medals", "ribbons", "names"] {
            m.insert(f, Characters);
        }
        // 3. Идеи и законы
        for f in ["ideas", "idea_tags", "autonomous_states", "occupation_laws",
                  "bop", "ideologies"] {
            m.insert(f, Ideas);
        }
        // 4. Национальные фокусы
        for f in ["national_focus", "continuous_focus", "focus_inlay_windows"] {
            m.insert(f, Focus);
        }
        // 5. Решения и события
        for f in ["decisions", "on_actions", "mtth"] {
            m.insert(f, Events);
        }
        // 6. Технологии и доктрины
        for f in ["technologies", "doctrines", "technology_sharing", "technology_tags",
                  "special_projects", "military_industrial_organization"] {
            m.insert(f, Tech);
        }
        // 7. Войска и снаряжение
        for f in ["units", "unit_tags", "ai_equipment", "ai_navy", "ai_templates",
                  "equipment_groups"] {
            m.insert(f, Units);
        }
        // 8. Здания, ресурсы, география
        for f in ["buildings", "resources", "state_category", "terrain",
                  "strategic_locations"] {
            m.insert(f, World);
        }
        // 9. ИИ и стратегия
        for f in ["ai_areas", "ai_focuses", "ai_strategy", "ai_strategy_plans",
                  "scorers", "ai_faction_theaters", "generation"] {
            m.insert(f, Ai);
        }
        // 10. Модификаторы
        for f in ["modifiers", "modifier_definitions", "opinion_modifiers",
                  "dynamic_modifiers", "resistance_activity",
                  "resistance_compliance_modifiers"] {
            m.insert(f, Modifiers);
        }
        // 11. Скрипты и DSL
        for f in ["scripted_effects", "scripted_triggers", "scripted_guis",
                  "scripted_localisation", "scripted_diplomatic_actions",
                  "script_constants", "defines"] {
            m.insert(f, Scripted);
        }
        // 12. Интерфейс и настройки
        for f in ["bookmarks", "game_rules", "difficulty_settings", "frontend",
                  "map_modes", "profile_backgrounds", "profile_pictures"] {
            m.insert(f, Settings);
        }
        // 13. Активности
        for f in ["aces", "raids", "abilities", "timed_activities",
                  "operations", "operation_phases", "operation_tokens"] {
            m.insert(f, Activities);
        }
        // 14. Дипломатия и спецсистемы
        for f in ["wargoals", "peace_conference", "factions", "collections",
                  "intelligence_agencies", "intelligence_agency_upgrades"] {
            m.insert(f, Diplomacy);
        }

        m
    })
}

/// Карта «имя верхнеуровневого файла common/*.txt» → группа.
fn top_level_file_routing() -> &'static HashMap<&'static str, CommonGroup> {
    static MAP: OnceLock<HashMap<&'static str, CommonGroup>> = OnceLock::new();
    MAP.get_or_init(|| {
        use CommonGroup::*;
        let mut m = HashMap::new();

        // 5. Решения и события
        m.insert("event_modifiers.txt", Events);
        m.insert("triggered_modifiers.txt", Events);
        // 7. Войска и снаряжение
        m.insert("combat_tactics.txt", Units);
        m.insert("acclimatation.txt", Units);
        // 8. Здания, ресурсы, география
        m.insert("region_colors.txt", World);
        // 9. ИИ и стратегия
        m.insert("ai_attitudes.txt", Ai);
        m.insert("ai_personalities.txt", Ai);
        // 11. Скрипты и DSL
        m.insert("script_enums.txt", Scripted);
        // 12. Интерфейс и настройки
        m.insert("alerts.txt", Settings);
        m.insert("weather.txt", Settings);
        m.insert("graphicalculturetype.txt", Settings);
        m.insert("achievements.txt", Settings);

        // Осознанно пропускаем: не PDX-формат.
        m.insert("msgrdk_achievements.json", Skipped);

        m
    })
}

/// Определяет группу по абсолютному или относительному пути файла.
///
/// Логика: ищем сегмент с именем `common`. Если файл лежит в его
/// непосредственной подпапке (`common/<sub>/...`), используем `<sub>` для
/// поиска в [`folder_routing`]. Если файл лежит прямо в `common/`, ищем
/// его имя в [`top_level_file_routing`]. В остальных случаях — `Unknown`.
pub fn classify_path<P: AsRef<Path>>(path: P) -> CommonGroup {
    let p = path.as_ref();

    let components: Vec<&str> = p
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let common_idx = components
        .iter()
        .rposition(|c| c.eq_ignore_ascii_case("common"));
    let Some(idx) = common_idx else {
        return CommonGroup::Unknown;
    };

    // Случай 1: common/<sub>/...
    if idx + 1 < components.len() - 1 {
        let sub = components[idx + 1];
        if let Some(g) = folder_routing().get(sub) {
            return *g;
        }
        return CommonGroup::Unknown;
    }

    // Случай 2: common/<file>
    if idx + 1 == components.len() - 1 {
        let name = components[idx + 1];
        if let Some(g) = top_level_file_routing().get(name) {
            return *g;
        }
        return CommonGroup::Unknown;
    }

    CommonGroup::Unknown
}

/// Базовый синтаксический разбор файла common/ + диспетчеризация в валидатор группы.
///
/// На этапе ядра каждый групповой `validate` — заглушка, возвращающая пустой
/// список ошибок. Семантическая логика добавляется отдельными изменениями.
pub fn parse_file(path: &str) -> Result<CommonParseResult, String> {
    let group = classify_path(path);
    let mut errors = Vec::new();

    if group == CommonGroup::Skipped {
        return Ok(CommonParseResult {
            group,
            source_path: path.to_string(),
            ast: Vec::new(),
            typed: None,
            errors,
        });
    }

    if group == CommonGroup::Unknown {
        errors.push(CommonError {
            file: path.to_string(),
            line_number: 0,
            message: format!(
                "Файл не относится ни к одной известной группе common/: '{}'",
                path
            ),
            severity: "Warning".to_string(),
            group,
        });
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Не удалось прочитать файл '{}': {}", path, e))?;

    let tokens = pdx_script::lex(&content);
    let ast = match pdx_script::parse_tokens(&tokens) {
        Ok(a) => a,
        Err(e) => {
            let line_number = pdx_script::extract_line_number_from_err(&e).unwrap_or(1);
            errors.push(CommonError {
                file: path.to_string(),
                line_number,
                message: format!("Ошибка синтаксиса: {}", e),
                severity: "Error".to_string(),
                group,
            });
            return Ok(CommonParseResult {
                group,
                source_path: path.to_string(),
                ast: Vec::new(),
                typed: None,
                errors,
            });
        }
    };

    // Диспетчеризация в семантический валидатор группы. На этапе ядра все
    // `validate` — заглушки. Каждая возвращает `(typed, errors)` для своей
    // группы; ядро только склеивает результаты.
    let (typed, mut group_errors) = match group {
        CommonGroup::Countries  => countries::validate(path, &ast),
        CommonGroup::Characters => characters::validate(path, &ast),
        CommonGroup::Ideas      => ideas::validate(path, &ast),
        CommonGroup::Focus      => focus::validate(path, &ast),
        CommonGroup::Events     => events::validate(path, &ast),
        CommonGroup::Tech       => tech::validate(path, &ast),
        CommonGroup::Units      => units::validate(path, &ast),
        CommonGroup::World      => world::validate(path, &ast),
        CommonGroup::Ai         => ai::validate(path, &ast),
        CommonGroup::Modifiers  => modifiers::validate(path, &ast),
        CommonGroup::Scripted   => scripted::validate(path, &ast),
        CommonGroup::Settings   => settings::validate(path, &ast),
        CommonGroup::Activities => activities::validate(path, &ast),
        CommonGroup::Diplomacy  => diplomacy::validate(path, &ast),
        CommonGroup::Skipped | CommonGroup::Unknown => (None, Vec::new()),
    };
    errors.append(&mut group_errors);

    Ok(CommonParseResult {
        group,
        source_path: path.to_string(),
        ast,
        typed,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Создаёт временный путь `<tmp>/bbj_common_test_<n>/common/<sub>/<file>` с
    /// нужным содержимым и возвращает PathBuf файла. Очищает за собой через
    /// RAII-обёртку.
    struct ScopedCommonFile {
        file: std::path::PathBuf,
        root: std::path::PathBuf,
    }

    impl ScopedCommonFile {
        fn new(sub_dir: &str, file_name: &str, content: &[u8]) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir()
                .join(format!("bbj_common_test_{}_{}", std::process::id(), n));
            let dir = root.join("common").join(sub_dir);
            std::fs::create_dir_all(&dir).expect("create_dir_all");
            let file = dir.join(file_name);
            std::fs::write(&file, content).expect("write file");
            ScopedCommonFile { file, root }
        }

        /// Для верхнеуровневого файла (common/*.txt), без подпапки.
        fn new_top_level(file_name: &str, content: &[u8]) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir()
                .join(format!("bbj_common_test_top_{}_{}", std::process::id(), n));
            let dir = root.join("common");
            std::fs::create_dir_all(&dir).expect("create_dir_all");
            let file = dir.join(file_name);
            std::fs::write(&file, content).expect("write file");
            ScopedCommonFile { file, root }
        }

        fn path_str(&self) -> String {
            self.file.to_str().unwrap().to_string()
        }
    }

    impl Drop for ScopedCommonFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn routes_countries_subfolder() {
        assert_eq!(
            classify_path("E:/HOI4/common/countries/GER.txt"),
            CommonGroup::Countries
        );
        assert_eq!(
            classify_path(r"E:\HOI4\common\country_tags\00_countries.txt"),
            CommonGroup::Countries
        );
        assert_eq!(
            classify_path("/mod/common/country_tag_aliases/aliases.txt"),
            CommonGroup::Countries
        );
    }

    #[test]
    fn routes_all_known_groups_via_sample_subfolder() {
        let samples = [
            ("characters/leaders.txt",                      CommonGroup::Characters),
            ("ideas/national_ideas.txt",                    CommonGroup::Ideas),
            ("national_focus/germany.txt",                  CommonGroup::Focus),
            ("decisions/d.txt",                             CommonGroup::Events),
            ("technologies/infantry.txt",                   CommonGroup::Tech),
            ("units/division.txt",                          CommonGroup::Units),
            ("buildings/00_buildings.txt",                  CommonGroup::World),
            ("ai_strategy/x.txt",                           CommonGroup::Ai),
            ("modifiers/m.txt",                             CommonGroup::Modifiers),
            ("scripted_effects/s.txt",                      CommonGroup::Scripted),
            ("bookmarks/bm.txt",                            CommonGroup::Settings),
            ("aces/a.txt",                                  CommonGroup::Activities),
            ("wargoals/w.txt",                              CommonGroup::Diplomacy),
            ("intelligence_agencies/i.txt",                 CommonGroup::Diplomacy),
            ("military_industrial_organization/m.txt",      CommonGroup::Tech),
            ("operations/op.txt",                           CommonGroup::Activities),
        ];
        for (rel, expected) in samples {
            let full = format!("/some/mod/common/{}", rel);
            assert_eq!(classify_path(&full), expected, "path: {}", full);
        }
    }

    #[test]
    fn routes_top_level_txt_files() {
        let samples = [
            ("event_modifiers.txt",         CommonGroup::Events),
            ("combat_tactics.txt",          CommonGroup::Units),
            ("region_colors.txt",           CommonGroup::World),
            ("ai_personalities.txt",        CommonGroup::Ai),
            ("script_enums.txt",            CommonGroup::Scripted),
            ("alerts.txt",                  CommonGroup::Settings),
            ("graphicalculturetype.txt",    CommonGroup::Settings),
        ];
        for (name, expected) in samples {
            let full = format!("/HOI4/common/{}", name);
            assert_eq!(classify_path(&full), expected, "file: {}", name);
        }
    }

    #[test]
    fn routes_skipped_non_pdx_files() {
        assert_eq!(
            classify_path("/HOI4/common/msgrdk_achievements.json"),
            CommonGroup::Skipped
        );
    }

    #[test]
    fn unknown_subfolder_is_reported_but_does_not_fail_parse() {
        let f = ScopedCommonFile::new(
            "totally_unknown_dir",
            "some.txt",
            b"foo = bar\n",
        );
        let result = parse_file(&f.path_str()).expect("parse_file должен вернуть Ok");
        assert_eq!(result.group, CommonGroup::Unknown);
        assert!(
            result.errors.iter().any(|e| e.severity == "Warning"
                && e.message.contains("Файл не относится ни к одной известной группе")),
            "ожидалось warning об Unknown, получено: {:?}",
            result.errors
        );
    }

    #[test]
    fn parse_file_handles_valid_known_group_file() {
        let f = ScopedCommonFile::new(
            "scripted_effects",
            "sample.txt",
            b"some_key = { value = 1 }\n",
        );
        let result = parse_file(&f.path_str()).expect("parse_file должен вернуть Ok");
        assert_eq!(result.group, CommonGroup::Scripted);
        assert!(
            result.errors.is_empty(),
            "не ожидались ошибки, получены: {:?}",
            result.errors
        );
        assert!(!result.ast.is_empty(), "ожидался непустой AST");
    }

    #[test]
    fn parse_file_reports_syntax_error_with_line_number() {
        // Сломанный синтаксис: незакрытая скобка.
        let f = ScopedCommonFile::new(
            "modifiers",
            "broken.txt",
            b"key = {\n  nested = 1\n",
        );
        let result = parse_file(&f.path_str()).expect("parse_file должен вернуть Ok");
        assert_eq!(result.group, CommonGroup::Modifiers);
        assert!(
            result.errors.iter().any(|e| e.severity == "Error"
                && e.message.starts_with("Ошибка синтаксиса")),
            "ожидалась синтаксическая ошибка, получено: {:?}",
            result.errors
        );
    }

    #[test]
    fn parse_file_skipped_file_returns_clean_result() {
        let f = ScopedCommonFile::new_top_level(
            "msgrdk_achievements.json",
            b"{ \"foo\": 1 }\n",
        );
        let result = parse_file(&f.path_str()).expect("parse_file должен вернуть Ok");
        assert_eq!(result.group, CommonGroup::Skipped);
        assert!(result.errors.is_empty(), "у Skipped не должно быть ошибок");
        assert!(result.ast.is_empty(), "у Skipped не должно быть AST");
    }
}
