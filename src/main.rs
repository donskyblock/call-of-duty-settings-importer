use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use chrono::Local;
use dirs;
use eframe::egui;
use rfd::FileDialog;
use serde_json::json;

struct SettingsApp {
    cod_path: Option<PathBuf>,
    status_text: String,
}

impl Default for SettingsApp {
    fn default() -> Self {
        Self {
            cod_path: check_cod_default(),
            status_text: String::new(),
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Path display
                ui.horizontal(|ui| {
                    ui.label("Game Path: ");
                    ui.label(self.cod_path.as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not set".to_string()));
                });

                // Select folder button
                if ui.button("Select Folder").clicked() {
                    if let Some(selected) = FileDialog::new().pick_folder() {
                        let cod_exe = selected.join("cod.exe");
                        if cod_exe.is_file() {
                            self.cod_path = Some(selected);
                            self.status_text = "COD path set successfully.".to_string();
                        } else {
                            self.status_text = "cod.exe not found in selected folder.".to_string();
                        }
                    } else {
                        self.status_text = "No folder selected.".to_string();
                    }
                }

                ui.horizontal(|ui| {
                    // Export button
                    if ui.button("Export").clicked() {
                        self.status_text = "Exporting settings...".to_string();
                        match find_cod_settings() {
                            Ok(settings_files) => {
                                let filters = ["mouse", "fov", "brightness", "hdr", "adssensitivity", "gamepad", "sprint"];
                                let export_path = Path::new("cod_settings_export.json");
                                
                                // Show which files we're exporting from
                                println!("Exporting settings from:");
                                for file in &settings_files {
                                    println!("  - {}", file.display());
                                }
                                
                                match export_to_json(&settings_files, export_path, &filters) {
                                    Ok(_) => {
                                        println!("Settings exported to: {}", export_path.display());
                                        self.status_text = format!("Settings exported to cod_settings_export.json");
                                    },
                                    Err(e) => {
                                        println!("Export error: {}", e);
                                        self.status_text = format!("Export failed: {}", e);
                                    }
                                }
                            }
                            Err(e) => self.status_text = format!("Could not find settings files: {}", e),
                        }
                    }

                    // Import button
                    if ui.button("Import").clicked() {
                        if let Some(json_path) = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Select settings file to import")
                            .pick_file() 
                        {
                            self.status_text = "Importing settings...".to_string();
                            println!("Importing settings from: {}", json_path.display());
                            
                            match find_cod_settings() {
                                Ok(settings_files) => {
                                    println!("Found these game settings files:");
                                    for file in &settings_files {
                                        println!("  - {}", file.display());
                                    }
                                    
                                    match import_from_json(&settings_files, &json_path) {
                                        Ok(_) => {
                                            println!("Settings imported successfully!");
                                            self.status_text = "Settings imported successfully!".to_string();
                                        },
                                        Err(e) => {
                                            println!("Import error: {}", e);
                                            self.status_text = format!("Import failed: {}", e);
                                        }
                                    }
                                }
                                Err(e) => self.status_text = format!("Could not find settings files: {}", e),
                            }
                        } else {
                            self.status_text = "No JSON selected.".to_string();
                        }
                    }

                    // Backup button
                    if ui.button("Backup").clicked() {
                        self.status_text = "Creating backups...".to_string();
                        match find_cod_settings() {
                            Ok(settings_files) => {
                                let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                                let mut success_count = 0;
                                let mut error_messages = Vec::new();

                                for settings_file in settings_files {
                                    let file_name = settings_file.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown");
                                    
                                    // Create backup in the same directory as the original file
                                    let backup_path = settings_file.with_file_name(
                                        format!("{}.bak_{}", file_name, timestamp)
                                    );

                                    match fs::copy(&settings_file, &backup_path) {
                                        Ok(_) => {
                                            success_count += 1;
                                            println!("Backed up: {} -> {}", 
                                                settings_file.display(), 
                                                backup_path.display());
                                        },
                                        Err(e) => error_messages.push(format!("{}: {}", file_name, e)),
                                    }
                                }

                                if error_messages.is_empty() {
                                    self.status_text = format!("Successfully backed up {} files", success_count);
                                } else {
                                    self.status_text = format!("Backed up {} files. Errors: {}", 
                                        success_count, 
                                        error_messages.join(", ")
                                    );
                                }
                            }
                            Err(e) => self.status_text = format!("Could not find settings files: {}", e),
                        }
                    }
                });

                // Status text
                ui.label(&self.status_text);
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some(egui::vec2(420.0, 280.0));
    options.window_builder = Some(Box::new(|b| {
        b.with_resizable(true)
            .with_min_inner_size(egui::vec2(420.0, 280.0))
            .with_title("MW3 Settings Tool")
    }));

    eframe::run_native(
        "MW3 Settings Tool",
        options,
        Box::new(|_cc| Box::<SettingsApp>::default()),
    )
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

fn find_cod_settings() -> io::Result<Vec<PathBuf>> {
    let docs = dirs::document_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Documents folder not found"))?;
    
    // Find all steam user folders in the Call of Duty players directory
    let base = docs.join("Call of Duty/players");
    let mut settings_files = Vec::new();

    // Look in each user directory for settings files
    if let Ok(user_dirs) = fs::read_dir(&base) {
        for user_dir in user_dirs.filter_map(Result::ok) {
            if user_dir.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                // Look for g.*.txt* files in each user directory
                if let Ok(entries) = fs::read_dir(user_dir.path()) {
                    for entry in entries.filter_map(Result::ok) {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        // Match patterns like 'g.1.0.l.txt0', 'g.1.0.l.txt1', etc.
                        if file_name.starts_with("g.") && (file_name.ends_with(".txt0") || file_name.ends_with(".txt1")) {
                            settings_files.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    if settings_files.is_empty() {
        Err(io::Error::new(io::ErrorKind::NotFound, "No settings files found"))
    } else {
        Ok(settings_files)
    }
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

fn export_to_json(settings_files: &[PathBuf], output: &Path, filters: &[&str]) -> io::Result<()> {
    let mut all_settings = HashMap::new();
    
    for file in settings_files {
        let file_name = file.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        if let Ok(settings) = parse_cod_settings(file) {
            let filtered: HashMap<_, _> = settings
                .iter()
                .filter(|(k, _)| filters.iter().any(|f| k.to_lowercase().contains(f)))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            
            all_settings.insert(file_name, filtered);
        }
    }

    fs::write(output, serde_json::to_string_pretty(&json!(all_settings))?)?;
    Ok(())
}

fn import_from_json(settings_files: &[PathBuf], json_path: &Path) -> io::Result<()> {
    let content = fs::read_to_string(json_path)?;
    let imported: HashMap<String, HashMap<String, String>> = serde_json::from_str(&content)?;

    for file in settings_files {
        let file_name = file.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();

        if let Some(settings) = imported.get(&file_name) {
            let mut lines: Vec<String> = fs::read_to_string(file)?
                .lines()
                .map(|l| l.to_string())
                .collect();

            let mut updated = false;
            for (k, v) in settings {
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
                    lines.push(format!("{} = {}", k, v));
                    updated = true;
                }
            }

            if updated {
                fs::write(file, lines.join("\n"))?;
            }
        }
    }
    Ok(())
}
