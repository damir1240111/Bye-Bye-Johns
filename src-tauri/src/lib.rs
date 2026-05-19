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
pub struct SynergyReport {
    pub missing_gfx_icons: Vec<MissingGfxIconError>,
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
fn validate_synergy(localisation_paths: Vec<String>, gfx_paths: Vec<String>) -> Result<SynergyReport, String> {
    let mut defined_sprites = HashSet::new();
    for path in gfx_paths {
        if let Ok(gfx_file) = parser::gfx::parse_file(&path) {
            for entry in gfx_file.entries {
                defined_sprites.insert(entry.name);
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

    Ok(SynergyReport { missing_gfx_icons })
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
        ).unwrap();

        assert_eq!(report.missing_gfx_icons.len(), 1);
        assert_eq!(report.missing_gfx_icons[0].icon_name, "missing_icon");
        assert_eq!(report.missing_gfx_icons[0].expected_gfx_name, "GFX_missing_icon");
        assert_eq!(report.missing_gfx_icons[0].key, "LOC_KEY");

        let _ = std::fs::remove_file(gfx_path);
        let _ = std::fs::remove_file(loc_path);
    }
}

