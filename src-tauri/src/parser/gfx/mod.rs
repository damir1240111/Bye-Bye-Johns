use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GfxEntry {
    pub entry_type: String,
    pub name: String,
    pub texture_files: Vec<String>,
    pub line_number: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GfxError {
    pub line_number: usize,
    pub message: String,
    pub severity: String, // "Error" or "Warning"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GfxFile {
    pub entries: Vec<GfxEntry>,
    pub errors: Vec<GfxError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    OpenBrace,             // {
    CloseBrace,            // }
    Equals,                // =
    Identifier(String),    // word, number, path, bool value
    StringLiteral(String), // "value"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWithPos {
    pub token: Token,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Boolean(bool),
    Number(String),
    Block(Vec<KeyValuePair>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValuePair {
    pub key: String,
    pub value: Value,
    pub line_number: usize,
}

/// Лексер для токенизации Paradox Script
pub fn lex(input: &str) -> Vec<TokenWithPos> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line_number = 1;

    while let Some(&c) = chars.peek() {
        match c {
            '\n' => {
                line_number += 1;
                chars.next();
            }
            '\r' => {
                chars.next();
            }
            ' ' | '\t' => {
                chars.next();
            }
            '#' => {
                // Комментарий, пропускаем до конца строки
                chars.next();
                while let Some(&nc) = chars.peek() {
                    if nc == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '{' => {
                tokens.push(TokenWithPos { token: Token::OpenBrace, line_number });
                chars.next();
            }
            '}' => {
                tokens.push(TokenWithPos { token: Token::CloseBrace, line_number });
                chars.next();
            }
            '=' => {
                tokens.push(TokenWithPos { token: Token::Equals, line_number });
                chars.next();
            }
            '"' => {
                // Разбор строкового литерала в кавычках
                chars.next(); // Потребляем открывающую кавычку
                let mut s = String::new();
                let mut escaped = false;
                while let Some(nc) = chars.next() {
                    if escaped {
                        match nc {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            _ => {
                                s.push('\\');
                                s.push(nc);
                            }
                        }
                        escaped = false;
                    } else if nc == '\\' {
                        escaped = true;
                    } else if nc == '"' {
                        break; // Конец строки
                    } else {
                        s.push(nc);
                    }
                }
                tokens.push(TokenWithPos { token: Token::StringLiteral(s), line_number });
            }
            _ if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' || c == '+' || c == '/' || c == '\\' || c == ':' => {
                // Разбор идентификатора (включая числа, пути, булевы значения и ключи с двоеточием)
                let mut word = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' || nc == '.' || nc == '-' || nc == '+' || nc == '/' || nc == '\\' || nc == ':' {
                        word.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(TokenWithPos { token: Token::Identifier(word), line_number });
            }
            _ => {
                // Игнорируем или пропускаем неизвестные символы
                chars.next();
            }
        }
    }
    tokens
}

/// Парсер Paradox Script
pub fn parse_tokens(tokens: &[TokenWithPos]) -> Result<Vec<KeyValuePair>, String> {
    let mut index = 0;
    parse_block_contents(tokens, &mut index, false)
}

fn parse_block_contents(
    tokens: &[TokenWithPos],
    index: &mut usize,
    is_nested: bool,
) -> Result<Vec<KeyValuePair>, String> {
    let mut pairs = Vec::new();
    while *index < tokens.len() {
        let current = &tokens[*index];
        if is_nested && current.token == Token::CloseBrace {
            return Ok(pairs);
        }

        // Ожидаем ключ (Идентификатор)
        let key = match &current.token {
            Token::Identifier(k) => k.clone(),
            _ => return Err(format!("Строка {}: Ожидался ключ, найдено {:?}", current.line_number, current.token)),
        };
        let key_line = current.line_number;
        *index += 1;

        // Ожидаем символ '='
        if *index >= tokens.len() {
            return Err(format!("Строка {}: Неожиданный конец файла после ключа '{}'", key_line, key));
        }
        if tokens[*index].token != Token::Equals {
            return Err(format!("Строка {}: Ожидался символ '=', найдено {:?}", tokens[*index].line_number, tokens[*index].token));
        }
        *index += 1;

        // Ожидаем значение
        if *index >= tokens.len() {
            return Err(format!("Строка {}: Неожиданный конец файла после '='", key_line));
        }

        let value = parse_value(tokens, index)?;
        pairs.push(KeyValuePair {
            key,
            value,
            line_number: key_line,
        });
    }

    if is_nested {
        return Err("Неожиданный конец файла: отсутствует закрывающая фигурная скобка '}'".to_string());
    }

    Ok(pairs)
}

fn parse_value(tokens: &[TokenWithPos], index: &mut usize) -> Result<Value, String> {
    let current = &tokens[*index];
    match &current.token {
        Token::StringLiteral(s) => {
            *index += 1;
            Ok(Value::String(s.clone()))
        }
        Token::Identifier(s) => {
            *index += 1;
            if s == "yes" {
                Ok(Value::Boolean(true))
            } else if s == "no" {
                Ok(Value::Boolean(false))
            } else {
                Ok(Value::Number(s.clone()))
            }
        }
        Token::OpenBrace => {
            *index += 1; // Потребляем '{'
            let block = parse_block_contents(tokens, index, true)?;
            if *index >= tokens.len() || tokens[*index].token != Token::CloseBrace {
                return Err(format!("Строка {}: Ожидалась закрывающая скобка '}}'", current.line_number));
            }
            *index += 1; // Потребляем '}'
            Ok(Value::Block(block))
        }
        _ => Err(format!("Строка {}: Недопустимое значение {:?}", current.line_number, current.token)),
    }
}

fn extract_line_number_from_err(err: &str) -> Option<usize> {
    if err.starts_with("Строка ") {
        let after_prefix = &err["Строка ".len()..];
        if let Some(colon_pos) = after_prefix.find(':') {
            if let Ok(line) = after_prefix[..colon_pos].parse::<usize>() {
                return Some(line);
            }
        }
    }
    None
}

/// Парсит файл графических ресурсов (.gfx) по указанному пути
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<GfxFile, String> {
    let file_path = path.as_ref();
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mut errors = Vec::new();

    // 1. Читаем файл
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(format!("Не удалось прочитать файл: {}", e));
        }
    };

    // 2. Токенизация (лексический анализ)
    let tokens = lex(&content);

    // 3. Синтаксический разбор в AST
    let ast = match parse_tokens(&tokens) {
        Ok(a) => a,
        Err(e) => {
            let line_number = extract_line_number_from_err(&e).unwrap_or(1);
            errors.push(GfxError {
                line_number,
                message: format!("Ошибка синтаксиса: {}", e),
                severity: "Error".to_string(),
            });
            return Ok(GfxFile {
                entries: Vec::new(),
                errors,
            });
        }
    };

    // 4. Семантическая валидация правил и сбор ошибок
    let (entries, val_errors) = validate_ast(&ast, file_name);
    errors.extend(val_errors);

    Ok(GfxFile { entries, errors })
}

fn validate_ast(ast: &[KeyValuePair], file_name: &str) -> (Vec<GfxEntry>, Vec<GfxError>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut sprite_types_found = false;

    if !file_name.is_empty() && !file_name.ends_with(".gfx") {
        errors.push(GfxError {
            line_number: 1,
            message: format!("Предупреждение: Имя файла '{}' должно иметь расширение '.gfx'", file_name),
            severity: "Warning".to_string(),
        });
    }

    for pair in ast {
        if pair.key != "spriteTypes" {
            errors.push(GfxError {
                line_number: pair.line_number,
                message: format!("Недопустимый корневой элемент '{}'. Все элементы должны находиться внутри блока 'spriteTypes'", pair.key),
                severity: "Error".to_string(),
            });
            continue;
        }

        sprite_types_found = true;

        match &pair.value {
            Value::Block(sub_pairs) => {
                for sub_pair in sub_pairs {
                    let entry_type = &sub_pair.key;
                    match entry_type.as_str() {
                        "spriteType" | "corneredTileSpriteType" | "progressbartype" | 
                        "maskedShieldType" | "frameAnimatedSpriteType" | "textSpriteType" => {
                            match &sub_pair.value {
                                Value::Block(fields) => {
                                    validate_sprite_block(entry_type, fields, sub_pair.line_number, &mut entries, &mut errors);
                                }
                                _ => {
                                    errors.push(GfxError {
                                        line_number: sub_pair.line_number,
                                        message: format!("Элемент '{}' должен быть блоком в фигурных скобках", entry_type),
                                        severity: "Error".to_string(),
                                    });
                                }
                            }
                        }
                        _ => {
                            errors.push(GfxError {
                                line_number: sub_pair.line_number,
                                message: format!("Предупреждение: Неизвестный тип спрайта '{}' внутри spriteTypes", entry_type),
                                severity: "Warning".to_string(),
                            });
                        }
                    }
                }
            }
            _ => {
                errors.push(GfxError {
                    line_number: pair.line_number,
                    message: "Блок 'spriteTypes' должен содержать фигурные скобки с описанием спрайтов".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
    }

    if !sprite_types_found && errors.is_empty() {
        errors.push(GfxError {
            line_number: 1,
            message: "В файле отсутствует обязательный корневой блок 'spriteTypes'".to_string(),
            severity: "Error".to_string(),
        });
    }

    // Проверка дублирования имен спрайтов в файле
    let mut names_seen = HashSet::new();
    for entry in &entries {
        if !names_seen.insert(&entry.name) {
            errors.push(GfxError {
                line_number: entry.line_number,
                message: format!("Предупреждение: Дубликат имени спрайта: '{}' уже определен в этом файле", entry.name),
                severity: "Warning".to_string(),
            });
        }
    }

    (entries, errors)
}

fn validate_sprite_block(
    entry_type: &str,
    fields: &[KeyValuePair],
    line_number: usize,
    entries: &mut Vec<GfxEntry>,
    errors: &mut Vec<GfxError>,
) {
    let mut name = None;
    let mut texture_files = Vec::new();
    let mut border_size = None;
    let mut size = None;
    let mut effect_file = None;
    let mut no_of_frames = None;
    let mut looping = None;
    let mut play_on_show = None;
    let mut animation_rate_fps = None;

    for field in fields {
        let key = field.key.to_lowercase();
        match key.as_str() {
            "name" => {
                if let Value::String(s) = &field.value {
                    name = Some((s.clone(), field.line_number));
                } else if let Value::Number(n) = &field.value {
                    name = Some((n.clone(), field.line_number));
                } else {
                    errors.push(GfxError {
                        line_number: field.line_number,
                        message: "Поле 'name' должно быть строковым значением".to_string(),
                        severity: "Error".to_string(),
                    });
                }
            }
            "texturefile" | "texturefile1" | "texturefile2" => {
                if let Value::String(s) = &field.value {
                    texture_files.push((s.clone(), field.line_number));
                    validate_texture_path(s, field.line_number, errors);
                } else if let Value::Number(s) = &field.value {
                    texture_files.push((s.clone(), field.line_number));
                    validate_texture_path(s, field.line_number, errors);
                    errors.push(GfxError {
                        line_number: field.line_number,
                        message: format!("Предупреждение: Рекомендуется заключать путь к файлу '{}' в кавычки", s),
                        severity: "Warning".to_string(),
                    });
                } else {
                    errors.push(GfxError {
                        line_number: field.line_number,
                        message: format!("Поле '{}' должно быть строковым значением (путь к файлу)", field.key),
                        severity: "Error".to_string(),
                    });
                }
            }
            "bordersize" => {
                border_size = Some((&field.value, field.line_number));
                validate_xy_block(&field.value, "borderSize", field.line_number, errors);
            }
            "size" => {
                size = Some((&field.value, field.line_number));
                validate_xy_block(&field.value, "size", field.line_number, errors);
            }
            "effectfile" => {
                if let Value::String(s) = &field.value {
                    effect_file = Some((s.clone(), field.line_number));
                } else if let Value::Number(s) = &field.value {
                    effect_file = Some((s.clone(), field.line_number));
                } else {
                    errors.push(GfxError {
                        line_number: field.line_number,
                        message: "Поле 'effectFile' должно быть строковым значением".to_string(),
                        severity: "Error".to_string(),
                    });
                }
            }
            "noofframes" => {
                no_of_frames = Some((&field.value, field.line_number));
                validate_positive_int(&field.value, "noOfFrames", field.line_number, errors);
            }
            "looping" => {
                looping = Some((&field.value, field.line_number));
                validate_boolean(&field.value, "looping", field.line_number, errors);
            }
            "play_on_show" => {
                play_on_show = Some((&field.value, field.line_number));
                validate_boolean(&field.value, "play_on_show", field.line_number, errors);
            }
            "animation_rate_fps" => {
                animation_rate_fps = Some((&field.value, field.line_number));
                validate_positive_float(&field.value, "animation_rate_fps", field.line_number, errors);
            }
            _ => {
                // Игнорируем неопознанные свойства, чтобы избежать ложных предупреждений
            }
        }
    }

    let name_str = match name {
        Some((n, ln)) => {
            if !n.starts_with("GFX_") {
                errors.push(GfxError {
                    line_number: ln,
                    message: format!("Предупреждение: Имя спрайта '{}' нарушает конвенцию именования (должно начинаться с 'GFX_')", n),
                    severity: "Warning".to_string(),
                });
            }
            n
        }
        None => {
            errors.push(GfxError {
                line_number,
                message: format!("Элемент '{}' не содержит обязательное поле 'name'", entry_type),
                severity: "Error".to_string(),
            });
            "".to_string()
        }
    };

    let has_textures = !texture_files.is_empty();

    match entry_type {
        "spriteType" => {
            if !has_textures {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'spriteType' не содержит обязательное поле 'texturefile'".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
        "corneredTileSpriteType" => {
            if !has_textures {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'corneredTileSpriteType' не содержит обязательное поле 'texturefile'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if border_size.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'corneredTileSpriteType' не содержит обязательное поле 'borderSize'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if size.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'corneredTileSpriteType' не содержит обязательное поле 'size'".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
        "progressbartype" => {
            let mut has_t1 = false;
            let mut has_t2 = false;
            for f in fields {
                let k = f.key.to_lowercase();
                if k == "texturefile1" { has_t1 = true; }
                if k == "texturefile2" { has_t2 = true; }
            }
            if !has_t1 {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'progressbartype' не содержит обязательное поле 'textureFile1'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if !has_t2 {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'progressbartype' не содержит обязательное поле 'textureFile2'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if size.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'progressbartype' не содержит обязательное поле 'size'".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
        "maskedShieldType" => {
            let mut has_t1 = false;
            let mut has_t2 = false;
            for f in fields {
                let k = f.key.to_lowercase();
                if k == "texturefile1" { has_t1 = true; }
                if k == "texturefile2" { has_t2 = true; }
            }
            if !has_t1 {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'maskedShieldType' не содержит обязательное поле 'textureFile1'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if !has_t2 {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'maskedShieldType' не содержит обязательное поле 'textureFile2'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if effect_file.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'maskedShieldType' не содержит обязательное поле 'effectFile'".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
        "frameAnimatedSpriteType" => {
            if !has_textures {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'frameAnimatedSpriteType' не содержит обязательное поле 'texturefile'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if no_of_frames.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'frameAnimatedSpriteType' не содержит обязательное поле 'noOfFrames'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if animation_rate_fps.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'frameAnimatedSpriteType' не содержит обязательное поле 'animation_rate_fps'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if looping.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'frameAnimatedSpriteType' не содержит обязательное поле 'looping'".to_string(),
                    severity: "Error".to_string(),
                });
            }
            if play_on_show.is_none() {
                errors.push(GfxError {
                    line_number,
                    message: "Элемент 'frameAnimatedSpriteType' не содержит обязательное поле 'play_on_show'".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
        _ => {}
    }

    if !name_str.is_empty() {
        entries.push(GfxEntry {
            entry_type: entry_type.to_string(),
            name: name_str,
            texture_files: texture_files.into_iter().map(|(t, _)| t).collect(),
            line_number,
        });
    }
}

fn validate_texture_path(path: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    if path.contains('\\') {
        errors.push(GfxError {
            line_number,
            message: format!("Предупреждение: В пути к файлу '{}' используются обратные слэши '\\'. Рекомендуется использовать прямые слэши '/' для кроссплатформенности", path),
            severity: "Warning".to_string(),
        });
    }

    let lowercase_path = path.to_lowercase();
    let has_valid_ext = lowercase_path.ends_with(".dds") || lowercase_path.ends_with(".tga") || lowercase_path.ends_with(".png");
    if !has_valid_ext {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Файл '{}' имеет неподдерживаемое расширение. Разрешены только .dds, .tga и .png", path),
            severity: "Error".to_string(),
        });
    }

    if path.starts_with("c:") || path.starts_with("C:") || path.starts_with("d:") || path.starts_with("D:") || path.starts_with("/") {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Абсолютный путь '{}' недопустим. Путь должен быть относительным корня игры (например, 'gfx/interface/...')", path),
            severity: "Error".to_string(),
        });
    }
}

fn validate_xy_block(value: &Value, field_name: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    match value {
        Value::Block(pairs) => {
            let mut has_x = false;
            let mut has_y = false;
            for pair in pairs {
                let k = pair.key.to_lowercase();
                if k == "x" {
                    has_x = true;
                    validate_int(&pair.value, &format!("{}.x", field_name), pair.line_number, errors);
                } else if k == "y" {
                    has_y = true;
                    validate_int(&pair.value, &format!("{}.y", field_name), pair.line_number, errors);
                }
            }
            if !has_x {
                errors.push(GfxError {
                    line_number,
                    message: format!("Ошибка: В структуре '{}' отсутствует координата 'x'", field_name),
                    severity: "Error".to_string(),
                });
            }
            if !has_y {
                errors.push(GfxError {
                    line_number,
                    message: format!("Ошибка: В структуре '{}' отсутствует координата 'y'", field_name),
                    severity: "Error".to_string(),
                });
            }
        }
        _ => {
            errors.push(GfxError {
                line_number,
                message: format!("Ошибка: Структура '{}' должна быть блоком вида {{ x = ... y = ... }}", field_name),
                severity: "Error".to_string(),
            });
        }
    }
}

fn validate_int(val: &Value, name: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    if let Value::Number(s) = val {
        if s.parse::<i32>().is_err() {
            errors.push(GfxError {
                line_number,
                message: format!("Ошибка: Значение '{}' для '{}' должно быть целым числом", s, name),
                severity: "Error".to_string(),
            });
        }
    } else {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Значение для '{}' должно быть числом", name),
            severity: "Error".to_string(),
        });
    }
}

fn validate_positive_int(val: &Value, name: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    if let Value::Number(s) = val {
        match s.parse::<i32>() {
            Ok(v) => {
                if v <= 0 {
                    errors.push(GfxError {
                        line_number,
                        message: format!("Ошибка: Значение '{}' для '{}' должно быть строго больше 0", s, name),
                        severity: "Error".to_string(),
                    });
                }
            }
            Err(_) => {
                errors.push(GfxError {
                    line_number,
                    message: format!("Ошибка: Значение '{}' для '{}' должно быть целым числом", s, name),
                    severity: "Error".to_string(),
                });
            }
        }
    } else {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Значение для '{}' должно быть числом", name),
            severity: "Error".to_string(),
        });
    }
}

fn validate_positive_float(val: &Value, name: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    if let Value::Number(s) = val {
        match s.parse::<f32>() {
            Ok(v) => {
                if v <= 0.0 {
                    errors.push(GfxError {
                        line_number,
                        message: format!("Ошибка: Значение '{}' для '{}' должно быть больше 0.0", s, name),
                        severity: "Error".to_string(),
                    });
                }
            }
            Err(_) => {
                errors.push(GfxError {
                    line_number,
                    message: format!("Ошибка: Значение '{}' для '{}' должно быть числом", s, name),
                    severity: "Error".to_string(),
                });
            }
        }
    } else {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Значение для '{}' должно быть числом", name),
            severity: "Error".to_string(),
        });
    }
}

fn validate_boolean(val: &Value, name: &str, line_number: usize, errors: &mut Vec<GfxError>) {
    if !matches!(val, Value::Boolean(_)) {
        errors.push(GfxError {
            line_number,
            message: format!("Ошибка: Значение для '{}' должно быть 'yes' или 'no'", name),
            severity: "Error".to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempFile {
        path: std::path::PathBuf,
    }

    impl TempFile {
        fn new(content: &str) -> Self {
            Self::new_with_filename(content, "temp_ui.gfx")
        }

        fn new_with_filename(content: &str, filename_pattern: &str) -> Self {
            let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut path = std::env::temp_dir();
            let filename = format!("{}_{}", count, filename_pattern);
            path.push(filename);
            std::fs::write(&path, content).unwrap();
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_lexer_and_parser_happy_path() {
        let content = r#"
            # Комментарий
            spriteTypes = {
                spriteType = {
                    name = "GFX_ger_democracy"
                    texturefile = "gfx/interface/goals/focus_ger_democracy.dds"
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].entry_type, "spriteType");
        assert_eq!(res.entries[0].name, "GFX_ger_democracy");
        assert_eq!(res.entries[0].texture_files[0], "gfx/interface/goals/focus_ger_democracy.dds");
    }

    #[test]
    fn test_lexer_unquoted_path() {
        let content = r#"
            spriteTypes = {
                spriteType = {
                    name = "GFX_ger_democracy"
                    texturefile = gfx/interface/goals/focus_ger_democracy.dds
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        // Должно успешно распарситься, но выдать Warning о нецитируемом пути
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].name, "GFX_ger_democracy");
        assert_eq!(res.entries[0].texture_files[0], "gfx/interface/goals/focus_ger_democracy.dds");
        assert!(res.errors.iter().any(|e| e.severity == "Warning" && e.message.contains("кавычки")));
    }

    #[test]
    fn test_cornered_tile_sprite() {
        let content = r#"
            spriteTypes = {
                corneredTileSpriteType = {
                    name = "GFX_tiled_window"
                    texturefile = "gfx/interface/tiles/tiled_window.dds"
                    size = { x = 192 y = 192 }
                    borderSize = { x = 64 y = 64 }
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].entry_type, "corneredTileSpriteType");
        assert_eq!(res.entries[0].name, "GFX_tiled_window");
    }

    #[test]
    fn test_progressbar_and_masked_shield() {
        let content = r#"
            spriteTypes = {
                progressbartype = {
                    name = "GFX_bar"
                    textureFile1 = "gfx/interface/bar_full.dds"
                    textureFile2 = "gfx/interface/bar_empty.dds"
                    size = { x = 100 y = 10 }
                }
                maskedShieldType = {
                    name = "GFX_shield"
                    textureFile1 = "gfx/interface/flag.dds"
                    textureFile2 = "gfx/interface/mask.dds"
                    effectFile = "gfx/FX/shield.lua"
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 2);
    }

    #[test]
    fn test_animated_sprite() {
        let content = r#"
            spriteTypes = {
                frameAnimatedSpriteType = {
                    name = "GFX_anim"
                    texturefile = "gfx/interface/animated.dds"
                    noOfFrames = 10
                    animation_rate_fps = 15.5
                    looping = yes
                    play_on_show = no
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.entries.len(), 1);
    }

    #[test]
    fn test_errors_missing_braces() {
        let content = r#"
            spriteTypes = {
                spriteType = {
                    name = "GFX_foo"
                    texturefile = "gfx/interface/foo.dds"
                # Забыли скобку }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 0);
        assert!(res.errors.len() > 0);
        assert!(res.errors[0].message.contains("отсутствует закрывающая фигурная скобка"));
    }

    #[test]
    fn test_errors_naming_convention() {
        let content = r#"
            spriteTypes = {
                spriteType = {
                    name = "foo_without_prefix"
                    texturefile = "gfx/interface/foo.dds"
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 1);
        assert!(res.errors.iter().any(|e| e.severity == "Warning" && e.message.contains("конвенцию именования")));
    }

    #[test]
    fn test_errors_wrong_slashes_and_missing_mandatory_fields() {
        let content = r#"
            spriteTypes = {
                spriteType = {
                    name = "GFX_wrong_path"
                    texturefile = "gfx\interface\foo.dds" # Неверные слэши
                }
                corneredTileSpriteType = {
                    name = "GFX_missing_size"
                    texturefile = "gfx/interface/tile.dds"
                    # size и borderSize пропущены
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        // Должны быть ошибки валидации
        assert!(res.errors.iter().any(|e| e.severity == "Warning" && e.message.contains("обратные слэши")));
        assert!(res.errors.iter().any(|e| e.severity == "Error" && e.message.contains("не содержит обязательное поле 'borderSize'")));
        assert!(res.errors.iter().any(|e| e.severity == "Error" && e.message.contains("не содержит обязательное поле 'size'")));
    }

    #[test]
    fn test_duplicate_sprite_names() {
        let content = r#"
            spriteTypes = {
                spriteType = {
                    name = "GFX_duplicate"
                    texturefile = "gfx/interface/foo.dds"
                }
                spriteType = {
                    name = "GFX_duplicate"
                    texturefile = "gfx/interface/bar.dds"
                }
            }
        "#;
        let file = TempFile::new(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.entries.len(), 2);
        assert!(res.errors.iter().any(|e| e.severity == "Warning" && e.message.contains("Дубликат имени спрайта")));
    }

    #[test]
    fn test_field_gfx_files() {
        let mod_dir = match dirs::document_dir() {
            Some(mut p) => {
                p.push("Paradox Interactive");
                p.push("Hearts of Iron IV");
                p.push("mod");
                p
            }
            None => {
                println!("Пропущено: не удалось определить папку документов.");
                return;
            }
        };

        if !mod_dir.exists() {
            println!("Пропущено: папка модов HOI4 не найдена по пути {:?}", mod_dir);
            return;
        }

        println!("Сканирование папки модов на .gfx файлы: {:?}", mod_dir);
        let mut files_scanned = 0;
        let mut total_errors = 0;
        let mut failed_files = Vec::new();

        fn scan_dir(
            dir: &std::path::Path,
            files_scanned: &mut usize,
            total_errors: &mut usize,
            failed_files: &mut Vec<(std::path::PathBuf, usize)>,
        ) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        scan_dir(&path, files_scanned, total_errors, failed_files);
                    } else if path.extension().map_or(false, |ext| ext == "gfx") {
                        *files_scanned += 1;
                        match parse_file(&path) {
                            Ok(res) => {
                                if !res.errors.is_empty() {
                                    *total_errors += res.errors.len();
                                    failed_files.push((path.clone(), res.errors.len()));
                                }
                            }
                            Err(e) => {
                                println!("Сбой при чтении файла {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        scan_dir(
            &mod_dir,
            &mut files_scanned,
            &mut total_errors,
            &mut failed_files,
        );

        println!("\n=== ОТЧЕТ ПО ПОЛЕВОМУ ТЕСТИРОВАНИЮ GFX ===");
        println!("Просканировано .gfx файлов: {}", files_scanned);
        println!("Файлов с предупреждениями/ошибками: {}", failed_files.len());
        println!("Всего обнаружено проблем/ошибок: {}", total_errors);

        if !failed_files.is_empty() {
            println!("\nТоп 10 файлов с наибольшим количеством проблем:");
            failed_files.sort_by(|a, b| b.1.cmp(&a.1));
            for (path, err_count) in failed_files.iter().take(10) {
                println!(
                    "- {:?} (проблем: {})",
                    path.file_name().unwrap_or_default(),
                    err_count
                );
            }
        }
        println!("==========================================\n");

        assert!(true);
    }
}
