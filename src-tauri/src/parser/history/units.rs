use std::fs;
use crate::parser::pdx_script::{self, Value, KeyValuePair};
use crate::parser::history::HistoryError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Regiment {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DivisionTemplate {
    pub name: String,
    pub regiments: Vec<Regiment>,
    pub support: Vec<Regiment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Division {
    pub name: String,
    pub location: u32,
    pub template_name: String,
    pub experience: Option<f64>,
    pub equipment: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct UnitOob {
    pub division_templates: Vec<DivisionTemplate>,
    pub divisions: Vec<Division>,
}

fn parse_regiment(
    name: &str,
    block: &[KeyValuePair],
    path: &str,
    line_number: usize,
    errors: &mut Vec<HistoryError>,
) -> Option<Regiment> {
    let mut x = None;
    let mut y = None;

    for f in block {
        if f.key == "x" {
            if let Value::Number(num_str) = &f.value {
                x = num_str.parse::<i32>().ok();
            }
        } else if f.key == "y" {
            if let Value::Number(num_str) = &f.value {
                y = num_str.parse::<i32>().ok();
            }
        }
    }

    if x.is_none() || y.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number,
            message: format!("Шаблон полка '{}' имеет невалидные или отсутствующие координаты x/y", name),
            severity: "Error".to_string(),
        });
        return None;
    }

    Some(Regiment {
        name: name.to_string(),
        x: x.unwrap(),
        y: y.unwrap(),
    })
}

fn parse_division_template(
    block: &[KeyValuePair],
    path: &str,
    errors: &mut Vec<HistoryError>,
) -> Option<DivisionTemplate> {
    let mut name = None;
    let mut regiments = Vec::new();
    let mut support = Vec::new();

    for field in block {
        match field.key.as_str() {
            "name" => {
                match &field.value {
                    Value::String(s) | Value::Number(s) => name = Some(s.clone()),
                    _ => {}
                }
            }
            "regiments" => {
                if let Value::Block(reg_block) = &field.value {
                    for r_field in reg_block {
                        if let Value::Block(r_config) = &r_field.value {
                            if let Some(reg) = parse_regiment(&r_field.key, r_config, path, r_field.line_number, errors) {
                                regiments.push(reg);
                            }
                        }
                    }
                }
            }
            "support" => {
                if let Value::Block(supp_block) = &field.value {
                    for s_field in supp_block {
                        if let Value::Block(s_config) = &s_field.value {
                            if let Some(supp) = parse_regiment(&s_field.key, s_config, path, s_field.line_number, errors) {
                                support.push(supp);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let name = match name {
        Some(n) => n,
        None => {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: 1,
                message: "Отсутствует имя шаблона дивизии (name)".to_string(),
                severity: "Error".to_string(),
            });
            return None;
        }
    };

    Some(DivisionTemplate {
        name,
        regiments,
        support,
    })
}

fn parse_division(
    block: &[KeyValuePair],
    path: &str,
    line_number: usize,
    errors: &mut Vec<HistoryError>,
) -> Option<Division> {
    let mut name = None;
    let mut location = None;
    let mut template_name = None;
    let mut experience = None;
    let mut equipment = None;

    for f in block {
        match f.key.as_str() {
            "name" => {
                match &f.value {
                    Value::String(s) | Value::Number(s) => name = Some(s.clone()),
                    _ => {}
                }
            }
            "location" => {
                if let Value::Number(n) = &f.value {
                    location = n.parse::<u32>().ok();
                }
            }
            "division_template" => {
                match &f.value {
                    Value::String(s) | Value::Number(s) => template_name = Some(s.clone()),
                    _ => {}
                }
            }
            "start_experience_factor" => {
                if let Value::Number(n) = &f.value {
                    if let Ok(val) = n.parse::<f64>() {
                        experience = Some(val);
                        if val < 0.0 || val > 1.0 {
                            errors.push(HistoryError {
                                file: path.to_string(),
                                line_number: f.line_number,
                                message: format!(
                                    "Значение start_experience_factor ({}) выходит за пределы [0.0, 1.0]",
                                    val
                                ),
                                severity: "Warning".to_string(),
                            });
                        }
                    }
                }
            }
            "start_equipment_factor" => {
                if let Value::Number(n) = &f.value {
                    if let Ok(val) = n.parse::<f64>() {
                        equipment = Some(val);
                        if val < 0.0 || val > 1.0 {
                            errors.push(HistoryError {
                                file: path.to_string(),
                                line_number: f.line_number,
                                message: format!(
                                    "Значение start_equipment_factor ({}) выходит за пределы [0.0, 1.0]",
                                    val
                                ),
                                severity: "Warning".to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if location.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number,
            message: "Отсутствует координата (location) у дивизии".to_string(),
            severity: "Error".to_string(),
        });
        return None;
    }

    if template_name.is_none() {
        errors.push(HistoryError {
            file: path.to_string(),
            line_number,
            message: "Отсутствует шаблон (division_template) у дивизии".to_string(),
            severity: "Error".to_string(),
        });
        return None;
    }

    Some(Division {
        name: name.unwrap_or_default(),
        location: location.unwrap(),
        template_name: template_name.unwrap(),
        experience,
        equipment,
    })
}

fn parse_units(block: &[KeyValuePair], path: &str, errors: &mut Vec<HistoryError>) -> Vec<Division> {
    let mut divisions = Vec::new();
    for field in block {
        if field.key == "division" {
            if let Value::Block(div_block) = &field.value {
                if let Some(div) = parse_division(div_block, path, field.line_number, errors) {
                    divisions.push(div);
                }
            }
        }
    }
    divisions
}

pub fn parse_file(path: &str) -> Result<(UnitOob, Vec<HistoryError>), String> {
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
            return Ok((UnitOob::default(), errors));
        }
    };

    let mut division_templates = Vec::new();
    let mut divisions = Vec::new();

    for field in &parsed {
        match field.key.as_str() {
            "division_template" => {
                if let Value::Block(b) = &field.value {
                    if let Some(t) = parse_division_template(b, path, &mut errors) {
                        division_templates.push(t);
                    }
                }
            }
            "units" => {
                if let Value::Block(b) = &field.value {
                    divisions.extend(parse_units(b, path, &mut errors));
                }
            }
            _ => {}
        }
    }

    let defined_templates: std::collections::HashSet<String> =
        division_templates.iter().map(|t| t.name.clone()).collect();
    for div in &divisions {
        if !defined_templates.contains(&div.template_name) {
            errors.push(HistoryError {
                file: path.to_string(),
                line_number: 1,
                message: format!(
                    "Дивизия '{}' ссылается на неизвестный шаблон '{}'",
                    div.name, div.template_name
                ),
                severity: "Error".to_string(),
            });
        }
    }

    Ok((
        UnitOob {
            division_templates,
            divisions,
        },
        errors,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_oob() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("ALB_1936.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            division_template = {{
                name = "Division Template Name"
                regiments = {{
                    infantry = {{ x = 0 y = 0 }}
                    infantry = {{ x = 0 y = 1 }}
                }}
                support = {{
                    support_artillery = {{ x = 0 y = 0 }}
                }}
            }}
            units = {{
                division = {{
                    name = "1. Division"
                    location = 3838
                    division_template = "Division Template Name"
                    start_experience_factor = 0.5
                    start_equipment_factor = 0.8
                }}
            }}
        "#).unwrap();

        let (oob, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(oob.division_templates.len(), 1);
        assert_eq!(oob.division_templates[0].name, "Division Template Name");
        assert_eq!(oob.division_templates[0].regiments.len(), 2);
        assert_eq!(oob.division_templates[0].support.len(), 1);
        assert_eq!(oob.divisions.len(), 1);
        assert_eq!(oob.divisions[0].name, "1. Division");
        assert_eq!(oob.divisions[0].location, 3838);
        assert_eq!(oob.divisions[0].template_name, "Division Template Name");

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_invalid_template_ref() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("invalid_ref.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            units = {{
                division = {{
                    name = "1. Division"
                    location = 3838
                    division_template = "Nonexistent Template"
                }}
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert!(!errors.is_empty());
        assert_eq!(errors[0].severity, "Error");
        assert!(errors[0].message.contains("неизвестный шаблон"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_invalid_experience_range() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("invalid_exp.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"
            division_template = {{
                name = "Template"
            }}
            units = {{
                division = {{
                    name = "1. Division"
                    location = 3838
                    division_template = "Template"
                    start_experience_factor = 1.5
                }}
            }}
        "#).unwrap();

        let (_, errors) = parse_file(file_path.to_str().unwrap()).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, "Warning");
        assert!(errors[0].message.contains("start_experience_factor"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_field_units_files() {
        crate::parser::test_utils::run_history_field_test("units", "Units", |path| {
            parse_file(path.to_str().unwrap()).ok().map(|(_, errs)| errs.len())
        });
    }
}
