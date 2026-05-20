//! Кроссфайловая ("синергетическая") валидация общих групп common-парсера.
//!
//! Каждая семантическая группа отвечает за свой формат внутри одного файла.
//! Здесь живут проверки, которые требуют **сопоставления** между файлами и
//! группами — то, что не может проверить одиночный валидатор.
//!
//! Сейчас реализована синергия только для группы 1 (Countries). По мере
//! реализации остальных групп сюда будут добавляться их синергетические
//! проверки.

use std::collections::HashMap;
use std::path::Path;

use crate::parser::common::countries::CountriesData;
use crate::parser::common::{self, CommonGroup, CommonTyped};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CountriesSynergyError {
    pub file: String,
    pub line_number: usize,
    pub tag: String,
    /// Тип ошибки. Стабильный машинный код, удобный для группировки на фронте:
    /// `MissingCountryFile`, `OrphanColorEntry`, `OrphanAliasReference`,
    /// `DuplicateTagAcrossFiles`, `MultipleTagsSameFile`.
    pub error_type: String,
    pub severity: String, // "Error" | "Warning"
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct CountriesSynergyReport {
    pub errors: Vec<CountriesSynergyError>,
}

/// Кроссфайловая валидация группы 1.
///
/// Все пути считаются относительными к корню мода (или установки HOI4) — то
/// есть путь, по которому файл откроется через `parse_file`. Пути, по которым
/// `parse_file` вернул не подходящий тип `CountriesData`, тихо игнорируются:
/// этот валидатор не отвечает за классификацию файлов, классификацией уже
/// занимается ядро `common`. Если вызывающий код ошибся с классификацией — мы
/// просто получаем меньше данных, а не неправильный отчёт.
pub fn validate_countries_synergy(
    country_tag_paths: Vec<String>,
    country_def_paths: Vec<String>,
    colors_paths: Vec<String>,
    alias_paths: Vec<String>,
) -> Result<CountriesSynergyReport, String> {
    let mut report = CountriesSynergyReport::default();

    // ---- Шаг 1: собрать все теги из country_tags/*.txt + проверки 4 и 5 -----
    //
    // Ключ — TAG. Значение — список (file_path, file_value, line_number).
    // По нему детектируем:
    //   * проверка 4: один TAG в нескольких файлах country_tags;
    //   * проверка 5: разные TAG ссылаются на один и тот же file_value.

    let mut tag_to_entries: HashMap<String, Vec<(String, String, usize)>> = HashMap::new();
    let mut file_value_to_tags: HashMap<String, Vec<(String, String, usize)>> = HashMap::new();

    for path in &country_tag_paths {
        let parsed = common::parse_file(path).map_err(|e| {
            format!("synergy: не удалось распарсить '{}': {}", path, e)
        })?;
        if parsed.group != CommonGroup::Countries {
            continue;
        }
        if let Some(CommonTyped::Countries(CountriesData::TagsMap(tags))) = parsed.typed {
            for entry in tags.entries {
                tag_to_entries
                    .entry(entry.tag.clone())
                    .or_default()
                    .push((path.clone(), entry.file_path.clone(), entry.line_number));
                file_value_to_tags
                    .entry(entry.file_path.clone())
                    .or_default()
                    .push((entry.tag.clone(), path.clone(), entry.line_number));
            }
        }
    }

    // Проверка 4: один и тот же TAG в нескольких разных файлах country_tags.
    for (tag, entries) in &tag_to_entries {
        let unique_files: std::collections::BTreeSet<&str> =
            entries.iter().map(|(f, _, _)| f.as_str()).collect();
        if unique_files.len() > 1 {
            // Сообщаем только об одной записи (первой по сортировке файлов),
            // но в сообщение включаем список всех файлов, где встречается тег.
            let files_list: Vec<&str> = unique_files.into_iter().collect();
            let mut sorted = entries.clone();
            sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
            let first = &sorted[0];
            report.errors.push(CountriesSynergyError {
                file: first.0.clone(),
                line_number: first.2,
                tag: tag.clone(),
                error_type: "DuplicateTagAcrossFiles".to_string(),
                severity: "Error".to_string(),
                message: format!(
                    "Строка {}: тег '{}' определён в нескольких файлах country_tags: {}",
                    first.2,
                    tag,
                    files_list.join(", ")
                ),
            });
        }
    }

    // Проверка 5: несколько разных TAG ссылаются на один и тот же файл.
    for (file_value, tags) in &file_value_to_tags {
        let unique_tags: std::collections::BTreeSet<&str> =
            tags.iter().map(|(t, _, _)| t.as_str()).collect();
        if unique_tags.len() > 1 {
            let tags_list: Vec<&str> = unique_tags.into_iter().collect();
            let mut sorted = tags.clone();
            sorted.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
            let first = &sorted[0];
            report.errors.push(CountriesSynergyError {
                file: first.1.clone(),
                line_number: first.2,
                tag: first.0.clone(),
                error_type: "MultipleTagsSameFile".to_string(),
                severity: "Warning".to_string(),
                message: format!(
                    "Строка {}: разные теги {} ссылаются на один и тот же файл '{}'",
                    first.2,
                    tags_list.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(", "),
                    file_value
                ),
            });
        }
    }

    // ---- Шаг 2: индекс известных стран по списку реальных файлов -----------
    //
    // Чтобы проверка 1 не зависела от регистра и разделителей путей, индекс
    // строится по «нормализованному» относительному пути — `countries/<file>`
    // (lowercase). country_tags хранит путь как `"countries/<X>.txt"`, и в
    // переданных нам путях `country_def_paths` мы тоже ищем последние два
    // сегмента.

    let mut available_country_files: std::collections::BTreeSet<String> = Default::default();
    for path in &country_def_paths {
        if let Some(norm) = normalize_country_file_ref(path) {
            available_country_files.insert(norm);
        }
    }

    // ---- Проверка 1: TAG ссылается на отсутствующий countries/<X>.txt ------
    for (tag, entries) in &tag_to_entries {
        for (file, file_value, line) in entries {
            let normalized = file_value.to_ascii_lowercase();
            if !available_country_files.contains(&normalized) {
                report.errors.push(CountriesSynergyError {
                    file: file.clone(),
                    line_number: *line,
                    tag: tag.clone(),
                    error_type: "MissingCountryFile".to_string(),
                    severity: "Error".to_string(),
                    message: format!(
                        "Строка {}: тег '{}' ссылается на файл '{}', но он отсутствует в списке загруженных countries/",
                        line, tag, file_value
                    ),
                });
            }
        }
    }

    // ---- Шаг 3: colors.txt — теги без определения в country_tags -----------
    for path in &colors_paths {
        let parsed = common::parse_file(path).map_err(|e| {
            format!("synergy: не удалось распарсить '{}': {}", path, e)
        })?;
        if parsed.group != CommonGroup::Countries {
            continue;
        }
        if let Some(CommonTyped::Countries(CountriesData::Colors(colors))) = parsed.typed {
            for entry in colors.entries {
                if !tag_to_entries.contains_key(&entry.tag) {
                    report.errors.push(CountriesSynergyError {
                        file: path.clone(),
                        line_number: entry.line_number,
                        tag: entry.tag.clone(),
                        error_type: "OrphanColorEntry".to_string(),
                        severity: "Warning".to_string(),
                        message: format!(
                            "Строка {}: запись цвета для '{}' в colors.txt не имеет определения в country_tags/",
                            entry.line_number, entry.tag
                        ),
                    });
                }
            }
        }
    }

    // ---- Шаг 4: алиасы — original_tag и fallback должны существовать -------
    for path in &alias_paths {
        let parsed = common::parse_file(path).map_err(|e| {
            format!("synergy: не удалось распарсить '{}': {}", path, e)
        })?;
        if parsed.group != CommonGroup::Countries {
            continue;
        }
        if let Some(CommonTyped::Countries(CountriesData::TagAliases(aliases))) = parsed.typed {
            for alias in aliases.aliases {
                check_alias_ref(
                    &mut report,
                    path,
                    alias.line_number,
                    &alias.tag,
                    "original_tag",
                    alias.original_tag.as_deref(),
                    &tag_to_entries,
                );
                check_alias_ref(
                    &mut report,
                    path,
                    alias.line_number,
                    &alias.tag,
                    "fallback",
                    alias.fallback.as_deref(),
                    &tag_to_entries,
                );
            }
        }
    }

    Ok(report)
}

fn check_alias_ref(
    report: &mut CountriesSynergyReport,
    path: &str,
    line: usize,
    alias_tag: &str,
    field_name: &str,
    referenced: Option<&str>,
    known_tags: &HashMap<String, Vec<(String, String, usize)>>,
) {
    let Some(reference) = referenced else { return };
    if reference.is_empty() {
        return;
    }
    if known_tags.contains_key(reference) {
        return;
    }
    report.errors.push(CountriesSynergyError {
        file: path.to_string(),
        line_number: line,
        tag: alias_tag.to_string(),
        error_type: "OrphanAliasReference".to_string(),
        severity: "Warning".to_string(),
        message: format!(
            "Строка {}: алиас '{}' ссылается через '{}' на '{}', но такого тега нет в country_tags/",
            line, alias_tag, field_name, reference
        ),
    });
}

/// Берёт абсолютный путь к файлу страны и возвращает нормализованное
/// представление вида `countries/<filename>` (в нижнем регистре), чтобы его
/// можно было сравнить с тем, как путь записан в `country_tags/*.txt`.
fn normalize_country_file_ref(path: &str) -> Option<String> {
    let p = Path::new(path);
    let comps: Vec<&str> = p
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let file_name = comps.last().copied()?;
    let parent = if comps.len() >= 2 { Some(comps[comps.len() - 2]) } else { None };
    // colors.txt — это таблица переопределений, а не файл-страна. Исключаем её.
    if file_name.eq_ignore_ascii_case("colors.txt") {
        return None;
    }
    match parent {
        Some(p) if p.eq_ignore_ascii_case("countries") => {
            Some(format!("countries/{}", file_name).to_ascii_lowercase())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// RAII-каркас, создающий временную поддиректорию `common/` с нужными
    /// группами и удаляющий её на выходе.
    struct ScopedCommonDir {
        root: PathBuf,
    }

    impl ScopedCommonDir {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir()
                .join(format!("bbj_synergy_{}_{}", std::process::id(), n));
            std::fs::create_dir_all(root.join("common").join("countries")).unwrap();
            std::fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
            std::fs::create_dir_all(root.join("common").join("country_tag_aliases")).unwrap();
            ScopedCommonDir { root }
        }

        fn write_country(&self, name: &str, body: &str) -> String {
            let path = self.root.join("common").join("countries").join(name);
            std::fs::write(&path, body).unwrap();
            path.to_str().unwrap().to_string()
        }
        fn write_tags(&self, name: &str, body: &str) -> String {
            let path = self.root.join("common").join("country_tags").join(name);
            std::fs::write(&path, body).unwrap();
            path.to_str().unwrap().to_string()
        }
        fn write_colors(&self, body: &str) -> String {
            let path = self.root.join("common").join("countries").join("colors.txt");
            std::fs::write(&path, body).unwrap();
            path.to_str().unwrap().to_string()
        }
        fn write_aliases(&self, name: &str, body: &str) -> String {
            let path = self.root.join("common").join("country_tag_aliases").join(name);
            std::fs::write(&path, body).unwrap();
            path.to_str().unwrap().to_string()
        }
    }

    impl Drop for ScopedCommonDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn happy_path_no_errors() {
        let d = ScopedCommonDir::new();
        let tags = d.write_tags(
            "00_countries.txt",
            r#"
                GER = "countries/Germany.txt"
                ENG = "countries/United Kingdom.txt"
            "#,
        );
        let ger = d.write_country("Germany.txt", r#"color = { 64 64 64 }"#);
        let eng = d.write_country("United Kingdom.txt", r#"color = { 200 50 50 }"#);
        let colors = d.write_colors(
            r#"
                GER = { color = rgb { 1 2 3 } color_ui = rgb { 4 5 6 } }
                ENG = { color = rgb { 7 8 9 } }
            "#,
        );
        let aliases = d.write_aliases(
            "tag_aliases.txt",
            r#"
                SPA = { original_tag = GER fallback = ENG }
            "#,
        );

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger, eng],
            vec![colors],
            vec![aliases],
        )
        .expect("no error");
        assert!(report.errors.is_empty(), "ожидался пустой отчёт: {:#?}", report.errors);
    }

    #[test]
    fn detects_missing_country_file() {
        let d = ScopedCommonDir::new();
        let tags = d.write_tags(
            "00_countries.txt",
            r#"
                GER = "countries/Germany.txt"
                XXX = "countries/DoesNotExist.txt"
            "#,
        );
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger],
            vec![],
            vec![],
        )
        .unwrap();

        let missing: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "MissingCountryFile")
            .collect();
        assert_eq!(missing.len(), 1, "ожидалась одна ошибка MissingCountryFile, отчёт: {:#?}", report.errors);
        assert_eq!(missing[0].tag, "XXX");
        assert_eq!(missing[0].severity, "Error");
    }

    #[test]
    fn detects_orphan_color_entry() {
        let d = ScopedCommonDir::new();
        let tags = d.write_tags("00.txt", r#"GER = "countries/Germany.txt""#);
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);
        let colors = d.write_colors(
            r#"
                GER = { color = rgb { 1 2 3 } }
                ZZZ = { color = rgb { 4 5 6 } }
            "#,
        );

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger],
            vec![colors],
            vec![],
        )
        .unwrap();

        let orphans: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "OrphanColorEntry")
            .collect();
        assert_eq!(orphans.len(), 1, "ожидался один orphan: {:#?}", report.errors);
        assert_eq!(orphans[0].tag, "ZZZ");
        assert_eq!(orphans[0].severity, "Warning");
    }

    #[test]
    fn detects_orphan_alias_reference() {
        let d = ScopedCommonDir::new();
        let tags = d.write_tags("00.txt", r#"GER = "countries/Germany.txt""#);
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);
        let aliases = d.write_aliases(
            "tag_aliases.txt",
            r#"
                SPA = { original_tag = ZZZ fallback = GER }
                SPB = { original_tag = GER fallback = QQQ }
            "#,
        );

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger],
            vec![],
            vec![aliases],
        )
        .unwrap();

        let orphans: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "OrphanAliasReference")
            .collect();
        assert_eq!(orphans.len(), 2, "ожидалось два orphan: {:#?}", report.errors);
        let zzz = orphans.iter().find(|e| e.message.contains("'ZZZ'")).unwrap();
        assert_eq!(zzz.tag, "SPA");
        assert_eq!(zzz.severity, "Warning");
        let qqq = orphans.iter().find(|e| e.message.contains("'QQQ'")).unwrap();
        assert_eq!(qqq.tag, "SPB");
    }

    #[test]
    fn detects_duplicate_tag_across_files() {
        let d = ScopedCommonDir::new();
        let tags1 = d.write_tags("00_a.txt", r#"GER = "countries/Germany.txt""#);
        let tags2 = d.write_tags("99_z.txt", r#"GER = "countries/Germany.txt""#);
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);

        let report = validate_countries_synergy(
            vec![tags1, tags2],
            vec![ger],
            vec![],
            vec![],
        )
        .unwrap();

        let dups: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "DuplicateTagAcrossFiles")
            .collect();
        assert_eq!(dups.len(), 1, "ожидался один dup: {:#?}", report.errors);
        assert_eq!(dups[0].tag, "GER");
        assert_eq!(dups[0].severity, "Error");
        // В сообщении должны быть оба файла.
        assert!(dups[0].message.contains("00_a.txt"));
        assert!(dups[0].message.contains("99_z.txt"));
    }

    #[test]
    fn does_not_flag_same_tag_in_same_file() {
        // Один и тот же тег, повторённый внутри одного файла — это уже отлавливается
        // парсером per-file как `повторное определение`. Synergy не должен дополнительно
        // ругаться на это как на DuplicateTagAcrossFiles.
        let d = ScopedCommonDir::new();
        let tags = d.write_tags(
            "00.txt",
            r#"
                GER = "countries/Germany.txt"
                GER = "countries/Germany.txt"
            "#,
        );
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger],
            vec![],
            vec![],
        )
        .unwrap();

        let dups: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "DuplicateTagAcrossFiles")
            .collect();
        assert!(dups.is_empty(), "DuplicateTagAcrossFiles не должен срабатывать на дубль внутри одного файла: {:#?}", report.errors);
    }

    /// Полевой тест: запускает synergy против настоящей установки HOI4.
    /// Не падает, если HOI4 не установлен; в остальном — печатает отчёт.
    #[test]
    #[ignore]
    fn test_field_countries_synergy() {
        use std::path::Path;
        let common_dir = Path::new(r"E:\SteamLibrary\steamapps\common\Hearts of Iron IV\common");
        if !common_dir.exists() {
            println!("Пропущено: папка common HOI4 не найдена по пути {:?}", common_dir);
            return;
        }

        // Собираем три набора путей: country_tags/, countries/<X>.txt (без colors.txt),
        // countries/colors.txt, country_tag_aliases/.
        fn collect_txt(dir: &Path) -> Vec<String> {
            let mut out = Vec::new();
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_file() && p.extension().map_or(false, |x| x == "txt") {
                        if let Some(s) = p.to_str() {
                            out.push(s.to_string());
                        }
                    }
                }
            }
            out
        }

        let tags_paths = collect_txt(&common_dir.join("country_tags"));
        let alias_paths = collect_txt(&common_dir.join("country_tag_aliases"));

        let countries_dir = common_dir.join("countries");
        let mut def_paths = Vec::new();
        let mut colors_paths = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&countries_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                if p.extension().map_or(false, |x| x == "txt") {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name.eq_ignore_ascii_case("colors.txt") {
                        if let Some(s) = p.to_str() {
                            colors_paths.push(s.to_string());
                        }
                    } else if let Some(s) = p.to_str() {
                        def_paths.push(s.to_string());
                    }
                }
            }
        }

        println!(
            "Synergy field test: country_tags={}, countries={}, colors={}, aliases={}",
            tags_paths.len(),
            def_paths.len(),
            colors_paths.len(),
            alias_paths.len()
        );

        let report = validate_countries_synergy(tags_paths, def_paths, colors_paths, alias_paths)
            .expect("validate_countries_synergy не должен возвращать Err");

        println!("\n=== ОТЧЁТ SYNERGY COUNTRIES (поле) ===");
        println!("Всего ошибок/предупреждений: {}", report.errors.len());

        let mut by_type: std::collections::BTreeMap<&str, usize> = Default::default();
        for e in &report.errors {
            *by_type.entry(e.error_type.as_str()).or_insert(0) += 1;
        }
        for (t, c) in &by_type {
            println!("  {}: {}", t, c);
        }
        println!("\nПервые 10 записей:");
        for e in report.errors.iter().take(10) {
            println!("- [{}] {} :: {}", e.severity, e.error_type, e.message);
        }
        println!("=====================================\n");
    }

    #[test]
    fn detects_multiple_tags_same_file() {
        let d = ScopedCommonDir::new();
        let tags = d.write_tags(
            "00.txt",
            r#"
                GER = "countries/Germany.txt"
                XXX = "countries/Germany.txt"
            "#,
        );
        let ger = d.write_country("Germany.txt", r#"color = { 1 2 3 }"#);

        let report = validate_countries_synergy(
            vec![tags],
            vec![ger],
            vec![],
            vec![],
        )
        .unwrap();

        let warns: Vec<_> = report.errors.iter()
            .filter(|e| e.error_type == "MultipleTagsSameFile")
            .collect();
        assert_eq!(warns.len(), 1, "ожидался один warn: {:#?}", report.errors);
        assert_eq!(warns[0].severity, "Warning");
        assert!(warns[0].message.contains("'GER'"));
        assert!(warns[0].message.contains("'XXX'"));
        assert!(warns[0].message.contains("countries/Germany.txt"));
    }
}
