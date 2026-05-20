use std::fs;
use crate::parser::pdx_script::{self, Value};
use crate::parser::history::HistoryError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct CountryHistory {
    pub tag: String,
    pub capital: Option<u32>,
    pub oob: Option<String>,
    pub characters: Vec<String>,
    pub ruling_party: Option<String>,
    pub popularities: Vec<(String, f64)>,
    pub technologies: Vec<String>,
}

fn extract_tag_from_path(path: &str) -> Option<String> {
    let file_name = std::path::Path::new(path).file_name()?.to_str()?;
    if file_name.len() >= 3 {
        let tag = &file_name[0..3];
        if tag.chars().all(|c| c.is_ascii_alphanumeric()) {
            if file_name.len() == 3 || (file_name.len() > 3 && !file_name.chars().nth(3)?.is_ascii_alphabetic()) {
                return Some(tag.to_ascii_uppercase());
            }
        }
    }
    None
}


pub fn parse_file(path: &str) -> Result<(CountryHistory, Vec<HistoryError>), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    let tag = match extract_tag_from_path(path) {
        Some(t) => t,
        None => {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: 1,
                message: format!("Имя файла не начинается с 3-буквенного TAG страны: '{}'", path),
                severity: "Error".to_string(),
            });
            "".to_string()
        }
    };

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
            return Ok((CountryHistory::default(), errors));
        }
    };

    let mut capital = None;
    let mut oob = None;
    let mut characters = Vec::new();
    let mut ruling_party = None;
    let mut popularities = Vec::new();
    let mut technologies = Vec::new();

    for field in &parsed {
        match field.key.as_str() {
            "capital" => {
                if let Value::Number(n) = &field.value {
                    if let Ok(val) = n.parse::<u32>() {
                        capital = Some(val);
                    } else {
                        errors.push(HistoryError {
                            file: path.to_string(),
                            line_number: field.line_number,
                            message: format!("Неверный формат столичной области: '{}'", n),
                            severity: "Error".to_string(),
                        });
                    }
                }
            }
            "oob" => {
                match &field.value {
                    Value::String(s) | Value::Number(s) => oob = Some(s.clone()),
                    _ => {}
                }
            }
            "recruit_character" => {
                match &field.value {
                    Value::String(s) | Value::Number(s) => characters.push(s.clone()),
                    _ => {}
                }
            }
            "set_politics" => {
                if let Value::Block(block) = &field.value {
                    for f in block {
                        if f.key == "ruling_party" {
                            match &f.value {
                                Value::String(s) | Value::Number(s) => ruling_party = Some(s.clone()),
                                _ => {}
                            }
                        }
                    }
                }
            }
            "set_popularities" => {
                if let Value::Block(block) = &field.value {
                    let mut sum = 0.0;
                    for f in block {
                        if let Value::Number(num_str) = &f.value {
                            if let Ok(val) = num_str.parse::<f64>() {
                                popularities.push((f.key.clone(), val));
                                sum += val;
                            } else {
                                errors.push(HistoryError {
                                    file: path.to_string(),
                                    line_number: f.line_number,
                                    message: format!("Неверный процент популярности для идеологии '{}': '{}'", f.key, num_str),
                                    severity: "Error".to_string(),
                                });
                            }
                        }
                    }
                    if !popularities.is_empty() && (sum < 99.0 || sum > 101.0) {
                        errors.push(HistoryError {
                            file: path.to_string(),
                            line_number: field.line_number,
                            message: format!("Сумма популярности идеологий не равна 100% (текущая сумма: {:.1}%)", sum),
                            severity: "Warning".to_string(),
                        });
                    }
                }
            }
            "set_technology" => {
                if let Value::Block(block) = &field.value {
                    for f in block {
                        if !f.key.is_empty() {
                            technologies.push(f.key.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok((
        CountryHistory {
            tag,
            capital,
            oob,
            characters,
            ruling_party,
            popularities,
            technologies,
        },
        errors,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_country_happy_path() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("ALB - Albania.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            capital = 43
            oob = "ALB_1936"
            set_politics = {{
                ruling_party = neutrality
            }}
            set_popularities = {{
                democratic = 10
                fascism = 20
                neutrality = 70
            }}
            recruit_character = ALB_ahmed_zogu
            set_technology = {{
                infantry_weapons = 1
            }}
        "#).unwrap();

        let (country, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(country.tag, "ALB");
        assert_eq!(country.capital, Some(43));
        assert_eq!(country.oob, Some("ALB_1936".to_string()));
        assert_eq!(country.ruling_party, Some("neutrality".to_string()));
        assert_eq!(country.characters, vec!["ALB_ahmed_zogu".to_string()]);
        assert_eq!(country.technologies, vec!["infantry_weapons".to_string()]);
        assert_eq!(country.popularities.len(), 3);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_country_popularity_warning() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("GER - Germany.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            capital = 1
            set_popularities = {{
                democratic = 10
                fascism = 50
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, "Warning");
        assert!(errors[0].message.contains("не равна 100%"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_parse_country_invalid_filename() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("Germany.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            capital = 1
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, "Error");
        assert!(errors[0].message.contains("TAG"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_field_country_files() {
        crate::parser::test_utils::run_history_field_test("countries", "Countries", |path| {
            parse_file(path.to_str().unwrap()).ok().map(|(_, errs)| errs.len())
        });
    }
}
