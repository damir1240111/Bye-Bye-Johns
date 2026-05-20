use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LocalisationEntry {
    pub key: String,
    pub version: Option<u32>,
    pub value: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LocalisationError {
    pub line_number: usize,
    pub message: String,
    pub raw_line: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LocalisationFile {
    pub language: String,
    pub entries: Vec<LocalisationEntry>,
    pub errors: Vec<LocalisationError>,
}

/// Удаляет кавычки и обрабатывает экранированные символы в значении локализации.
fn parse_value(input: &str) -> Result<(String, &str), String> {
    if !input.starts_with('"') {
        return Err("Значение должно начинаться с кавычки (\")".to_string());
    }

    let mut chars = input.chars().skip(1);
    let mut value = String::new();
    let mut escaped = false;
    let mut closed = false;
    let mut bytes_read = 1; // Для открывающей кавычки '"'

    while let Some(c) = chars.next() {
        bytes_read += c.len_utf8();
        if escaped {
            match c {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'r' => value.push('\r'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                _ => {
                    // Если экранирован неизвестный символ, сохраняем как есть
                    value.push('\\');
                    value.push(c);
                }
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            closed = true;
            break;
        } else {
            value.push(c);
        }
    }

    if !closed {
        return Err("Отсутствует закрывающая кавычка (\")".to_string());
    }

    Ok((value, &input[bytes_read..]))
}

/// Распознает заголовок языка (например, "l_english:" или "l_russian: # комментарий")
fn parse_language_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Игнорируем комментарии на той же строке
    let main_part = trimmed.split('#').next()?.trim();
    if main_part.starts_with("l_") && main_part.ends_with(':') {
        let lang = &main_part[2..main_part.len() - 1];
        if !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
            return Some(lang.to_string());
        }
    }
    None
}

/// Парсит файл локализации Hearts of Iron IV по указанному пути.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<LocalisationFile, String> {
    let file_path = path.as_ref();
    let mut errors = Vec::new();

    // Читаем файл с отслеживанием некорректной кодировки (UTF-16 или ANSI вместо UTF-8)
    let raw_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            if let Ok(bytes) = fs::read(file_path) {
                if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
                    errors.push(LocalisationError {
                        line_number: 1,
                        message: "Ошибка кодировки: файл сохранен в формате UTF-16, игра требует UTF-8 с BOM".to_string(),
                        raw_line: String::new(),
                    });
                } else if std::str::from_utf8(&bytes).is_err() {
                    errors.push(LocalisationError {
                        line_number: 1,
                        message: "Ошибка кодировки: файл содержит некорректные символы UTF-8 (возможно, сохранен в кодировке ANSI/Windows-1251)".to_string(),
                        raw_line: String::new(),
                    });
                } else {
                    errors.push(LocalisationError {
                        line_number: 1,
                        message: format!("Ошибка чтения файла: {}", e),
                        raw_line: String::new(),
                    });
                }
            } else {
                errors.push(LocalisationError {
                    line_number: 1,
                    message: format!("Не удалось прочитать файл: {}", e),
                    raw_line: String::new(),
                });
            }
            return Ok(LocalisationFile {
                language: String::new(),
                entries: Vec::new(),
                errors,
            });
        }
    };

    // Проверяем наличие метки UTF-8 BOM
    let has_bom = raw_content.starts_with('\u{FEFF}');
    if !has_bom {
        errors.push(LocalisationError {
            line_number: 1,
            message: "Предупреждение: Отсутствует метка UTF-8 BOM. Игра может проигнорировать этот файл или некорректно отобразить символы".to_string(),
            raw_line: String::new(),
        });
    }

    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mut has_lang_suffix = false;
    let mut expected_lang = String::new();
    if !file_name.is_empty() {
        for lang in &[
            "english",
            "french",
            "german",
            "spanish",
            "braz_por",
            "polish",
            "russian",
            "japanese",
            "simp_chinese",
            "korean",
        ] {
            if file_name.ends_with(&format!("_l_{}.yml", lang)) {
                has_lang_suffix = true;
                expected_lang = lang.to_string();
                break;
            }
        }
        if !has_lang_suffix {
            errors.push(LocalisationError {
                line_number: 1,
                message: format!(
                    "Предупреждение: Имя файла '{}' не соответствует правилам HOI4. Имя файла должно заканчиваться на '_l_<язык>.yml' (например, '_l_russian.yml')",
                    file_name
                ),
                raw_line: String::new(),
            });
        }
    }

    let content = raw_content.strip_prefix('\u{FEFF}').unwrap_or(&raw_content);

    let mut language = String::new();
    let mut entries = Vec::new();
    let mut language_found = false;
    let mut keys_seen = HashSet::new();

    for (zero_indexed_line, line) in content.lines().enumerate() {
        let line_number = zero_indexed_line + 1;
        let trimmed = line.trim();

        // Пропускаем пустые строки и полные комментарии
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Первой значащей строкой должен быть заголовок языка
        if !language_found {
            if let Some(lang) = parse_language_header(line) {
                language = lang;
                language_found = true;

                // Проверяем соответствие заголовка языка и имени файла
                if !expected_lang.is_empty() && language != expected_lang {
                    errors.push(LocalisationError {
                        line_number,
                        message: format!(
                            "Предупреждение: Несоответствие языка. Заголовок указывает на 'l_{}:', но имя файла ожидает язык '{}'",
                            language, expected_lang
                        ),
                        raw_line: line.to_string(),
                    });
                }
                continue;
            } else {
                errors.push(LocalisationError {
                    line_number,
                    message: "Файл должен начинаться с заголовка языка (например, l_english:)"
                        .to_string(),
                    raw_line: line.to_string(),
                });
                // Прекращаем парсинг, так как без языка Paradox-локализация не имеет смысла
                return Ok(LocalisationFile {
                    language,
                    entries,
                    errors,
                });
            }
        }

        // Парсим ключ локализации
        let colon_pos = match line.find(':') {
            Some(pos) => pos,
            None => {
                errors.push(LocalisationError {
                    line_number,
                    message: "Ожидалось двоеточие после ключа локализации".to_string(),
                    raw_line: line.to_string(),
                });
                continue;
            }
        };

        let key_part = &line[..colon_pos];
        let key = key_part.trim();

        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        {
            errors.push(LocalisationError {
                line_number,
                message: format!("Недопустимый ключ локализации: '{}'. Ключ может содержать только латиницу, цифры, точки, дефисы и подчеркивания", key),
                raw_line: line.to_string(),
            });
            continue;
        }

        // Проверка на дубликат ключа
        if !keys_seen.insert(key.to_string()) {
            errors.push(LocalisationError {
                line_number,
                message: format!(
                    "Предупреждение: Дубликат ключа локализации: '{}' уже определен в этом файле",
                    key
                ),
                raw_line: line.to_string(),
            });
        }

        // Парсим версию и значение после двоеточия
        let mut rest = line[colon_pos + 1..].trim_start();
        let mut version = None;

        if let Some(first_char) = rest.chars().next() {
            if first_char.is_ascii_digit() {
                let mut version_str = String::new();
                for c in rest.chars() {
                    if c.is_ascii_digit() {
                        version_str.push(c);
                    } else {
                        break;
                    }
                }
                let version_len = version_str.len();
                if let Ok(v) = version_str.parse::<u32>() {
                    version = Some(v);
                }
                rest = &rest[version_len..];
            }
        }

        rest = rest.trim_start();

        match parse_value(rest) {
            Ok((value, trailing)) => {
                let trailing_trimmed = trailing.trim();
                if !trailing_trimmed.is_empty() && !trailing_trimmed.starts_with('#') {
                    errors.push(LocalisationError {
                        line_number,
                        message: "Лишние символы после закрывающей кавычки".to_string(),
                        raw_line: line.to_string(),
                    });
                    continue;
                }

                entries.push(LocalisationEntry {
                    key: key.to_string(),
                    version,
                    value,
                    line_number,
                });
            }
            Err(err_msg) => {
                errors.push(LocalisationError {
                    line_number,
                    message: err_msg,
                    raw_line: line.to_string(),
                });
            }
        }
    }

    if !language_found && errors.is_empty() {
        errors.push(LocalisationError {
            line_number: 1,
            message: "Файл пуст или в нем отсутствует заголовок языка".to_string(),
            raw_line: String::new(),
        });
    }

    Ok(LocalisationFile {
        language,
        entries,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::test_utils::TempFile;

    fn make_temp_file(content: &[u8]) -> TempFile {
        TempFile::new_with_filename(content, "temp_loc_l_english.yml")
    }


    #[test]
    fn test_happy_path() {
        let content = "\u{FEFF}l_english:\n GER_fascism:0 \"Fascism\"\n  GER_fascism_desc: \"German fascism description\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.language, "english");
        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 2);

        assert_eq!(res.entries[0].key, "GER_fascism");
        assert_eq!(res.entries[0].version, Some(0));
        assert_eq!(res.entries[0].value, "Fascism");
        assert_eq!(res.entries[0].line_number, 2);

        assert_eq!(res.entries[1].key, "GER_fascism_desc");
        assert_eq!(res.entries[1].version, None);
        assert_eq!(res.entries[1].value, "German fascism description");
        assert_eq!(res.entries[1].line_number, 3);
    }

    #[test]
    fn test_utf8_with_bom() {
        // UTF-8 BOM bytes are: EF BB BF
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice("l_russian:\n  RU_communism:10 \"Communism\"\n".as_bytes());
        let file = TempFile::new_with_filename(&content, "temp_loc_l_russian.yml");
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.language, "russian");
        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].key, "RU_communism");
        assert_eq!(res.entries[0].version, Some(10));
        assert_eq!(res.entries[0].value, "Communism");
    }

    #[test]
    fn test_comments_and_spaces() {
        let content = "\u{FEFF}# This is a comment\n\nl_english: # Inline comment\n  # Nested comment\n  KEY_1:0 \"Val 1\" # Comment after val\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.language, "english");
        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].key, "KEY_1");
        assert_eq!(res.entries[0].value, "Val 1");
    }

    #[test]
    fn test_escaping_and_special_chars() {
        let content = "\u{FEFF}l_english:\n  KEY_ESC:0 \"Line 1\\nLine 2 with \\\"quotes\\\" and \\t tabs\"\n  KEY_COLOR: \"Some §Yyellow text§! here\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 2);
        assert_eq!(
            res.entries[0].value,
            "Line 1\nLine 2 with \"quotes\" and \t tabs"
        );
        assert_eq!(res.entries[1].value, "Some §Yyellow text§! here");
    }

    #[test]
    fn test_error_missing_language() {
        let content = "\u{FEFF}  KEY_1:0 \"Val 1\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.language, "");
        assert_eq!(res.errors.len(), 1);
        assert_eq!(res.errors[0].line_number, 1);
        assert!(res.errors[0].message.contains("заголовка языка"));
    }

    #[test]
    fn test_error_missing_colon() {
        let content = "\u{FEFF}l_english:\n  KEY_NO_COLON \"Val 1\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.errors.len(), 1);
        assert_eq!(res.errors[0].line_number, 2);
        assert!(res.errors[0].message.contains("двоеточие"));
    }

    #[test]
    fn test_error_invalid_key() {
        let content =
            "\u{FEFF}l_english:\n  KEY WITH SPACES:0 \"Val\"\n  KЁY_INVALID:0 \"Val\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.errors.len(), 2);
        assert!(res.errors[0].message.contains("Недопустимый ключ"));
        assert!(res.errors[1].message.contains("Недопустимый ключ"));
    }

    #[test]
    fn test_error_missing_quotes() {
        let content = "\u{FEFF}l_english:\n  KEY_1:0 Val\n  KEY_2:0 \"Missing closing\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.errors.len(), 2);
        assert_eq!(res.errors[0].line_number, 2);
        assert_eq!(res.errors[1].line_number, 3);
        assert!(res.errors[0].message.contains("Значение должно начинаться"));
        assert!(res.errors[1].message.contains("закрывающая кавычка"));
    }

    #[test]
    fn test_error_trailing_characters() {
        let content = "\u{FEFF}l_english:\n  KEY_1:0 \"Val 1\" trailing text here\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.errors.len(), 1);
        assert_eq!(res.errors[0].line_number, 2);
        assert!(res.errors[0]
            .message
            .contains("Лишние символы после закрывающей"));
    }

    #[test]
    fn test_error_missing_bom() {
        let content = "l_english:\n KEY_1:0 \"Val\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0]
            .message
            .contains("Отсутствует метка UTF-8 BOM"));
    }

    #[test]
    fn test_error_invalid_filename() {
        let content = "\u{FEFF}l_english:\n KEY_1:0 \"Val\"\n".as_bytes();
        let file = TempFile::new_with_filename(content, "invalid_name.yml");
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0]
            .message
            .contains("не соответствует правилам HOI4"));
    }

    #[test]
    fn test_error_language_mismatch() {
        let content = "\u{FEFF}l_russian:\n KEY_1:0 \"Val\"\n".as_bytes();
        let file = TempFile::new_with_filename(content, "temp_loc_l_english.yml");
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("Несоответствие языка"));
    }

    #[test]
    fn test_error_duplicate_keys() {
        let content = "\u{FEFF}l_english:\n KEY_1:0 \"Val 1\"\n KEY_1:0 \"Val 2\"\n".as_bytes();
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 2);
        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("Дубликат ключа"));
    }

    #[test]
    fn test_error_encoding_utf16() {
        // UTF-16 LE BOM: FF FE
        let mut content = vec![0xFF, 0xFE];
        for c in "l_english:\n KEY_1:0 \"Val\"\n".encode_utf16() {
            content.extend_from_slice(&c.to_le_bytes());
        }
        let file = make_temp_file(content.as_slice());
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("формате UTF-16"));
    }

    #[test]
    fn test_error_encoding_ansi() {
        // Запишем некорректные байты UTF-8 (Windows-1251)
        let content = b"l_english:\n KEY_1:0 \"\xCF\xF0\xE8\xE2\xE5\xF2\" # Windows-1251 Privet\n";
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        // 1 ошибка (некорректные символы UTF-8)
        assert_eq!(res.errors.len(), 1);
        assert!(res.errors[0].message.contains("некорректные символы UTF-8"));
    }

    #[test]
    fn test_field_localisation_files() {
        crate::parser::test_utils::run_field_test("yml", "ЛОКАЛИЗАЦИИ", |path| {
            parse_file(path).ok().map(|res| res.errors.len())
        });
    }
}
