use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
    rc::Rc,
};

use chrono::Local;
use dirs;
use rfd::FileDialog;
use serde_json::json;
use slint::SharedString;

// ---------------- SLINT UI ---------------- //
slint::slint! {
    component MainWindow inherits Window {  // Removed "export" - not needed for components in Slint
        in-out property <string> cod_path_text: "Not set";
        in-out property <string> status_text: "";

        callback select_cod_folder();
        callback export_settings();
        callback import_settings();
        callback backup_settings();

        title: "MW3 Settings Tool";
        width: 420px;
        height: 280px;

        VerticalLayout {
            spacing: 12px;
            padding: 12px;

            Text { text: cod_path_text; }

            HorizontalLayout {
                spacing: 8px;
                Button { text: "Select Folder"; clicked => root.select_cod_folder(); }
            }

            HorizontalLayout {
                spacing: 8px;
                Button { text: "Export"; clicked => root.export_settings(); }
                Button { text: "Import"; clicked => root.import_settings(); }
                Button { text: "Backup"; clicked => root.backup_settings(); }
            }

            Text { text: status_text; }
        }
    }
}
// ------------------------------------------ //

thread_local! {
    static COD_PATH: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
}

fn main() -> io::Result<()> {
    let ui = MainWindow::new().unwrap();

    // Detect default COD install path
    if let Some(default_path) = check_cod_default() {
        COD_PATH.with(|p| *p.borrow_mut() = Some(default_path.clone()));
        ui.set_cod_path_text(SharedString::from(default_path.display().to_string()));
    }

    let ui_handle = ui.as_weak();
    ui.on_select_cod_folder(move || {
        let ui = ui_handle.unwrap();
        if let Some(selected) = FileDialog::new().pick_folder() {
            let cod_exe = selected.join("cod.exe");
            if cod_exe.is_file() {
                COD_PATH.with(|p| *p.borrow_mut() = Some(selected.clone()));
                ui.set_cod_path_text(SharedString::from(selected.display().to_string()));
                ui.set_status_text("COD path set successfully.".into());
            } else {
                ui.set_status_text("cod.exe not found in selected folder.".into());
            }
        } else {
            ui.set_status_text("No folder selected.".into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_export_settings(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_text("Exporting settings...".into());

        if let Ok(settings_file) = find_cod_settings() {
            match parse_cod_settings(&settings_file) {
                Ok(settings) => {
                    let filters = ["mouse", "fov", "brightness", "hdr", "adssensitivity", "gamepad", "sprint"];
                    let export_path = Path::new("cod_settings_export.json");
                    if let Err(e) = export_to_json(&settings, export_path, &filters) {
                        ui.set_status_text(SharedString::from(format!("Export failed: {}", e)));
                    } else {
                        ui.set_status_text(SharedString::from(format!("Export saved to {:?}", export_path)));
                    }
                }
                Err(e) => ui.set_status_text(SharedString::from(format!("Parse failed: {}", e))),
            }
        } else {
            ui.set_status_text("Could not find settings file.".into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_import_settings(move || {
        let ui = ui_handle.unwrap();
        if let Some(json_path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            ui.set_status_text("Importing...".into());
            if let Ok(settings_file) = find_cod_settings() {
                if let Err(e) = import_from_json(&settings_file, &json_path) {
                    ui.set_status_text(SharedString::from(format!("Import failed: {}", e)));
                } else {
                    ui.set_status_text(SharedString::from(format!("Import successful from {:?}", json_path)));
                }
            } else {
                ui.set_status_text("Settings file not found.".into());
            }
        } else {
            ui.set_status_text("No JSON selected.".into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_backup_settings(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_text("Creating backup...".into());
        if let Ok(settings_file) = find_cod_settings() {
            let backup_path = settings_file.with_extension(format!(
                "bak_{}",
                Local::now().format("%Y%m%d_%H%M%S")
            ));
            if let Err(e) = fs::copy(&settings_file, &backup_path) {
                ui.set_status_text(SharedString::from(format!("Backup failed: {}", e)));
            } else {
                ui.set_status_text(SharedString::from(format!("Backup created at {:?}", backup_path)));
            }
        } else {
            ui.set_status_text("Settings file not found.".into());
        }
    });

    ui.run().unwrap();
    Ok(())
}

// ---------------- HELPER FUNCTIONS ---------------- //

fn check_cod_default() -> Option<PathBuf> {
    let default_path = Path::new(r"C:/Program Files (x86)/Steam/steamapps/common/Call of Duty");
    let cod_exe = default_path.join("cod.exe");
    if cod_exe.is_file() {
        Some(default_path.to_path_buf())
    } else {
        None
    }
}

fn find_cod_settings() -> io::Result<PathBuf> {
    let docs = dirs::document_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Documents folder not found"))?;
    let base = docs.join("Call of Duty/players");
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path().join("config.cfg");
            if path.exists() {
                return Ok(path);
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Settings file not found"))
}

fn parse_cod_settings(path: &Path) -> io::Result<HashMap<String, String>> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut map = HashMap::new();
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    Ok(map)
}

fn export_to_json(settings: &HashMap<String, String>, output: &Path, filters: &[&str]) -> io::Result<()> {
    let filtered: HashMap<_, _> = settings
        .iter()
        .filter(|(k, _)| filters.iter().any(|f| k.to_lowercase().contains(f)))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    fs::write(output, serde_json::to_string_pretty(&json!(filtered))?)?;
    Ok(())
}

fn import_from_json(settings_file: &Path, json_path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(json_path)?;
    let imported: HashMap<String, String> = serde_json::from_str(&content)?;
    let mut lines: Vec<String> = fs::read_to_string(settings_file)?
        .lines()
        .map(|l| l.to_string())
        .collect();

    // Improved logic: Update existing lines or add new ones if the key doesn't exist
    let mut updated = false;
    for (k, v) in &imported {
        let mut found = false;
        for line in &mut lines {
            if line.trim_start().starts_with(k) && line.contains('=') {
                *line = format!("{} = {}", k, v);
                found = true;
                updated = true;
                break;
            }
        }
        if !found {
            lines.push(format!("{} = {}", k, v));  // Add new key-value if not found
            updated = true;
        }
    }

    if updated {
        fs::write(settings_file, lines.join("\n"))?;
    }
    Ok(())
}
