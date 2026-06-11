use std::fs;
use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::models::{Entry, Project, RepoConfig};
use crate::notes;
use crate::store;

/// Sticky-note palette; new projects cycle through it for visual variety.
const PALETTE: [&str; 6] = [
    "#ffd966", "#f6a5c0", "#a0d8b3", "#a5c8f6", "#d8b3f6", "#f6c8a0",
];

const NOTE_FILE: &str = "NOTES.md";

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Write the sidecar config + seed note file into `dir`. Idempotent: never
/// overwrites either file if it already exists. Pure (no Tauri context) so it
/// can be unit-tested. Returns the note file name stored in the config.
fn provision_files(
    dir: &Path,
    name: &str,
    color: &str,
    id: &str,
    now: &str,
) -> Result<String, String> {
    let config_path = dir.join(".repotasks.json");
    if !config_path.exists() {
        let cfg = RepoConfig {
            id: id.to_string(),
            name: name.to_string(),
            color: color.to_string(),
            created_at: now.to_string(),
            note_file: NOTE_FILE.to_string(),
        };
        let data = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
        fs::write(&config_path, data).map_err(|e| e.to_string())?;
    }

    let note_path = dir.join(NOTE_FILE);
    if !note_path.exists() {
        let template = format!("# {name} — Notes\n\n## Inbox\n\n## Notes\n");
        fs::write(&note_path, template).map_err(|e| e.to_string())?;
    }

    Ok(NOTE_FILE.to_string())
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    store::load_projects(&app)
}

/// Register a folder as a project, provisioning its NOTES.md + .repotasks.json
/// in place. Idempotent: re-importing a known path returns the existing entry
/// and never overwrites files that already exist.
#[tauri::command]
pub fn import_project(app: AppHandle, path: String) -> Result<Project, String> {
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a folder: {path}"));
    }

    let mut projects = store::load_projects(&app)?;
    let abs = dir.to_string_lossy().to_string();
    if let Some(existing) = projects.iter().find(|p| p.path == abs) {
        return Ok(existing.clone());
    }

    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".to_string());
    let color = PALETTE[projects.len() % PALETTE.len()].to_string();
    let id = new_id();
    let now = now_iso();

    provision_files(dir, &name, &color, &id, &now)?;

    let project = Project {
        id,
        name,
        path: abs,
        color,
        pinned: false,
        added_at: now,
    };
    projects.push(project.clone());
    store::save_projects(&app, &projects)?;
    Ok(project)
}

/// Forget a project from the central index. Files in the repo are left intact.
#[tauri::command]
pub fn remove_project(app: AppHandle, id: String) -> Result<(), String> {
    let mut projects = store::load_projects(&app)?;
    let before = projects.len();
    projects.retain(|p| p.id != id);
    if projects.len() == before {
        return Err(format!("No project with id {id}"));
    }
    store::save_projects(&app, &projects)
}

/// Append a note/todo under the project's `## Inbox`, stamped with local time.
#[tauri::command]
pub fn add_entry(
    app: AppHandle,
    project_id: String,
    text: String,
    is_todo: bool,
) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Cannot add an empty note".into());
    }

    let projects = store::load_projects(&app)?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("No project with id {project_id}"))?;

    let note_path = Path::new(&project.path).join(NOTE_FILE);
    let content = if note_path.exists() {
        fs::read_to_string(&note_path).map_err(|e| e.to_string())?
    } else {
        format!("# {} — Notes\n\n## Inbox\n\n## Notes\n", project.name)
    };

    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let line = notes::format_entry(text, is_todo, &stamp);
    let updated = notes::append_to_inbox(&content, &line);
    fs::write(&note_path, updated).map_err(|e| e.to_string())
}

/// Parse a project's NOTES.md into structured notes/todos for the board.
#[tauri::command]
pub fn read_notes(app: AppHandle, project_id: String) -> Result<Vec<Entry>, String> {
    let projects = store::load_projects(&app)?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("No project with id {project_id}"))?;

    let note_path = Path::new(&project.path).join(NOTE_FILE);
    if !note_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&note_path).map_err(|e| e.to_string())?;
    Ok(notes::parse_notes(&content))
}

/// Pin or unpin a project (pinned cards sort first on the board).
#[tauri::command]
pub fn set_pinned(app: AppHandle, id: String, pinned: bool) -> Result<(), String> {
    let mut projects = store::load_projects(&app)?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("No project with id {id}"))?;
    project.pinned = pinned;
    store::save_projects(&app, &projects)
}

/// Resolve a project's NOTES.md, apply a pure rewrite, and persist it.
fn rewrite_notes(
    app: &AppHandle,
    project_id: &str,
    f: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let project = find_project(app, project_id)?;
    let note_path = Path::new(&project.path).join(NOTE_FILE);
    let content = fs::read_to_string(&note_path).map_err(|e| e.to_string())?;
    let updated = f(&content)?;
    fs::write(&note_path, updated).map_err(|e| e.to_string())
}

fn find_project(app: &AppHandle, project_id: &str) -> Result<Project, String> {
    store::load_projects(app)?
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("No project with id {project_id}"))
}

/// Flip a todo's checkbox at the given 0-based file line.
#[tauri::command]
pub fn toggle_todo(app: AppHandle, project_id: String, line: usize) -> Result<(), String> {
    rewrite_notes(&app, &project_id, |c| notes::toggle_todo_at(c, line))
}

/// Replace the text of an entry at the given line (keeps marker + timestamp).
#[tauri::command]
pub fn update_entry(
    app: AppHandle,
    project_id: String,
    line: usize,
    text: String,
) -> Result<(), String> {
    rewrite_notes(&app, &project_id, |c| notes::update_text_at(c, line, &text))
}

/// Delete the entry at the given line.
#[tauri::command]
pub fn delete_entry(app: AppHandle, project_id: String, line: usize) -> Result<(), String> {
    rewrite_notes(&app, &project_id, |c| notes::delete_at(c, line))
}

/// Open the repo folder in the OS file manager.
#[tauri::command]
pub fn open_folder(app: AppHandle, project_id: String) -> Result<(), String> {
    let project = find_project(&app, &project_id)?;
    app.opener()
        .open_path(project.path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Open the project's NOTES.md with the OS default handler.
#[tauri::command]
pub fn open_in_editor(app: AppHandle, project_id: String) -> Result<(), String> {
    let project = find_project(&app, &project_id)?;
    let note_path = Path::new(&project.path).join(NOTE_FILE);
    app.opener()
        .open_path(note_path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisions_notes_and_config_then_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("repotasks_test_{}", new_id()));
        fs::create_dir_all(&dir).unwrap();

        // First import creates both files with the expected content.
        let note_file =
            provision_files(&dir, "MyRepo", "#ffd966", "id-123", "2026-06-11T10:00:00").unwrap();
        assert_eq!(note_file, "NOTES.md");

        let notes = fs::read_to_string(dir.join("NOTES.md")).unwrap();
        assert!(notes.contains("# MyRepo — Notes"));
        assert!(notes.contains("## Inbox"));

        let cfg: RepoConfig =
            serde_json::from_str(&fs::read_to_string(dir.join(".repotasks.json")).unwrap()).unwrap();
        assert_eq!(cfg.name, "MyRepo");
        assert_eq!(cfg.color, "#ffd966");
        assert_eq!(cfg.note_file, "NOTES.md");

        // Re-import must not clobber an edited note file.
        fs::write(dir.join("NOTES.md"), "my hand-edited notes").unwrap();
        provision_files(&dir, "MyRepo", "#ffd966", "id-123", "2026-06-11T10:00:00").unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("NOTES.md")).unwrap(),
            "my hand-edited notes"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
