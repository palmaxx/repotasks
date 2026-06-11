use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::models::Project;

/// Path to the central project index, creating the config dir if needed.
fn projects_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("projects.json"))
}

pub fn load_projects(app: &AppHandle) -> Result<Vec<Project>, String> {
    let path = projects_file(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save_projects(app: &AppHandle, projects: &[Project]) -> Result<(), String> {
    let path = projects_file(app)?;
    let data = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}
