//! Группа 1: Страны и теги (countries/, country_tags/, country_tag_aliases/).
//!
//! Эта группа объединяет четыре разных формата файлов, которые роутер `common`
//! отправляет в один и тот же валидатор. Тип файла определяется по сегменту
//! пути и имени файла:
//!
//! * `countries/<Name>.txt`            — определение одной страны (цвет,
//!                                       графическая культура).
//! * `countries/colors.txt`            — таблица переопределений цветов:
//!                                       `TAG = { color = ... color_ui = ... }`.
//! * `country_tags/*.txt`              — словарь `TAG = "countries/<Name>.txt"`.
//! * `country_tag_aliases/*.txt`       — алиасы тегов с условиями.
//!
//! Каждый формат разбирается в своё представление [`CountriesData`].
//! Кроссфайловые проверки (например, что путь из `country_tags` указывает на
//! существующий файл в `countries/`) сюда не входят — это отдельный
//! синергетический шаг.

use std::collections::HashMap;
use std::path::Path;

use crate::parser::common::{CommonError, CommonGroup, CommonTyped};
use crate::parser::pdx_script::{KeyValuePair, Value};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CountriesData {
    Country(CountryDefinition),
    Colors(CountryColorsFile),
    TagsMap(CountryTagsFile),
    TagAliases(CountryTagAliasesFile),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub struct CountryDefinition {
    pub graphical_culture: Option<String>,
    pub graphical_culture_2d: Option<String>,
    pub color: Option<ColorTriple>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub struct CountryColorsFile {
    pub entries: Vec<CountryColorEntry>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CountryColorEntry {
    pub tag: String,
    pub color: Option<ColorTriple>,
    pub color_ui: Option<ColorTriple>,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ColorTriple {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub kind: ColorKind,
    /// true, если в исходнике все три числа — целые в диапазоне 0..=255;
    /// иначе ожидаются дроби 0.0..=1.0.
    pub byte_scale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColorKind {
    Rgb,
    Hsv,
    /// Префикс не указан — по умолчанию Paradox трактует как RGB.
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct CountryTagsFile {
    pub entries: Vec<TagFileMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TagFileMapping {
    pub tag: String,
    pub file_path: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct CountryTagAliasesFile {
    pub aliases: Vec<CountryTagAlias>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CountryTagAlias {
    pub tag: String,
    pub original_tag: Option<String>,
    pub fallback: Option<String>,
    pub line_number: usize,
}

/// Тип файла внутри группы 1, определённый по сегменту пути.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    CountryDef,
    Colors,
    TagsMap,
    TagAliases,
    Other,
}

fn detect_file_kind(path: &str) -> FileKind {
    let p = Path::new(path);
    let comps: Vec<&str> = p
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let file_name = comps.last().copied().unwrap_or("");
    let parent = if comps.len() >= 2 { Some(comps[comps.len() - 2]) } else { None };

    match parent {
        Some(s) if s.eq_ignore_ascii_case("countries") => {
            if file_name.eq_ignore_ascii_case("colors.txt") {
                FileKind::Colors
            } else {
                FileKind::CountryDef
            }
        }
        Some(s) if s.eq_ignore_ascii_case("country_tags") => FileKind::TagsMap,
        Some(s) if s.eq_ignore_ascii_case("country_tag_aliases") => FileKind::TagAliases,
        _ => FileKind::Other,
    }
}

fn is_valid_country_tag(tag: &str) -> bool {
    tag.len() == 3
        && tag.chars().all(|c| c.is_ascii_alphabetic() && c.is_ascii_uppercase())
}

fn err(path: &str, line: usize, msg: impl Into<String>, severity: &str) -> CommonError {
    CommonError {
        file: path.to_string(),
        line_number: line,
        message: msg.into(),
        severity: severity.to_string(),
        group: CommonGroup::Countries,
    }
}

// --- Общий разбор цветового поля ---------------------------------------------

/// Ищет поле цвета (`color`, `color_ui`, ...) внутри заданного списка
/// `KeyValuePair`-ов и возвращает разобранный `ColorTriple` плюс число
/// потреблённых элементов (для look-ahead'а вокруг разорванного префикса
/// `rgb`/`hsv`).
///
/// Возвращает `Some((triple_opt, consumed))` если поле найдено по индексу
/// `index`, где `triple_opt` — `Some` при успехе и `None` при синтаксической
/// ошибке (которая уже добавлена в `errors`). Возвращает `None`, если по
/// индексу `index` нет поля с таким именем.
///
/// Логика обработки префикса:
///
/// * `key = { r g b }`        — `pair.value` это `Block` → consumed = 1.
/// * `key = rgb { r g b }`    — лексер разбивает на пару `key = rgb` и
///                              следующий анонимный list-item с блоком →
///                              consumed = 2. То же для `hsv`/`HSV`/`RGB`.
fn try_parse_color_field_at(
    path: &str,
    pairs: &[KeyValuePair],
    index: usize,
    key: &str,
    errors: &mut Vec<CommonError>,
) -> Option<(Option<ColorTriple>, usize)> {
    let pair = pairs.get(index)?;
    if pair.key != key {
        return None;
    }

    match &pair.value {
        Value::Block(block) => {
            let triple = parse_color_block(path, block, pair.line_number, errors);
            Some((triple, 1))
        }
        Value::Number(s) => {
            let kind = if s.eq_ignore_ascii_case("rgb") {
                ColorKind::Rgb
            } else if s.eq_ignore_ascii_case("hsv") {
                ColorKind::Hsv
            } else {
                errors.push(err(
                    path,
                    pair.line_number,
                    format!(
                        "Строка {}: поле '{}' должно быть блоком '{{ r g b }}', найдено '{}'",
                        pair.line_number, key, s
                    ),
                    "Error",
                ));
                return Some((None, 1));
            };

            // Ожидаем следующий элемент — анонимный блок.
            match pairs.get(index + 1) {
                Some(next) if next.key.is_empty() => {
                    if let Value::Block(block) = &next.value {
                        let mut triple = parse_color_block(path, block, pair.line_number, errors);
                        if let Some(t) = triple.as_mut() {
                            t.kind = kind;
                        }
                        Some((triple, 2))
                    } else {
                        errors.push(err(
                            path,
                            pair.line_number,
                            format!(
                                "Строка {}: после '{} = {}' ожидался блок '{{ r g b }}'",
                                pair.line_number, key, s
                            ),
                            "Error",
                        ));
                        Some((None, 1))
                    }
                }
                _ => {
                    errors.push(err(
                        path,
                        pair.line_number,
                        format!(
                            "Строка {}: после '{} = {}' ожидался блок '{{ r g b }}'",
                            pair.line_number, key, s
                        ),
                        "Error",
                    ));
                    Some((None, 1))
                }
            }
        }
        other => {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: поле '{}' должно быть блоком '{{ r g b }}', найдено {:?}",
                    pair.line_number, key, other
                ),
                "Error",
            ));
            Some((None, 1))
        }
    }
}

// --- Country (countries/<Name>.txt) -----------------------------------------

fn parse_color_block(
    path: &str,
    block: &[KeyValuePair],
    line: usize,
    errors: &mut Vec<CommonError>,
) -> Option<ColorTriple> {
    // Paradox допускает: `color = { 64 160 167 }`, `color = rgb { 0.2 0.5 0.9 }`,
    // `color = hsv { 0.1 0.5 0.9 }`. В AST `pdx_script` префикс `rgb`/`hsv` мог
    // попасть в один из вариантов:
    //   1. отдельной парой вида `{ rgb { ... } }` — тогда снаружи единственный
    //      элемент-блок с пустым ключом и значением-блоком,
    //   2. блок color уже содержит три числа напрямую.
    // Покрываем оба случая.

    let mut kind = ColorKind::Unspecified;
    let numbers_block: &[KeyValuePair] = if block.len() == 1 && block[0].key.is_empty() {
        // Что-то странное — единственный безымянный элемент. Используем его блок,
        // если он Block, иначе считаем содержимым исходный.
        if let Value::Block(inner) = &block[0].value {
            inner
        } else {
            block
        }
    } else {
        block
    };

    // Если первый элемент — идентификатор `rgb`/`hsv` без `=`, лексер вернёт
    // его как пустой ключ с Value::Number("rgb"). Учитываем и это.
    let mut iter_start = 0usize;
    if let Some(first) = numbers_block.first() {
        if first.key.is_empty() {
            if let Value::Number(s) = &first.value {
                if s.eq_ignore_ascii_case("rgb") {
                    kind = ColorKind::Rgb;
                    iter_start = 1;
                } else if s.eq_ignore_ascii_case("hsv") {
                    kind = ColorKind::Hsv;
                    iter_start = 1;
                }
            }
        }
    }

    let values: Vec<&KeyValuePair> = numbers_block
        .iter()
        .skip(iter_start)
        .filter(|kv| kv.key.is_empty())
        .collect();

    if values.len() != 3 {
        errors.push(err(
            path,
            line,
            format!(
                "Строка {}: блок 'color' должен содержать ровно 3 числа, найдено: {}",
                line,
                values.len()
            ),
            "Error",
        ));
        return None;
    }

    let mut nums = [0f64; 3];
    let mut all_integer_bytes = true;
    for (i, kv) in values.iter().enumerate() {
        let raw = match &kv.value {
            Value::Number(s) => s.clone(),
            other => {
                errors.push(err(
                    path,
                    kv.line_number,
                    format!(
                        "Строка {}: в блоке 'color' ожидалось число, найдено {:?}",
                        kv.line_number, other
                    ),
                    "Error",
                ));
                return None;
            }
        };
        let parsed: f64 = match raw.parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(err(
                    path,
                    kv.line_number,
                    format!(
                        "Строка {}: в блоке 'color' значение '{}' не является числом",
                        kv.line_number, raw
                    ),
                    "Error",
                ));
                return None;
            }
        };
        if raw.contains('.') {
            all_integer_bytes = false;
        }
        nums[i] = parsed;
    }

    let byte_scale = all_integer_bytes && nums.iter().any(|&n| n > 1.0);
    // Диапазоны:
    //   - если byte_scale, ожидаем 0..=255;
    //   - иначе ожидаем 0.0..=1.0.
    for (i, &n) in nums.iter().enumerate() {
        let component = match i { 0 => "R", 1 => "G", _ => "B" };
        if byte_scale {
            if !(0.0..=255.0).contains(&n) {
                errors.push(err(
                    path,
                    line,
                    format!(
                        "Строка {}: компонент {} цвета '{}' вне диапазона 0..=255",
                        line, component, n
                    ),
                    "Error",
                ));
            }
        } else if !(0.0..=1.0).contains(&n) {
            errors.push(err(
                path,
                line,
                format!(
                    "Строка {}: компонент {} цвета '{}' вне диапазона 0.0..=1.0",
                    line, component, n
                ),
                "Error",
            ));
        }
    }

    Some(ColorTriple {
        r: nums[0],
        g: nums[1],
        b: nums[2],
        kind,
        byte_scale,
    })
}

fn parse_identifier_field(
    path: &str,
    pair: &KeyValuePair,
    key: &str,
    errors: &mut Vec<CommonError>,
) -> Option<String> {
    match &pair.value {
        Value::Number(s) | Value::String(s) => {
            if s.trim().is_empty() {
                errors.push(err(
                    path,
                    pair.line_number,
                    format!("Строка {}: пустое значение поля '{}'", pair.line_number, key),
                    "Error",
                ));
                None
            } else {
                Some(s.clone())
            }
        }
        _ => {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: поле '{}' должно быть идентификатором",
                    pair.line_number, key
                ),
                "Error",
            ));
            None
        }
    }
}

fn parse_country_definition(
    path: &str,
    ast: &[KeyValuePair],
    errors: &mut Vec<CommonError>,
) -> CountryDefinition {
    let mut def = CountryDefinition::default();
    let mut seen_any = false;

    let mut i = 0;
    while i < ast.len() {
        // Сначала пробуем разобрать как цветовое поле — это покрывает обе
        // формы (`color = { … }` и `color = rgb { … }`).
        if let Some((triple, consumed)) =
            try_parse_color_field_at(path, ast, i, "color", errors)
        {
            seen_any = true;
            def.color = triple;
            i += consumed;
            continue;
        }

        let pair = &ast[i];
        match pair.key.as_str() {
            "graphical_culture" => {
                seen_any = true;
                def.graphical_culture =
                    parse_identifier_field(path, pair, "graphical_culture", errors);
            }
            "graphical_culture_2d" => {
                seen_any = true;
                def.graphical_culture_2d =
                    parse_identifier_field(path, pair, "graphical_culture_2d", errors);
            }
            _ => {
                // Прочие поля (color_ui, monarchism и т.п.) допустимы и не валидируются
                // на этом этапе. Считаем их «увиденными», чтобы не сообщать о пустом файле.
                if !pair.key.is_empty() {
                    seen_any = true;
                }
            }
        }
        i += 1;
    }

    if !seen_any {
        errors.push(err(
            path,
            1,
            "Файл определения страны не содержит ни одного известного поля \
             (color, graphical_culture, graphical_culture_2d)",
            "Warning",
        ));
    }

    def
}

// --- Colors (countries/colors.txt) ------------------------------------------

fn parse_country_colors(
    path: &str,
    ast: &[KeyValuePair],
    errors: &mut Vec<CommonError>,
) -> CountryColorsFile {
    let mut file = CountryColorsFile::default();
    let mut seen: HashMap<String, usize> = HashMap::new();

    for pair in ast {
        let tag = &pair.key;
        if tag.is_empty() {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: запись без ключа в colors.txt, ожидался формат 'TAG = {{ ... }}'",
                    pair.line_number
                ),
                "Error",
            ));
            continue;
        }
        if !is_valid_country_tag(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: ключ '{}' в colors.txt не является 3-буквенным заглавным тегом",
                    pair.line_number, tag
                ),
                "Warning",
            ));
        }

        let body = match &pair.value {
            Value::Block(b) => b,
            other => {
                errors.push(err(
                    path,
                    pair.line_number,
                    format!(
                        "Строка {}: значение '{}' должно быть блоком '{{ color = ... color_ui = ... }}', найдено {:?}",
                        pair.line_number, tag, other
                    ),
                    "Error",
                ));
                continue;
            }
        };

        // Разбираем поля 'color' и 'color_ui' через общий помощник.
        // Помощник умеет обходить look-ahead для разорванного префикса rgb/hsv.
        let mut entry = CountryColorEntry {
            tag: tag.clone(),
            color: None,
            color_ui: None,
            line_number: pair.line_number,
        };

        let mut j = 0;
        while j < body.len() {
            if let Some((triple, consumed)) =
                try_parse_color_field_at(path, body, j, "color", errors)
            {
                entry.color = triple;
                j += consumed;
                continue;
            }
            if let Some((triple, consumed)) =
                try_parse_color_field_at(path, body, j, "color_ui", errors)
            {
                entry.color_ui = triple;
                j += consumed;
                continue;
            }
            j += 1;
        }

        if entry.color.is_none() {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: запись '{}' в colors.txt не содержит поля 'color'",
                    pair.line_number, tag
                ),
                "Warning",
            ));
        }

        if let Some(prev_line) = seen.get(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: повторная запись для '{}' в colors.txt (ранее в строке {})",
                    pair.line_number, tag, prev_line
                ),
                "Error",
            ));
        } else {
            seen.insert(tag.clone(), pair.line_number);
        }

        file.entries.push(entry);
    }

    file
}

// --- TagsMap (country_tags/*.txt) -------------------------------------------

fn parse_tags_map(
    path: &str,
    ast: &[KeyValuePair],
    errors: &mut Vec<CommonError>,
) -> CountryTagsFile {
    let mut file = CountryTagsFile::default();
    let mut seen: HashMap<String, usize> = HashMap::new();

    for pair in ast {
        let tag = &pair.key;
        if tag.is_empty() {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: запись без ключа в country_tags/, ожидался формат 'TAG = \"path\"'",
                    pair.line_number
                ),
                "Error",
            ));
            continue;
        }

        if !is_valid_country_tag(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: тег '{}' не является 3-буквенным заглавным идентификатором",
                    pair.line_number, tag
                ),
                "Warning",
            ));
            // Не пропускаем запись — добавим её ниже, чтобы покрыть в результате.
        }

        let file_path = match &pair.value {
            Value::String(s) => s.clone(),
            Value::Number(s) => s.clone(),
            other => {
                errors.push(err(
                    path,
                    pair.line_number,
                    format!(
                        "Строка {}: значение тега '{}' должно быть строкой пути, найдено {:?}",
                        pair.line_number, tag, other
                    ),
                    "Error",
                ));
                continue;
            }
        };

        if !file_path.starts_with("countries/") || !file_path.ends_with(".txt") {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: путь '{}' тега '{}' должен иметь вид 'countries/<Имя>.txt'",
                    pair.line_number, file_path, tag
                ),
                "Warning",
            ));
        }

        if let Some(prev_line) = seen.get(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: повторное определение тега '{}' (ранее в строке {})",
                    pair.line_number, tag, prev_line
                ),
                "Error",
            ));
        } else {
            seen.insert(tag.clone(), pair.line_number);
        }

        file.entries.push(TagFileMapping {
            tag: tag.clone(),
            file_path,
            line_number: pair.line_number,
        });
    }

    file
}

// --- TagAliases (country_tag_aliases/*.txt) ---------------------------------

fn extract_string_field(body: &[KeyValuePair], key: &str) -> Option<String> {
    body.iter().find(|kv| kv.key == key).and_then(|kv| match &kv.value {
        Value::String(s) | Value::Number(s) => Some(s.clone()),
        _ => None,
    })
}

fn parse_tag_aliases(
    path: &str,
    ast: &[KeyValuePair],
    errors: &mut Vec<CommonError>,
) -> CountryTagAliasesFile {
    let mut file = CountryTagAliasesFile::default();
    let mut seen: HashMap<String, usize> = HashMap::new();

    for pair in ast {
        let tag = &pair.key;
        if tag.is_empty() {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: запись без ключа в country_tag_aliases/, ожидался формат 'TAG = {{ ... }}'",
                    pair.line_number
                ),
                "Error",
            ));
            continue;
        }
        if !is_valid_country_tag(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: алиас-тег '{}' не является 3-буквенным заглавным идентификатором",
                    pair.line_number, tag
                ),
                "Warning",
            ));
        }

        let body = match &pair.value {
            Value::Block(b) => b,
            other => {
                errors.push(err(
                    path,
                    pair.line_number,
                    format!(
                        "Строка {}: значение алиаса '{}' должно быть блоком '{{ ... }}', найдено {:?}",
                        pair.line_number, tag, other
                    ),
                    "Error",
                ));
                continue;
            }
        };

        let original_tag = extract_string_field(body, "original_tag");
        let fallback = extract_string_field(body, "fallback");

        for (field_name, opt) in [("original_tag", &original_tag), ("fallback", &fallback)] {
            if let Some(v) = opt {
                if !is_valid_country_tag(v) {
                    errors.push(err(
                        path,
                        pair.line_number,
                        format!(
                            "Строка {}: поле '{}' алиаса '{}' содержит '{}' — не похоже на 3-буквенный тег",
                            pair.line_number, field_name, tag, v
                        ),
                        "Warning",
                    ));
                }
            }
        }

        if let Some(prev_line) = seen.get(tag) {
            errors.push(err(
                path,
                pair.line_number,
                format!(
                    "Строка {}: повторное определение алиаса '{}' (ранее в строке {})",
                    pair.line_number, tag, prev_line
                ),
                "Error",
            ));
        } else {
            seen.insert(tag.clone(), pair.line_number);
        }

        file.aliases.push(CountryTagAlias {
            tag: tag.clone(),
            original_tag,
            fallback,
            line_number: pair.line_number,
        });
    }

    file
}

// --- entry ------------------------------------------------------------------

pub fn validate(path: &str, ast: &[KeyValuePair]) -> (Option<CommonTyped>, Vec<CommonError>) {
    let mut errors = Vec::new();
    let kind = detect_file_kind(path);

    let data = match kind {
        FileKind::CountryDef => {
            let def = parse_country_definition(path, ast, &mut errors);
            CountriesData::Country(def)
        }
        FileKind::Colors => {
            let colors = parse_country_colors(path, ast, &mut errors);
            CountriesData::Colors(colors)
        }
        FileKind::TagsMap => {
            let tags = parse_tags_map(path, ast, &mut errors);
            CountriesData::TagsMap(tags)
        }
        FileKind::TagAliases => {
            let aliases = parse_tag_aliases(path, ast, &mut errors);
            CountriesData::TagAliases(aliases)
        }
        FileKind::Other => {
            // Файл попал в группу Countries по верхнеуровневому имени, которого
            // у этой группы нет. На практике этот путь недостижим из текущей
            // конфигурации роутера, но сохраняем семантическую честность:
            // возвращаем пустой Country-definition и предупреждение.
            errors.push(err(
                path,
                0,
                format!(
                    "Файл '{}' попал в группу Countries, но не лежит ни в одной из ожидаемых подпапок",
                    path
                ),
                "Warning",
            ));
            return (None, errors);
        }
    };

    (Some(CommonTyped::Countries(data)), errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::pdx_script;

    fn parse_ast(src: &str) -> Vec<KeyValuePair> {
        let tokens = pdx_script::lex(src);
        pdx_script::parse_tokens(&tokens).expect("valid AST for test fixture")
    }

    fn data_as_country(d: &CommonTyped) -> &CountryDefinition {
        match d {
            CommonTyped::Countries(CountriesData::Country(c)) => c,
            other => panic!("expected Country, got {:?}", other),
        }
    }
    fn data_as_tags(d: &CommonTyped) -> &CountryTagsFile {
        match d {
            CommonTyped::Countries(CountriesData::TagsMap(t)) => t,
            other => panic!("expected TagsMap, got {:?}", other),
        }
    }
    fn data_as_colors(d: &CommonTyped) -> &CountryColorsFile {
        match d {
            CommonTyped::Countries(CountriesData::Colors(c)) => c,
            other => panic!("expected Colors, got {:?}", other),
        }
    }
    fn data_as_aliases(d: &CommonTyped) -> &CountryTagAliasesFile {
        match d {
            CommonTyped::Countries(CountriesData::TagAliases(a)) => a,
            other => panic!("expected TagAliases, got {:?}", other),
        }
    }

    #[test]
    fn detects_file_kinds() {
        assert_eq!(detect_file_kind("/mod/common/countries/GER.txt"), FileKind::CountryDef);
        assert_eq!(detect_file_kind("/mod/common/countries/colors.txt"), FileKind::Colors);
        assert_eq!(detect_file_kind(r"E:\HOI4\common\countries\COLORS.TXT"), FileKind::Colors);
        assert_eq!(detect_file_kind(r"E:\HOI4\common\country_tags\00_countries.txt"), FileKind::TagsMap);
        assert_eq!(detect_file_kind("/common/country_tag_aliases/aliases.txt"), FileKind::TagAliases);
        assert_eq!(detect_file_kind("/common/something_else/x.txt"), FileKind::Other);
    }

    #[test]
    fn country_definition_happy_path_byte_color() {
        let ast = parse_ast(r#"
            graphical_culture = middle_eastern_gfx
            graphical_culture_2d = middle_eastern_2d
            color = { 64 160 167 }
        "#);
        let (typed, errors) = validate("/mod/common/countries/Afghanistan.txt", &ast);
        assert!(errors.is_empty(), "не ожидались ошибки: {:?}", errors);
        let c = data_as_country(typed.as_ref().unwrap());
        assert_eq!(c.graphical_culture.as_deref(), Some("middle_eastern_gfx"));
        assert_eq!(c.graphical_culture_2d.as_deref(), Some("middle_eastern_2d"));
        let col = c.color.as_ref().unwrap();
        assert_eq!((col.r, col.g, col.b), (64.0, 160.0, 167.0));
        assert!(col.byte_scale);
        assert_eq!(col.kind, ColorKind::Unspecified);
    }

    #[test]
    fn country_definition_color_float_with_rgb_prefix() {
        let ast = parse_ast(r#"
            color = rgb { 0.2 0.5 0.9 }
        "#);
        let (typed, errors) = validate("/mod/common/countries/X.txt", &ast);
        assert!(errors.is_empty(), "не ожидались ошибки: {:?}", errors);
        let c = data_as_country(typed.as_ref().unwrap());
        let col = c.color.as_ref().unwrap();
        assert_eq!(col.kind, ColorKind::Rgb);
        assert!(!col.byte_scale);
    }

    #[test]
    fn country_definition_bad_color_component_out_of_range() {
        let ast = parse_ast(r#"
            color = { 999 160 167 }
        "#);
        let (_, errors) = validate("/mod/common/countries/X.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("вне диапазона 0..=255")),
            "ожидалась ошибка о диапазоне: {:?}",
            errors
        );
    }

    #[test]
    fn country_definition_color_wrong_count() {
        let ast = parse_ast(r#"
            color = { 1 2 }
        "#);
        let (_, errors) = validate("/mod/common/countries/X.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("должен содержать ровно 3 числа")),
            "ожидалась ошибка о количестве компонент: {:?}",
            errors
        );
    }

    #[test]
    fn country_definition_warns_on_empty_file() {
        let ast = parse_ast("");
        let (_, errors) = validate("/mod/common/countries/X.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning" && e.message.contains("не содержит ни одного известного поля")),
            "ожидалось предупреждение об отсутствии полей: {:?}",
            errors
        );
    }

    #[test]
    fn tags_map_happy_path() {
        let ast = parse_ast(r#"
            GER = "countries/Germany.txt"
            ENG = "countries/United Kingdom.txt"
        "#);
        let (typed, errors) = validate("/mod/common/country_tags/00_countries.txt", &ast);
        assert!(errors.is_empty(), "не ожидались ошибки: {:?}", errors);
        let t = data_as_tags(typed.as_ref().unwrap());
        assert_eq!(t.entries.len(), 2);
        assert_eq!(t.entries[0].tag, "GER");
        assert_eq!(t.entries[0].file_path, "countries/Germany.txt");
    }

    #[test]
    fn tags_map_warns_on_bad_tag_shape() {
        let ast = parse_ast(r#"
            germany = "countries/Germany.txt"
        "#);
        let (_, errors) = validate("/mod/common/country_tags/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning" && e.message.contains("не является 3-буквенным заглавным")),
            "ожидалось предупреждение о форме тега: {:?}",
            errors
        );
    }

    #[test]
    fn tags_map_warns_on_bad_path() {
        let ast = parse_ast(r#"
            GER = "Germany.txt"
        "#);
        let (_, errors) = validate("/mod/common/country_tags/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning" && e.message.contains("должен иметь вид 'countries/")),
            "ожидалось предупреждение о пути: {:?}",
            errors
        );
    }

    #[test]
    fn tags_map_errors_on_duplicate_tag() {
        let ast = parse_ast(r#"
            GER = "countries/Germany.txt"
            GER = "countries/Other.txt"
        "#);
        let (_, errors) = validate("/mod/common/country_tags/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("повторное определение тега 'GER'")),
            "ожидалась ошибка о дубликате: {:?}",
            errors
        );
    }

    #[test]
    fn tag_aliases_happy_path() {
        let ast = parse_ast(r#"
            SPA = {
                original_tag = SPR
                fallback = GER
            }
        "#);
        let (typed, errors) = validate("/mod/common/country_tag_aliases/x.txt", &ast);
        assert!(errors.is_empty(), "не ожидались ошибки: {:?}", errors);
        let a = data_as_aliases(typed.as_ref().unwrap());
        assert_eq!(a.aliases.len(), 1);
        assert_eq!(a.aliases[0].tag, "SPA");
        assert_eq!(a.aliases[0].original_tag.as_deref(), Some("SPR"));
        assert_eq!(a.aliases[0].fallback.as_deref(), Some("GER"));
    }

    #[test]
    fn tag_aliases_warns_on_non_tag_original() {
        let ast = parse_ast(r#"
            SPA = {
                original_tag = soviet_union
            }
        "#);
        let (_, errors) = validate("/mod/common/country_tag_aliases/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning" && e.message.contains("'original_tag'") && e.message.contains("не похоже на 3-буквенный тег")),
            "ожидалось предупреждение о форме original_tag: {:?}",
            errors
        );
    }

    #[test]
    fn tag_aliases_errors_on_non_block_value() {
        let ast = parse_ast(r#"
            SPA = "broken"
        "#);
        let (_, errors) = validate("/mod/common/country_tag_aliases/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("должно быть блоком")),
            "ожидалась ошибка о не-блоке: {:?}",
            errors
        );
    }

    #[test]
    fn tag_aliases_errors_on_duplicate() {
        let ast = parse_ast(r#"
            SPA = { original_tag = SPR }
            SPA = { original_tag = SPR }
        "#);
        let (_, errors) = validate("/mod/common/country_tag_aliases/x.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("повторное определение алиаса 'SPA'")),
            "ожидалась ошибка о дубликате алиаса: {:?}",
            errors
        );
    }

    // --- colors.txt -------------------------------------------------------

    #[test]
    fn colors_happy_path_with_hsv_and_rgb_prefixes() {
        // Реальный фрагмент HOI4: HSV-префикс в верхнем регистре, rgb в нижнем,
        // оба `color` и `color_ui` присутствуют.
        let ast = parse_ast(r#"
            GER = {
                color = HSV { 0.1 0.15 0.4 }
                color_ui = rgb { 138 155 116 }
            }
            ENG = {
                color = rgb { 201 56 93 }
                color_ui = rgb { 255 73 121 }
            }
        "#);
        let (typed, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(errors.is_empty(), "не ожидались ошибки: {:?}", errors);
        let c = data_as_colors(typed.as_ref().unwrap());
        assert_eq!(c.entries.len(), 2);
        assert_eq!(c.entries[0].tag, "GER");
        let ger_color = c.entries[0].color.as_ref().unwrap();
        assert_eq!(ger_color.kind, ColorKind::Hsv);
        assert!(!ger_color.byte_scale);
        let ger_ui = c.entries[0].color_ui.as_ref().unwrap();
        assert_eq!(ger_ui.kind, ColorKind::Rgb);
        assert!(ger_ui.byte_scale);
    }

    #[test]
    fn colors_warns_when_color_missing() {
        let ast = parse_ast(r#"
            POL = {
                color_ui = rgb { 255 120 138 }
            }
        "#);
        let (_, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning"
                && e.message.contains("'POL'") && e.message.contains("не содержит поля 'color'")),
            "ожидалось предупреждение об отсутствующем color: {:?}",
            errors
        );
    }

    #[test]
    fn colors_warns_on_non_tag_key() {
        let ast = parse_ast(r#"
            germany = {
                color = rgb { 1 2 3 }
            }
        "#);
        let (_, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Warning" && e.message.contains("не является 3-буквенным")),
            "ожидалось предупреждение о форме тега: {:?}",
            errors
        );
    }

    #[test]
    fn colors_errors_on_duplicate_tag() {
        let ast = parse_ast(r#"
            GER = { color = rgb { 1 2 3 } }
            GER = { color = rgb { 4 5 6 } }
        "#);
        let (_, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("повторная запись для 'GER'")),
            "ожидалась ошибка о дубликате: {:?}",
            errors
        );
    }

    #[test]
    fn colors_errors_on_non_block_value() {
        let ast = parse_ast(r#"
            GER = "broken"
        "#);
        let (_, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("должно быть блоком")),
            "ожидалась ошибка о не-блоке: {:?}",
            errors
        );
    }

    #[test]
    fn colors_propagates_color_range_errors() {
        let ast = parse_ast(r#"
            GER = {
                color = rgb { 300 50 50 }
            }
        "#);
        let (_, errors) = validate("/mod/common/countries/colors.txt", &ast);
        assert!(
            errors.iter().any(|e| e.severity == "Error" && e.message.contains("вне диапазона 0..=255")),
            "ожидалась ошибка о диапазоне внутри colors.txt: {:?}",
            errors
        );
    }
}
