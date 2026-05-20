#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Token {
    OpenBrace,             // {
    CloseBrace,            // }
    Equals,                // =
    Identifier(String),    // word, number, path, bool value
    StringLiteral(String), // "value"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TokenWithPos {
    pub token: Token,
    pub line_number: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Value {
    String(String),
    Boolean(bool),
    Number(String),
    Block(Vec<KeyValuePair>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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
            _ if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' || c == '+' || c == '/' || c == '\\' || c == ':' || c == '@' => {
                // Разбор идентификатора (включая числа, пути, булевы значения, ключи с двоеточием и константы с @)
                let mut word = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' || nc == '.' || nc == '-' || nc == '+' || nc == '/' || nc == '\\' || nc == ':' || nc == '@' {
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

        // Check if this is a key-value pair
        let is_kv = match &current.token {
            Token::Identifier(_) | Token::StringLiteral(_) => {
                *index + 1 < tokens.len() && tokens[*index + 1].token == Token::Equals
            }
            _ => false,
        };

        if is_kv {
            let key = match &current.token {
                Token::Identifier(k) => k.clone(),
                Token::StringLiteral(k) => k.clone(),
                _ => unreachable!(),
            };
            let key_line = current.line_number;
            *index += 2; // skip key and '='

            if *index >= tokens.len() {
                return Err(format!("Строка {}: Неожиданный конец файла после '='", key_line));
            }

            let value = parse_value(tokens, index)?;
            pairs.push(KeyValuePair {
                key,
                value,
                line_number: key_line,
            });
        } else {
            // Treat as a list item (empty key, value only)
            let line_num = current.line_number;
            let value = parse_value(tokens, index)?;
            pairs.push(KeyValuePair {
                key: "".to_string(),
                value,
                line_number: line_num,
            });
        }
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

pub fn extract_line_number_from_err(err: &str) -> Option<usize> {
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
