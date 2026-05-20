use std::collections::HashMap;
use std::fs;
use crate::parser::pdx_script::{self, Value};
use crate::parser::history::HistoryError;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct State {
    pub id: u32,
    pub name: String,
    pub manpower: u32,
    pub state_category: String,
    pub provinces: Vec<u32>,
    pub owner: Option<String>,
    pub cores: Vec<String>,
    pub claims: Vec<String>,
    pub victory_points: Vec<(u32, u32)>,
    pub buildings: HashMap<String, u32>,
}

fn is_valid_tag(tag: &str) -> bool {
    tag.len() == 3 && tag.chars().all(|c| c.is_ascii_alphanumeric() && c.is_ascii_uppercase())
}

pub fn parse_file(path: &str) -> Result<(State, Vec<HistoryError>), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    // Check for commas in provinces or victory_points lines
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let clean_line = line.trim();
        if (clean_line.contains("provinces") || clean_line.contains("victory_points")) && clean_line.contains(',') {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: line_num,
                message: format!("Использование запятой ',' в списке: '{}'", clean_line),
                severity: "Error".to_string(),
            });
        }
    }

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
            return Ok((State::default(), errors));
        }
    };

    let mut state_block = None;
    for pair in &parsed {
        if pair.key == "state" {
            if let Value::Block(b) = &pair.value {
                state_block = Some(b);
                break;
            }
        }
    }

    let state_fields = match state_block {
        Some(b) => b,
        None => {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: 1,
                message: "Отсутствует корневой блок 'state = { ... }'".to_string(),
                severity: "Error".to_string(),
            });
            return Ok((State::default(), errors));
        }
    };

    let mut id = None;
    let mut name = None;
    let mut manpower = None;
    let mut state_category = None;
    let mut provinces = Vec::new();
    let mut owner = None;
    let mut cores = Vec::new();
    let mut claims = Vec::new();
    let mut victory_points = Vec::new();
    let mut buildings = HashMap::new();

    for field in state_fields {
        match field.key.as_str() {
            "id" => {
                if let Value::Number(n) = &field.value {
                    if let Ok(val) = n.parse::<u32>() {
                        id = Some(val);
                    } else {
                        errors.push(HistoryError {
                            file: path.to_string(),
                            line_number: field.line_number,
                            message: format!("Неверный формат id: '{}'", n),
                            severity: "Error".to_string(),
                        });
                    }
                }
            }
            "name" => {
                match &field.value {
                    Value::String(s) => name = Some(s.clone()),
                    Value::Number(n) => name = Some(n.clone()),
                    _ => {}
                }
            }
            "manpower" => {
                if let Value::Number(n) = &field.value {
                    if let Ok(val) = n.parse::<u32>() {
                        manpower = Some(val);
                    } else {
                        errors.push(HistoryError {
                            file: path.to_string(),
                            line_number: field.line_number,
                            message: format!("Неверный формат manpower: '{}'", n),
                            severity: "Error".to_string(),
                        });
                    }
                }
            }
            "state_category" => {
                match &field.value {
                    Value::String(s) => state_category = Some(s.clone()),
                    Value::Number(n) => state_category = Some(n.clone()),
                    _ => {}
                }
            }
            "provinces" => {
                if let Value::Block(b) = &field.value {
                    for prov_pair in b {
                        let prov_str = match &prov_pair.value {
                            Value::Number(n) => n.clone(),
                            Value::String(s) => s.clone(),
                            _ => "".to_string(),
                        };
                        if let Ok(prov_id) = prov_str.parse::<u32>() {
                            provinces.push(prov_id);
                        } else if !prov_str.is_empty() {
                            errors.push(HistoryError {
                                file: path.to_string(),
                                line_number: prov_pair.line_number,
                                message: format!("Неверный ID провинции в списке provinces: '{}'", prov_str),
                                severity: "Error".to_string(),
                            });
                        }
                    }
                }
            }
            "history" => {
                if let Value::Block(hist_block) = &field.value {
                    for hist_field in hist_block {
                        match hist_field.key.as_str() {
                            "owner" => {
                                match &hist_field.value {
                                    Value::String(s) | Value::Number(s) => {
                                        if is_valid_tag(s) {
                                            owner = Some(s.clone());
                                        } else {
                                            errors.push(HistoryError {
                                                file: path.to_string(),
                                                line_number: hist_field.line_number,
                                                message: format!("Некорректный формат TAG владельца: '{}'. Ожидалось 3 заглавных буквы/цифры", s),
                                                severity: "Error".to_string(),
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "add_core_of" => {
                                match &hist_field.value {
                                    Value::String(s) | Value::Number(s) => {
                                        if is_valid_tag(s) {
                                            cores.push(s.clone());
                                        } else {
                                            errors.push(HistoryError {
                                                file: path.to_string(),
                                                line_number: hist_field.line_number,
                                                message: format!("Некорректный формат TAG национальной корки: '{}'. Ожидалось 3 заглавных буквы/цифры", s),
                                                severity: "Error".to_string(),
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "add_claim_by" => {
                                match &hist_field.value {
                                    Value::String(s) | Value::Number(s) => {
                                        if is_valid_tag(s) {
                                            claims.push(s.clone());
                                        } else {
                                            errors.push(HistoryError {
                                                file: path.to_string(),
                                                line_number: hist_field.line_number,
                                                message: format!("Некорректный формат TAG претензии: '{}'. Ожидалось 3 заглавных буквы/цифры", s),
                                                severity: "Error".to_string(),
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "victory_points" => {
                                if let Value::Block(vp_block) = &hist_field.value {
                                    let mut flat = Vec::new();
                                    for vp_pair in vp_block {
                                        let val_str = match &vp_pair.value {
                                            Value::Number(n) => n.clone(),
                                            Value::String(s) => s.clone(),
                                            _ => "".to_string(),
                                        };
                                        if let Ok(v) = val_str.parse::<u32>() {
                                            flat.push((v, vp_pair.line_number));
                                        } else if !val_str.is_empty() {
                                            errors.push(HistoryError {
                                                file: path.to_string(),
                                                line_number: vp_pair.line_number,
                                                message: format!("Неверное значение в victory_points: '{}'", val_str),
                                                severity: "Error".to_string(),
                                            });
                                        }
                                    }
                                    if flat.len() % 2 != 0 {
                                        errors.push(HistoryError {
                                            file: path.to_string(),
                                            line_number: hist_field.line_number,
                                            message: "Нечетное количество аргументов в victory_points. Ожидаются пары (провинция, очки)".to_string(),
                                            severity: "Error".to_string(),
                                        });
                                    }
                                    for chunk in flat.chunks(2) {
                                        if chunk.len() == 2 {
                                            victory_points.push((chunk[0].0, chunk[1].0));
                                        }
                                    }
                                }
                            }
                            "buildings" => {
                                if let Value::Block(build_block) = &hist_field.value {
                                    for build_pair in build_block {
                                        if let Value::Block(prov_buildings) = &build_pair.value {
                                            if build_pair.key.parse::<u32>().is_err() {
                                                errors.push(HistoryError {
                                                    file: path.to_string(),
                                                    line_number: build_pair.line_number,
                                                    message: format!("Неверный ключ провинции в buildings: '{}'", build_pair.key),
                                                    severity: "Error".to_string(),
                                                });
                                            }
                                            for pb in prov_buildings {
                                                if let Value::Number(num_str) = &pb.value {
                                                    if let Ok(count) = num_str.parse::<u32>() {
                                                        buildings.insert(format!("{}_{}", build_pair.key, pb.key), count);
                                                    } else {
                                                        errors.push(HistoryError {
                                                            file: path.to_string(),
                                                            line_number: pb.line_number,
                                                            message: format!("Неверное количество построек в провинции: '{}'", num_str),
                                                            severity: "Error".to_string(),
                                                        });
                                                    }
                                                }
                                            }
                                        } else if let Value::Number(num_str) = &build_pair.value {
                                            if let Ok(count) = num_str.parse::<u32>() {
                                                buildings.insert(build_pair.key.clone(), count);
                                            } else {
                                                errors.push(HistoryError {
                                                    file: path.to_string(),
                                                    line_number: build_pair.line_number,
                                                    message: format!("Неверное количество построек: '{}'", num_str),
                                                    severity: "Error".to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Validate mandatory fields
    if id.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: 1,
            message: "Отсутствует обязательное поле 'id'".to_string(),
            severity: "Error".to_string(),
        });
    }
    if name.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: 1,
            message: "Отсутствует обязательное поле 'name'".to_string(),
            severity: "Error".to_string(),
        });
    }
    if manpower.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: 1,
            message: "Отсутствует обязательное поле 'manpower'".to_string(),
            severity: "Error".to_string(),
        });
    }
    if state_category.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: 1,
            message: "Отсутствует обязательное поле 'state_category'".to_string(),
            severity: "Error".to_string(),
        });
    }
    if provinces.is_empty() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number: 1,
            message: "Отсутствует обязательное поле 'provinces' или список пуст".to_string(),
            severity: "Error".to_string(),
        });
    }

    Ok((
        State {
            id: id.unwrap_or(0),
            name: name.unwrap_or_default(),
            manpower: manpower.unwrap_or(0),
            state_category: state_category.unwrap_or_default(),
            provinces,
            owner,
            cores,
            claims,
            victory_points,
            buildings,
        },
        errors,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_state_happy_path() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_state_1.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            state = {{
                id = 1
                name = "STATE_1"
                manpower = 500000
                state_category = town
                history = {{
                    owner = FRA
                    victory_points = {{ 3838 5 }}
                    buildings = {{
                        infrastructure = 3
                        3838 = {{
                            naval_base = 2
                        }}
                    }}
                    add_core_of = FRA
                    add_core_of = COR
                }}
                provinces = {{ 3838 3839 }}
            }}
        "#).unwrap();

        let (state, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(state.id, 1);
        assert_eq!(state.name, "STATE_1");
        assert_eq!(state.manpower, 500000);
        assert_eq!(state.state_category, "town");
        assert_eq!(state.owner, Some("FRA".to_string()));
        assert_eq!(state.provinces, vec![3838, 3839]);
        assert_eq!(state.cores, vec!["FRA".to_string(), "COR".to_string()]);
        assert_eq!(state.victory_points, vec![(3838, 5)]);
        assert_eq!(state.buildings.get("infrastructure").cloned(), Some(3));
        assert_eq!(state.buildings.get("3838_naval_base").cloned(), Some(2));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_state_missing_fields() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_state_missing.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            state = {{
                id = 2
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        let msg: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        assert!(msg.iter().any(|m| m.contains("name")));
        assert!(msg.iter().any(|m| m.contains("manpower")));
        assert!(msg.iter().any(|m| m.contains("state_category")));
        assert!(msg.iter().any(|m| m.contains("provinces")));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_state_invalid_tag() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_state_invalid_tag.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            state = {{
                id = 1
                name = "STATE_1"
                manpower = 500
                state_category = rural
                history = {{
                    owner = france
                }}
                provinces = {{ 1 }}
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("france"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_state_invalid_commas() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_state_commas.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            state = {{
                id = 1
                name = "STATE_1"
                manpower = 500
                state_category = rural
                history = {{
                    owner = FRA
                }}
                provinces = {{ 1, 2, 3 }}
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("запятой"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_field_state_files() {
        crate::parser::test_utils::run_history_field_test("states", "States", |path| {
            parse_file(path.to_str().unwrap()).ok().map(|(_, errs)| errs.len())
        });
    }
}
