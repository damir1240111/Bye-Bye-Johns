pub mod parser;

use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MissingGfxIconError {
    pub localisation_file: String,
    pub line_number: usize,
    pub key: String,
    pub icon_name: String,
    pub expected_gfx_name: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuiSynergyError {
    pub gui_file: String,
    pub line_number: usize,
    pub element_name: String,
    pub element_type: String,
    pub referenced_name: String, // name of the missing GFX sprite or localization key
    pub error_type: String,      // "MissingSprite" or "MissingLocalisation"
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SynergyReport {
    pub missing_gfx_icons: Vec<MissingGfxIconError>,
    pub gui_errors: Vec<GuiSynergyError>,
}

fn extract_icons(value: &str) -> Vec<String> {
    let mut icons = Vec::new();
    let mut chars = value.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '£' {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_alphanumeric() || nc == '_' || nc == '.' || nc == '-' {
                    name.push(nc);
                    chars.next();
                } else if nc == '|' {
                    chars.next();
                    while let Some(&fc) = chars.peek() {
                        if fc.is_ascii_alphanumeric() || fc == '_' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    break;
                } else {
                    break;
                }
            }
            if !name.is_empty() {
                icons.push(name);
            }
        }
    }
    icons
}

fn is_potential_loc_key(s: &str) -> bool {
    if s.is_empty() || s.contains(' ') || (s.starts_with('[') && s.ends_with(']')) {
        return false;
    }
    if s == "yes" || s == "no" || s == "true" || s == "false" {
        return false;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

fn collect_gui_synergy_errors(
    element: &parser::gui::GuiElement,
    gui_path: &str,
    defined_sprites: &HashSet<String>,
    defined_loc_keys: &HashSet<String>,
    gui_errors: &mut Vec<GuiSynergyError>,
) {
    if let Some(sprite) = &element.quad_texture_sprite {
        if !defined_sprites.contains(sprite) {
            gui_errors.push(GuiSynergyError {
                gui_file: gui_path.to_string(),
                line_number: element.line_number,
                element_name: element.name.clone(),
                element_type: element.element_type.clone(),
                referenced_name: sprite.clone(),
                error_type: "MissingSprite".to_string(),
                message: format!(
                    "Строка {}: Элемент '{}' ({}) ссылается на несуществующий спрайт '{}' (quadTextureSprite)",
                    element.line_number, element.name, element.element_type, sprite
                ),
            });
        }
    }

    if let Some(sprite) = &element.sprite_type {
        if !defined_sprites.contains(sprite) {
            gui_errors.push(GuiSynergyError {
                gui_file: gui_path.to_string(),
                line_number: element.line_number,
                element_name: element.name.clone(),
                element_type: element.element_type.clone(),
                referenced_name: sprite.clone(),
                error_type: "MissingSprite".to_string(),
                message: format!(
                    "Строка {}: Элемент '{}' ({}) ссылается на несуществующий спрайт '{}' (spriteType)",
                    element.line_number, element.name, element.element_type, sprite
                ),
            });
        }
    }

    if let Some(txt) = &element.text {
        if is_potential_loc_key(txt) && !defined_loc_keys.contains(txt) {
            gui_errors.push(GuiSynergyError {
                gui_file: gui_path.to_string(),
                line_number: element.line_number,
                element_name: element.name.clone(),
                element_type: element.element_type.clone(),
                referenced_name: txt.clone(),
                error_type: "MissingLocalisation".to_string(),
                message: format!(
                    "Строка {}: Элемент '{}' ({}) ссылается на ненайденный ключ локализации '{}'",
                    element.line_number, element.name, element.element_type, txt
                ),
            });
        }
    }

    if let Some(txt) = &element.pdx_tooltip {
        if is_potential_loc_key(txt) && !defined_loc_keys.contains(txt) {
            gui_errors.push(GuiSynergyError {
                gui_file: gui_path.to_string(),
                line_number: element.line_number,
                element_name: element.name.clone(),
                element_type: element.element_type.clone(),
                referenced_name: txt.clone(),
                error_type: "MissingLocalisation".to_string(),
                message: format!(
                    "Строка {}: Элемент '{}' ({}) ссылается на ненайденный ключ локализации '{}' (pdx_tooltip)",
                    element.line_number, element.name, element.element_type, txt
                ),
            });
        }
    }

    if let Some(txt) = &element.pdx_tooltip_delayed {
        if is_potential_loc_key(txt) && !defined_loc_keys.contains(txt) {
            gui_errors.push(GuiSynergyError {
                gui_file: gui_path.to_string(),
                line_number: element.line_number,
                element_name: element.name.clone(),
                element_type: element.element_type.clone(),
                referenced_name: txt.clone(),
                error_type: "MissingLocalisation".to_string(),
                message: format!(
                    "Строка {}: Элемент '{}' ({}) ссылается на ненайденный ключ локализации '{}' (pdx_tooltip_delayed)",
                    element.line_number, element.name, element.element_type, txt
                ),
            });
        }
    }

    for child in &element.children {
        collect_gui_synergy_errors(child, gui_path, defined_sprites, defined_loc_keys, gui_errors);
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn parse_localisation_file(path: &str) -> Result<parser::localisation::LocalisationFile, String> {
    parser::localisation::parse_file(path)
}

#[tauri::command]
fn parse_gfx_file(path: &str) -> Result<parser::gfx::GfxFile, String> {
    parser::gfx::parse_file(path)
}

#[tauri::command]
fn parse_gui_file(path: &str) -> Result<parser::gui::GuiFile, String> {
    parser::gui::parse_file(path)
}

#[tauri::command]
fn validate_synergy(
    localisation_paths: Vec<String>,
    gfx_paths: Vec<String>,
    gui_paths: Vec<String>,
) -> Result<SynergyReport, String> {
    let mut defined_sprites = HashSet::new();
    for path in gfx_paths {
        if let Ok(gfx_file) = parser::gfx::parse_file(&path) {
            for entry in gfx_file.entries {
                defined_sprites.insert(entry.name);
            }
        }
    }

    let mut defined_loc_keys = HashSet::new();
    for path in &localisation_paths {
        if let Ok(loc_file) = parser::localisation::parse_file(path) {
            for entry in loc_file.entries {
                defined_loc_keys.insert(entry.key);
            }
        }
    }

    let mut missing_gfx_icons = Vec::new();

    for loc_path in &localisation_paths {
        if let Ok(loc_file) = parser::localisation::parse_file(loc_path) {
            for entry in loc_file.entries {
                let icons = extract_icons(&entry.value);
                for icon in icons {
                    let expected_gfx_name = format!("GFX_{}", icon);
                    if !defined_sprites.contains(&expected_gfx_name) {
                        missing_gfx_icons.push(MissingGfxIconError {
                            localisation_file: loc_path.clone(),
                            line_number: entry.line_number,
                            key: entry.key.clone(),
                            icon_name: icon.clone(),
                            expected_gfx_name: expected_gfx_name.clone(),
                            message: format!(
                                "Строка {}: В локализации '{}' используется иконка '£{}', но спрайт '{}' не найден в .gfx файлах",
                                entry.line_number, entry.key, icon, expected_gfx_name
                            ),
                        });
                    }
                }
            }
        }
    }

    let mut gui_errors = Vec::new();
    for gui_path in &gui_paths {
        if let Ok(gui_file) = parser::gui::parse_file(gui_path) {
            for element in &gui_file.elements {
                collect_gui_synergy_errors(element, gui_path, &defined_sprites, &defined_loc_keys, &mut gui_errors);
            }
        }
    }

    Ok(SynergyReport {
        missing_gfx_icons,
        gui_errors,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            greet, 
            parse_localisation_file,
            parse_gfx_file,
            parse_gui_file,
            validate_synergy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_synergy_validation() {
        let temp_dir = std::env::temp_dir();
        
        let gfx_path = temp_dir.join("test_synergy.gfx");
        let mut gfx_file = File::create(&gfx_path).unwrap();
        writeln!(gfx_file, r#"
            spriteTypes = {{
                spriteType = {{
                    name = "GFX_political_power"
                    texturefile = "gfx/interface/political_power.dds"
                }}
            }}
        "#).unwrap();

        let loc_path = temp_dir.join("test_synergy_l_english.yml");
        let mut loc_file = File::create(&loc_path).unwrap();
        loc_file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
        writeln!(loc_file, r#"l_english:
 LOC_KEY:0 "Costs £political_power and £missing_icon|3""#).unwrap();

        let report = validate_synergy(
            vec![loc_path.to_str().unwrap().to_string()],
            vec![gfx_path.to_str().unwrap().to_string()],
            vec![],
        ).unwrap();

        assert_eq!(report.missing_gfx_icons.len(), 1);
        assert_eq!(report.missing_gfx_icons[0].icon_name, "missing_icon");
        assert_eq!(report.missing_gfx_icons[0].expected_gfx_name, "GFX_missing_icon");
        assert_eq!(report.missing_gfx_icons[0].key, "LOC_KEY");

        let _ = std::fs::remove_file(gfx_path);
        let _ = std::fs::remove_file(loc_path);
    }

    #[test]
    fn test_gui_synergy_validation() {
        let temp_dir = std::env::temp_dir();
        
        let gfx_path = temp_dir.join("test_gui_synergy.gfx");
        let mut gfx_file = File::create(&gfx_path).unwrap();
        writeln!(gfx_file, r#"
            spriteTypes = {{
                spriteType = {{
                    name = "GFX_gui_valid_sprite"
                    texturefile = "gfx/interface/valid.dds"
                }}
            }}
        "#).unwrap();

        let loc_path = temp_dir.join("test_gui_synergy_l_english.yml");
        let mut loc_file = File::create(&loc_path).unwrap();
        loc_file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
        writeln!(loc_file, r#"l_english:
 GUI_VALID_KEY:0 "Hello World"
 UNUSED_KEY:0 "Unused""#).unwrap();

        let gui_path = temp_dir.join("test_gui_synergy.gui");
        let mut gui_file = File::create(&gui_path).unwrap();
        writeln!(gui_file, r#"
            guiTypes = {{
                containerWindowType = {{
                    name = "my_container"
                    buttonType = {{
                        name = "btn_1"
                        quadTextureSprite = "GFX_gui_valid_sprite"
                        buttonText = "GUI_VALID_KEY"
                    }}
                    buttonType = {{
                        name = "btn_2"
                        quadTextureSprite = "GFX_missing_sprite"
                        buttonText = "GUI_MISSING_KEY"
                        pdx_tooltip = "GUI_MISSING_TOOLTIP_KEY"
                    }}
                }}
            }}
        "#).unwrap();

        let report = validate_synergy(
            vec![loc_path.to_str().unwrap().to_string()],
            vec![gfx_path.to_str().unwrap().to_string()],
            vec![gui_path.to_str().unwrap().to_string()],
        ).unwrap();

        assert_eq!(report.gui_errors.len(), 3);

        let missing_sprite_err = report.gui_errors.iter().find(|e| e.error_type == "MissingSprite").unwrap();
        assert_eq!(missing_sprite_err.referenced_name, "GFX_missing_sprite");
        assert_eq!(missing_sprite_err.element_name, "btn_2");

        let missing_loc_err = report.gui_errors.iter().find(|e| e.referenced_name == "GUI_MISSING_KEY").unwrap();
        assert_eq!(missing_loc_err.element_name, "btn_2");

        let missing_tooltip_err = report.gui_errors.iter().find(|e| e.referenced_name == "GUI_MISSING_TOOLTIP_KEY").unwrap();
        assert_eq!(missing_tooltip_err.element_name, "btn_2");

        let _ = std::fs::remove_file(gfx_path);
        let _ = std::fs::remove_file(loc_path);
        let _ = std::fs::remove_file(gui_path);
    }
}
