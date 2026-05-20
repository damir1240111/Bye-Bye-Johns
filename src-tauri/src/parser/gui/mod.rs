use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::parser::pdx_script::{
    lex, parse_tokens, extract_line_number_from_err, Value, KeyValuePair
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiElement {
    pub element_type: String,
    pub name: String,
    pub position: Option<(i32, i32)>,
    pub size: Option<(i32, i32)>,
    pub quad_texture_sprite: Option<String>,
    pub sprite_type: Option<String>,
    pub text: Option<String>,
    pub font: Option<String>,
    pub pdx_tooltip: Option<String>,
    pub pdx_tooltip_delayed: Option<String>,
    pub line_number: usize,
    pub children: Vec<GuiElement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiError {
    pub line_number: usize,
    pub message: String,
    pub severity: String, // "Error" or "Warning"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GuiFile {
    pub elements: Vec<GuiElement>,
    pub errors: Vec<GuiError>,
}

fn resolve_val<'a>(val: &'a str, constants: &'a HashMap<String, String>) -> &'a str {
    if val.starts_with('@') {
        if let Some(resolved) = constants.get(val) {
            return resolved.as_str();
        }
    }
    val
}

fn parse_xy(value: &Value, constants: &HashMap<String, String>) -> Option<(i32, i32)> {
    if let Value::Block(pairs) = value {
        let mut x = None;
        let mut y = None;
        for pair in pairs {
            let key = pair.key.to_lowercase();
            if key == "x" {
                if let Value::Number(s) = &pair.value {
                    let resolved = resolve_val(s, constants);
                    if resolved.starts_with('@') {
                        x = Some(0); // placeholder for unresolved constant
                    } else {
                        x = resolved.parse::<i32>().ok();
                    }
                }
            } else if key == "y" {
                if let Value::Number(s) = &pair.value {
                    let resolved = resolve_val(s, constants);
                    if resolved.starts_with('@') {
                        y = Some(0); // placeholder
                    } else {
                        y = resolved.parse::<i32>().ok();
                    }
                }
            }
        }
        if let (Some(xv), Some(yv)) = (x, y) {
            return Some((xv, yv));
        }
    }
    None
}

fn parse_size(value: &Value, constants: &HashMap<String, String>) -> Option<(i32, i32)> {
    if let Value::Block(pairs) = value {
        let mut width = None;
        let mut height = None;
        for pair in pairs {
            let key = pair.key.to_lowercase();
            if key == "width" || key == "x" {
                if let Value::Number(s) = &pair.value {
                    let resolved = resolve_val(s, constants);
                    if resolved.starts_with('@') {
                        width = Some(0); // placeholder
                    } else {
                        width = resolved.parse::<i32>().ok();
                    }
                }
            } else if key == "height" || key == "y" {
                if let Value::Number(s) = &pair.value {
                    let resolved = resolve_val(s, constants);
                    if resolved.starts_with('@') {
                        height = Some(0); // placeholder
                    } else {
                        height = resolved.parse::<i32>().ok();
                    }
                }
            }
        }
        if let (Some(w), Some(h)) = (width, height) {
            return Some((w, h));
        }
    }
    None
}

fn validate_sprite_name(name: &str, line_number: usize, constants: &HashMap<String, String>, errors: &mut Vec<GuiError>) {
    let resolved = resolve_val(name, constants);
    if resolved.starts_with('@') {
        return; // skip warning for unresolved constants
    }
    if !resolved.starts_with("GFX_") {
        errors.push(GuiError {
            line_number,
            message: format!("Предупреждение: Имя спрайта '{}' нарушает конвенцию именования (должно начинаться с 'GFX_')", resolved),
            severity: "Warning".to_string(),
        });
    }
}

fn parse_element_block(
    element_type: &str,
    value: &Value,
    line_number: usize,
    constants: &HashMap<String, String>,
    errors: &mut Vec<GuiError>,
) -> Option<GuiElement> {
    let fields = match value {
        Value::Block(f) => f,
        _ => return None,
    };

    let mut name = String::new();
    let mut position = None;
    let mut size = None;
    let mut quad_texture_sprite = None;
    let mut sprite_type = None;
    let mut text = None;
    let mut font = None;
    let mut pdx_tooltip = None;
    let mut pdx_tooltip_delayed = None;
    let mut children = Vec::new();

    for field in fields {
        let key_lower = field.key.to_lowercase();
        match key_lower.as_str() {
            "name" => {
                if let Value::String(s) = &field.value {
                    name = s.clone();
                } else if let Value::Number(n) = &field.value {
                    name = n.clone();
                } else {
                    errors.push(GuiError {
                        line_number: field.line_number,
                        message: "Поле 'name' должно быть строковым значением".to_string(),
                        severity: "Error".to_string(),
                    });
                }
            }
            "position" => {
                position = parse_xy(&field.value, constants);
                if position.is_none() {
                    errors.push(GuiError {
                        line_number: field.line_number,
                        message: "Некорректный формат координат в 'position'. Ожидалось { x = ... y = ... }".to_string(),
                        severity: "Error".to_string(),
                    });
                }
            }
            "size" => {
                size = parse_size(&field.value, constants);
                if size.is_none() {
                    errors.push(GuiError {
                        line_number: field.line_number,
                        message: "Некорректный формат размеров в 'size'. Ожидалось { width = ... height = ... } или { x = ... y = ... }".to_string(),
                        severity: "Error".to_string(),
                    });
                }
            }
            "quadtexturesprite" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    quad_texture_sprite = Some(resolved);
                    validate_sprite_name(s, field.line_number, constants, errors);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    quad_texture_sprite = Some(resolved);
                    validate_sprite_name(n, field.line_number, constants, errors);
                }
            }
            "spritetype" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    sprite_type = Some(resolved);
                    validate_sprite_name(s, field.line_number, constants, errors);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    sprite_type = Some(resolved);
                    validate_sprite_name(n, field.line_number, constants, errors);
                }
            }
            "text" | "buttontext" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    text = Some(resolved);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    text = Some(resolved);
                }
            }
            "font" | "buttonfont" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    font = Some(resolved);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    font = Some(resolved);
                }
            }
            "pdx_tooltip" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    pdx_tooltip = Some(resolved);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    pdx_tooltip = Some(resolved);
                }
            }
            "pdx_tooltip_delayed" => {
                if let Value::String(s) = &field.value {
                    let resolved = resolve_val(s, constants).to_string();
                    pdx_tooltip_delayed = Some(resolved);
                } else if let Value::Number(n) = &field.value {
                    let resolved = resolve_val(n, constants).to_string();
                    pdx_tooltip_delayed = Some(resolved);
                }
            }
            "bordersize" | "scrolloffset" => {
                if parse_xy(&field.value, constants).is_none() {
                    errors.push(GuiError {
                        line_number: field.line_number,
                        message: format!("Некорректный формат координат в '{}'", field.key),
                        severity: "Error".to_string(),
                    });
                }
            }
            _ => {
                if let Value::Block(_) = &field.value {
                    if let Some(child_el) = parse_element_block(&field.key, &field.value, field.line_number, constants, errors) {
                        children.push(child_el);
                    }
                }
            }
        }
    }

    if name.is_empty() && element_type != "background" && !element_type.to_lowercase().contains("scrollbar") {
        errors.push(GuiError {
            line_number,
            message: format!("Предупреждение: Элемент '{}' не содержит обязательное свойство 'name'", element_type),
            severity: "Warning".to_string(),
        });
    }

    Some(GuiElement {
        element_type: element_type.to_string(),
        name,
        position,
        size,
        quad_texture_sprite,
        sprite_type,
        text,
        font,
        pdx_tooltip,
        pdx_tooltip_delayed,
        line_number,
        children,
    })
}

fn validate_ast(ast: &[KeyValuePair], file_name: &str) -> (Vec<GuiElement>, Vec<GuiError>) {
    let mut elements = Vec::new();
    let mut errors = Vec::new();
    let mut gui_types_found = false;

    if !file_name.is_empty() && !file_name.ends_with(".gui") {
        errors.push(GuiError {
            line_number: 1,
            message: format!("Предупреждение: Имя файла '{}' должно иметь расширение '.gui'", file_name),
            severity: "Warning".to_string(),
        });
    }

    // Извлекаем константы на верхнем уровне
    let mut constants = HashMap::new();
    for pair in ast {
        if pair.key.starts_with('@') {
            if let Value::Number(s) = &pair.value {
                constants.insert(pair.key.clone(), s.clone());
            } else if let Value::String(s) = &pair.value {
                constants.insert(pair.key.clone(), s.clone());
            }
        }
    }

    for pair in ast {
        if pair.key.starts_with('@') {
            continue; // Пропускаем объявления констант
        }

        if pair.key != "guiTypes" {
            errors.push(GuiError {
                line_number: pair.line_number,
                message: format!("Недопустимый корневой элемент '{}'. Все элементы должны находиться внутри блока 'guiTypes'", pair.key),
                severity: "Error".to_string(),
            });
            continue;
        }

        gui_types_found = true;

        match &pair.value {
            Value::Block(sub_pairs) => {
                for sub_pair in sub_pairs {
                    if let Value::Block(_) = &sub_pair.value {
                        if let Some(el) = parse_element_block(&sub_pair.key, &sub_pair.value, sub_pair.line_number, &constants, &mut errors) {
                            elements.push(el);
                        }
                    } else {
                        errors.push(GuiError {
                            line_number: sub_pair.line_number,
                            message: format!("Элемент '{}' внутри guiTypes должен быть блоком в фигурных скобках", sub_pair.key),
                            severity: "Error".to_string(),
                        });
                    }
                }
            }
            _ => {
                errors.push(GuiError {
                    line_number: pair.line_number,
                    message: "Блок 'guiTypes' должен содержать фигурные скобки с описанием элементов интерфейса".to_string(),
                    severity: "Error".to_string(),
                });
            }
        }
    }

    if !gui_types_found && errors.is_empty() {
        errors.push(GuiError {
            line_number: 1,
            message: "В файле отсутствует обязательный корневой блок 'guiTypes'".to_string(),
            severity: "Error".to_string(),
        });
    }

    (elements, errors)
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<GuiFile, String> {
    let file_path = path.as_ref();
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mut errors = Vec::new();

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(format!("Не удалось прочитать файл: {}", e));
        }
    };

    let tokens = lex(&content);

    let ast = match parse_tokens(&tokens) {
        Ok(a) => a,
        Err(e) => {
            let line_number = extract_line_number_from_err(&e).unwrap_or(1);
            errors.push(GuiError {
                line_number,
                message: format!("Ошибка синтаксиса: {}", e),
                severity: "Error".to_string(),
            });
            return Ok(GuiFile {
                elements: Vec::new(),
                errors,
            });
        }
    };

    let (elements, val_errors) = validate_ast(&ast, file_name);
    errors.extend(val_errors);

    Ok(GuiFile { elements, errors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::test_utils::TempFile;

    fn make_temp_file(content: &str) -> TempFile {
        TempFile::new_with_filename(content, "temp_ui.gui")
    }

    #[test]
    fn test_parse_gui_happy_path() {
        let content = r#"
            guiTypes = {
                containerWindowType = {
                    name = "my_custom_window"
                    position = { x = 500 y = 200 }
                    size = { width = 300 height = 200 }
                    
                    background = {
                        name = "Background"
                        quadTextureSprite = "GFX_tiled_window_bg"
                    }

                    instantTextBoxType = {
                        name = "title_text"
                        position = { x = 20 y = 15 }
                        font = "hoi_18mbs"
                        text = "My Custom GUI"
                    }

                    buttonType = {
                        name = "my_button"
                        position = { x = 50 y = 100 }
                        quadTextureSprite = "GFX_button_123x34"
                        buttonText = "Click Me!"
                        buttonFont = "hoi_16mbs"
                        pdx_tooltip = "TOOLTIP_LOC"
                        pdx_tooltip_delayed = "TOOLTIP_LOC_DELAYED"
                    }
                }
            }
        "#;
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.elements.len(), 1);
        let container = &res.elements[0];
        assert_eq!(container.element_type, "containerWindowType");
        assert_eq!(container.name, "my_custom_window");
        assert_eq!(container.position, Some((500, 200)));
        assert_eq!(container.size, Some((300, 200)));
        assert_eq!(container.children.len(), 3);

        let bg = &container.children[0];
        assert_eq!(bg.element_type, "background");
        assert_eq!(bg.name, "Background");
        assert_eq!(bg.quad_texture_sprite, Some("GFX_tiled_window_bg".to_string()));

        let text = &container.children[1];
        assert_eq!(text.element_type, "instantTextBoxType");
        assert_eq!(text.name, "title_text");
        assert_eq!(text.font, Some("hoi_18mbs".to_string()));
        assert_eq!(text.text, Some("My Custom GUI".to_string()));

        let btn = &container.children[2];
        assert_eq!(btn.element_type, "buttonType");
        assert_eq!(btn.name, "my_button");
        assert_eq!(btn.quad_texture_sprite, Some("GFX_button_123x34".to_string()));
        assert_eq!(btn.text, Some("Click Me!".to_string()));
        assert_eq!(btn.font, Some("hoi_16mbs".to_string()));
        assert_eq!(btn.pdx_tooltip, Some("TOOLTIP_LOC".to_string()));
        assert_eq!(btn.pdx_tooltip_delayed, Some("TOOLTIP_LOC_DELAYED".to_string()));
    }

    #[test]
    fn test_parse_gui_nested() {
        let content = r#"
            guiTypes = {
                containerWindowType = {
                    name = "outer"
                    containerWindowType = {
                        name = "inner"
                        iconType = {
                            name = "my_icon"
                            spriteType = "GFX_icon"
                        }
                    }
                }
            }
        "#;
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.elements.len(), 1);
        let outer = &res.elements[0];
        assert_eq!(outer.name, "outer");
        assert_eq!(outer.children.len(), 1);

        let inner = &outer.children[0];
        assert_eq!(inner.element_type, "containerWindowType");
        assert_eq!(inner.name, "inner");
        assert_eq!(inner.children.len(), 1);

        let icon = &inner.children[0];
        assert_eq!(icon.element_type, "iconType");
        assert_eq!(icon.name, "my_icon");
        assert_eq!(icon.sprite_type, Some("GFX_icon".to_string()));
    }

    #[test]
    fn test_parse_gui_constants() {
        let content = r#"
            @win_w = 400
            @win_h = 300
            @btn_sprite = "GFX_my_btn"
            @tooltip_loc = "MY_LOC"

            guiTypes = {
                containerWindowType = {
                    name = "const_win"
                    size = { width = @win_w height = @win_h }
                    buttonType = {
                        name = "btn"
                        quadTextureSprite = @btn_sprite
                        pdx_tooltip = @tooltip_loc
                    }
                }
            }
        "#;
        let file = make_temp_file(content);
        let res = parse_file(file.path()).unwrap();

        assert_eq!(res.errors.len(), 0);
        assert_eq!(res.elements.len(), 1);
        let win = &res.elements[0];
        assert_eq!(win.size, Some((400, 300)));
        
        let btn = &win.children[0];
        assert_eq!(btn.quad_texture_sprite, Some("GFX_my_btn".to_string()));
        assert_eq!(btn.pdx_tooltip, Some("MY_LOC".to_string()));
    }

    #[test]
    fn test_parse_gui_errors() {
        let content = r#"
            guiTypes = {
                containerWindowType = {
                    position = { x = abc y = 20 }
                    size = { width = 100 }
                    buttonType = {
                        quadTextureSprite = "non_gfx_prefix_sprite"
                    }
                }
            }
        "#;
        let file = TempFile::new_with_filename(content, "invalid_extension.txt");
        let res = parse_file(file.path()).unwrap();

        assert!(res.errors.len() >= 5);
        
        let has_ext_warning = res.errors.iter().any(|e| e.message.contains("должно иметь расширение '.gui'") && e.severity == "Warning");
        let has_pos_error = res.errors.iter().any(|e| e.message.contains("Некорректный формат координат в 'position'") && e.severity == "Error");
        let has_size_error = res.errors.iter().any(|e| e.message.contains("Некорректный формат размеров в 'size'") && e.severity == "Error");
        let has_name_warning = res.errors.iter().any(|e| e.message.contains("не содержит обязательное свойство 'name'") && e.severity == "Warning");
        let has_prefix_warning = res.errors.iter().any(|e| e.message.contains("нарушает конвенцию именования") && e.severity == "Warning");

        assert!(has_ext_warning);
        assert!(has_pos_error);
        assert!(has_size_error);
        assert!(has_name_warning);
        assert!(has_prefix_warning);
    }

    #[test]
    fn test_field_gui_files() {
        crate::parser::test_utils::run_field_test("gui", "GUI", |path| {
            parse_file(path).ok().map(|res| res.errors.len())
        });
    }
}
