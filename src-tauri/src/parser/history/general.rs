use std::collections::HashMap;
use std::fs;
use crate::parser::pdx_script::{self, Value, KeyValuePair};
use crate::parser::history::HistoryError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct Character {
    pub token: String,
    pub name: String,
    pub portraits: Vec<(String, String)>, // (type/slot, sprite_name)
    pub advisor_slots: Vec<String>,
    pub skills: HashMap<String, u32>,
}

fn extract_portraits(portraits_block: &[KeyValuePair], prefix: &str, portraits: &mut Vec<(String, String)>) {
    for pair in portraits_block {
        let current_prefix = if prefix.is_empty() {
            pair.key.clone()
        } else {
            format!("{}_{}", prefix, pair.key)
        };
        match &pair.value {
            Value::Block(b) => {
                extract_portraits(b, &current_prefix, portraits);
            }
            Value::String(s) | Value::Number(s) => {
                portraits.push((current_prefix, s.clone()));
            }
            _ => {}
        }
    }
}

fn parse_character_block(
    block: &[KeyValuePair],
    path: &str,
    start_line: usize,
    errors: &mut Vec<HistoryError>,
) -> Option<Character> {
    let mut token = None;
    let mut name = None;
    let mut portraits = Vec::new();
    let mut advisor_slots = Vec::new();
    let mut skills = HashMap::new();

    for field in block {
        match field.key.as_str() {
            "token" | "token_base" => {
                match &field.value {
                    Value::String(s) | Value::Number(s) => token = Some(s.clone()),
                    _ => {}
                }
            }
            "name" => {
                match &field.value {
                    Value::String(s) | Value::Number(s) => name = Some(s.clone()),
                    _ => {}
                }
            }
            "portraits" => {
                if let Value::Block(pb) = &field.value {
                    extract_portraits(pb, "", &mut portraits);
                }
            }
            "advisor" => {
                if let Value::Block(ab) = &field.value {
                    for af in ab {
                        if af.key == "slot" {
                            match &af.value {
                                Value::String(s) | Value::Number(s) => advisor_slots.push(s.clone()),
                                _ => {}
                            }
                        }
                    }
                }
            }
            "corps_commander" | "field_marshal" | "navy_leader" => {
                if let Value::Block(cb) = &field.value {
                    for cf in cb {
                        if cf.key.ends_with("skill") || cf.key == "skill" {
                            if let Value::Number(n) = &cf.value {
                                if let Ok(val) = n.parse::<u32>() {
                                    skills.insert(cf.key.clone(), val);
                                    if val < 1 || val > 10 {
                                        errors.push(HistoryError {
                                            file: path.to_string(),
                                            line_number: cf.line_number,
                                            message: format!(
                                                "Характеристика '{}' персонажа имеет значение {}, выходящее за рамки [1-10]",
                                                cf.key, val
                                            ),
                                            severity: "Warning".to_string(),
                                        });
                                    }
                                } else {
                                    errors.push(HistoryError {
                                        file: path.to_string(),
                                        line_number: cf.line_number,
                                        message: format!("Неверный формат характеристики '{}': '{}'", cf.key, n),
                                        severity: "Error".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if token.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: start_line,
            message: "Отсутствует обязательное поле 'token' или 'token_base' у персонажа".to_string(),
            severity: "Error".to_string(),
        });
        return None;
    }

    for (port_type, sprite) in &portraits {
        if !sprite.starts_with("GFX_") {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: start_line,
                message: format!(
                    "Нарушение конвенции именования портрета '{}': спрайт '{}' должен начинаться с 'GFX_'",
                    port_type, sprite
                ),
                severity: "Warning".to_string(),
            });
        }
    }

    Some(Character {
        token: token.unwrap(),
        name: name.unwrap_or_default(),
        portraits,
        advisor_slots,
        skills,
    })
}

fn find_characters(
    pairs: &[KeyValuePair],
    path: &str,
    characters: &mut Vec<Character>,
    errors: &mut Vec<HistoryError>,
) {
    for pair in pairs {
        if pair.key == "generate_character" {
            if let Value::Block(b) = &pair.value {
                if let Some(char) = parse_character_block(b, path, pair.line_number, errors) {
                    characters.push(char);
                }
            }
        } else if let Value::Block(b) = &pair.value {
            find_characters(b, path, characters, errors);
        }
    }
}

pub fn parse_file(path: &str) -> Result<(Vec<Character>, Vec<HistoryError>), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    let tokens = pdx_script::lex(&content);
    let parsed = match pdx_script::parse_tokens(&tokens) {
        Ok(p) => p,
        Err(e) => {
            let line_number = pdx_script::extract_line_number_from_err(&e).unwrap_or(1);
            errors.push(HistoryError {
                file: path.to_string(),
                line_number,
                message: format!("Ошибка синтаксиса: {}", e),
                severity: "Error".to_string(),
            });
            return Ok((Vec::new(), errors));
        }
    };

    let mut characters = Vec::new();
    find_characters(&parsed, path, &mut characters, &mut errors);

    Ok((characters, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_characters() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("china_shared_advisors.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            every_possible_country = {{
                limit = {{}}
                generate_character = {{
                    token_base = CHI_advisor_alexander_von_falkenhausen
                    name = CHI_advisor_alexander_von_falkenhausen
                    portraits = {{
                        civil = {{
                            large = GFX_portrait_CHI_alexander_von_falkenhausen
                        }}
                    }}
                    advisor = {{
                        slot = political_advisor
                    }}
                }}
            }}
        "#).unwrap();

        let (chars, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0].token, "CHI_advisor_alexander_von_falkenhausen");
        assert_eq!(chars[0].name, "CHI_advisor_alexander_von_falkenhausen");
        assert_eq!(chars[0].portraits, vec![("civil_large".to_string(), "GFX_portrait_CHI_alexander_von_falkenhausen".to_string())]);
        assert_eq!(chars[0].advisor_slots, vec!["political_advisor".to_string()]);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_character_skill_warning() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("invalid_skills.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            generate_character = {{
                token = test_char
                corps_commander = {{
                    attack_skill = 15
                }}
            }}
        "#).unwrap();

        let (chars, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert_eq!(chars.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, "Warning");
        assert!(errors[0].message.contains("выходящее за рамки"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_field_general_files() {
        crate::parser::test_utils::run_history_field_test("general", "General", |path| {
            parse_file(path.to_str().unwrap()).ok().map(|(_, errs)| errs.len())
        });
    }
}
