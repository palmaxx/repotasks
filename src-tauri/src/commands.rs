use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::models::{Entry, Project, RepoConfig};
use crate::notes;

/// Sticky-note palette; new projects cycle through it for visual variety.
const PALETTE: [&str; 6] = [
    "#ffd966", "#f6a5c0", "#a0d8b3", "#a5c8f6", "#d8b3f6", "#f6c8a0",
];

pub const NOTE_FILE: &str = "NOTES.md";

pub fn get_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

pub fn projects_file(config_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    Ok(config_dir.join("projects.json"))
}

pub fn load_projects(config_dir: &Path) -> Result<Vec<Project>, String> {
    let path = projects_file(config_dir)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save_projects(config_dir: &Path, projects: &[Project]) -> Result<(), String> {
    let path = projects_file(config_dir)?;
    let data = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

fn new_id() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
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
    load_projects(&get_config_dir(&app)?)
}

/// Register a folder as a project, provisioning its NOTES.md + .repotasks.json
/// in place. Idempotent: re-importing a known path returns the existing entry
/// and never overwrites files that already exist.
#[tauri::command]
pub fn import_project_core(config_dir: &Path, path: &str) -> Result<Project, String> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        return Err(format!("Not a folder: {path}"));
    }

    let mut projects = load_projects(config_dir)?;
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
    save_projects(config_dir, &projects)?;
    Ok(project)
}

#[tauri::command]
pub fn import_project(app: AppHandle, path: String) -> Result<Project, String> {
    import_project_core(&get_config_dir(&app)?, &path)
}

/// Forget a project from the central index. Files in the repo are left intact.
#[tauri::command]
pub fn remove_project(app: AppHandle, id: String) -> Result<(), String> {
    let config_dir = get_config_dir(&app)?;
    let mut projects = load_projects(&config_dir)?;
    let before = projects.len();
    projects.retain(|p| p.id != id);
    if projects.len() == before {
        return Err(format!("No project with id {id}"));
    }
    save_projects(&config_dir, &projects)
}

/// Append a note/todo under the project's `## Inbox`, stamped with local time.
pub fn add_entry_core(config_dir: &Path, project_id: &str, text: &str, is_todo: bool) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Cannot add an empty note".into());
    }

    let project = find_project(config_dir, project_id)?;

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

#[tauri::command]
pub fn add_entry(app: AppHandle, project_id: String, text: String, is_todo: bool) -> Result<(), String> {
    add_entry_core(&get_config_dir(&app)?, &project_id, &text, is_todo)
}

/// Parse a project's NOTES.md into structured notes/todos for the board.
pub fn read_notes_core(config_dir: &Path, project_id: &str) -> Result<Vec<Entry>, String> {
    let project = find_project(config_dir, project_id)?;

    let note_path = Path::new(&project.path).join(NOTE_FILE);
    if !note_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&note_path).map_err(|e| e.to_string())?;
    Ok(notes::parse_notes(&content))
}

#[tauri::command]
pub fn read_notes(app: AppHandle, project_id: String) -> Result<Vec<Entry>, String> {
    read_notes_core(&get_config_dir(&app)?, &project_id)
}

/// Pin or unpin a project (pinned cards sort first on the board).
#[tauri::command]
pub fn set_pinned(app: AppHandle, id: String, pinned: bool) -> Result<(), String> {
    let config_dir = get_config_dir(&app)?;
    let mut projects = load_projects(&config_dir)?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("No project with id {id}"))?;
    project.pinned = pinned;
    save_projects(&config_dir, &projects)
}

/// Resolve a project's NOTES.md, apply a pure rewrite, and persist it.
fn rewrite_notes(
    config_dir: &Path,
    project_id: &str,
    f: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let project = find_project(config_dir, project_id)?;
    let note_path = Path::new(&project.path).join(NOTE_FILE);
    let content = fs::read_to_string(&note_path).map_err(|e| e.to_string())?;
    let updated = f(&content)?;
    fs::write(&note_path, updated).map_err(|e| e.to_string())
}

pub fn find_project(config_dir: &Path, project_id: &str) -> Result<Project, String> {
    load_projects(config_dir)?
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("No project with id {project_id}"))
}

/// Flip a todo's checkbox at the given 0-based file line.
pub fn toggle_todo_core(config_dir: &Path, project_id: &str, line: usize) -> Result<(), String> {
    rewrite_notes(config_dir, project_id, |c| notes::toggle_todo_at(c, line))
}

#[tauri::command]
pub fn toggle_todo(app: AppHandle, project_id: String, line: usize) -> Result<(), String> {
    toggle_todo_core(&get_config_dir(&app)?, &project_id, line)
}

/// Replace the text of an entry at the given line (keeps marker + timestamp).
#[tauri::command]
pub fn update_entry(
    app: AppHandle,
    project_id: String,
    line: usize,
    text: String,
) -> Result<(), String> {
    rewrite_notes(&get_config_dir(&app)?, &project_id, |c| notes::update_text_at(c, line, &text))
}

/// Delete the entry at the given line.
#[tauri::command]
pub fn delete_entry(app: AppHandle, project_id: String, line: usize) -> Result<(), String> {
    rewrite_notes(&get_config_dir(&app)?, &project_id, |c| notes::delete_at(c, line))
}

/// Open the repo folder in the OS file manager.
#[tauri::command]
pub fn open_folder(app: AppHandle, project_id: String) -> Result<(), String> {
    let project = find_project(&get_config_dir(&app)?, &project_id)?;
    app.opener()
        .open_path(project.path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Open the project's NOTES.md with the OS default handler.
#[tauri::command]
pub fn open_in_editor(app: AppHandle, project_id: String) -> Result<(), String> {
    let project = find_project(&get_config_dir(&app)?, &project_id)?;
    let note_path = Path::new(&project.path).join(NOTE_FILE);
    app.opener()
        .open_path(note_path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn git_cmd() -> Command {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    cmd
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct GitSyncStatus {
    pub is_git: bool,
    pub has_remote: bool,
    pub ahead: usize,
    pub behind: usize,
    pub has_uncommitted_notes: bool,
}

pub fn check_git_sync_status_core(config_dir: &Path, project_id: &str) -> Result<GitSyncStatus, String> {
    let project = find_project(config_dir, project_id)?;
    let path = Path::new(&project.path);

    // 1. Check if it's a git repo
    let is_git = git_cmd()
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git {
        return Ok(GitSyncStatus {
            is_git: false,
            has_remote: false,
            ahead: 0,
            behind: 0,
            has_uncommitted_notes: false,
        });
    }

    // 2. Check if NOTES.md is actually tracked in Git.
    // If it's not tracked, we shouldn't trigger warnings about sync status.
    let is_tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", NOTE_FILE])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_tracked {
        return Ok(GitSyncStatus {
            is_git: false, // treat it as non-git for sync warning purposes
            has_remote: false,
            ahead: 0,
            behind: 0,
            has_uncommitted_notes: false,
        });
    }

    // 3. Check if it has a remote
    let has_remote = git_cmd()
        .args(["remote"])
        .current_dir(path)
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            !stdout.trim().is_empty()
        })
        .unwrap_or(false);

    if !has_remote {
        return Ok(GitSyncStatus {
            is_git: true,
            has_remote: false,
            ahead: 0,
            behind: 0,
            has_uncommitted_notes: false,
        });
    }

    // 4. Check for unstaged/uncommitted NOTES.md modifications
    let has_uncommitted_notes = git_cmd()
        .args(["status", "--porcelain", NOTE_FILE])
        .current_dir(path)
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            !stdout.trim().is_empty()
        })
        .unwrap_or(false);

    // 5. Fetch from remote in background to update remote tracking branch.
    // Disable all interactive prompts (terminal and GUI) so it doesn't steal focus.
    let _ = git_cmd()
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "false")
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "")
        .args(["fetch", "--quiet"])
        .current_dir(path)
        .output();

    // 6. Get ahead/behind count compared to upstream tracking branch
    let upstream = git_cmd()
        .args(["rev-parse", "--abbrev-ref", "@{u}"])
        .current_dir(path)
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.trim().to_string()
        })
        .unwrap_or_default();

    let mut ahead = 0;
    let mut behind = 0;

    if !upstream.is_empty() && !upstream.contains("@{u}") {
        if let Ok(o) = git_cmd()
            .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .current_dir(path)
            .output()
        {
            let s = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() == 2 {
                ahead = parts[0].parse().unwrap_or(0);
                behind = parts[1].parse().unwrap_or(0);
            }
        }
    }

    Ok(GitSyncStatus {
        is_git: true,
        has_remote,
        ahead,
        behind,
        has_uncommitted_notes,
    })
}

#[tauri::command]
pub async fn check_git_sync_status(app: AppHandle, project_id: String) -> Result<GitSyncStatus, String> {
    check_git_sync_status_core(&get_config_dir(&app)?, &project_id)
}

pub fn commit_and_push_core(config_dir: &Path, project_id: &str) -> Result<(), String> {
    let project = find_project(config_dir, project_id)?;
    let path = Path::new(&project.path);
    
    git_cmd()
        .args(["add", NOTE_FILE, ".repotasks.json"])
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;

    git_cmd()
        .args(["commit", "-m", "Repotasks update"])
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;

    let o = git_cmd()
        .args(["push"])
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        return Err(String::from_utf8_lossy(&o.stderr).to_string());
    }

    Ok(())
}

#[tauri::command]
pub fn commit_and_push(app: AppHandle, project_id: String) -> Result<(), String> {
    commit_and_push_core(&get_config_dir(&app)?, &project_id)
}

pub fn pull_notes_core(config_dir: &Path, project_id: &str) -> Result<(), String> {
    let project = find_project(config_dir, project_id)?;
    let path = Path::new(&project.path);

    let o = git_cmd()
        .args(["pull", "--rebase"])
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;

    if !o.status.success() {
        // Abort the rebase if it fails to prevent locking up the user's repository
        let _ = git_cmd().args(["rebase", "--abort"]).current_dir(path).output();
        return Err(String::from_utf8_lossy(&o.stderr).to_string());
    }

    Ok(())
}

#[tauri::command]
pub fn pull_notes(app: AppHandle, project_id: String) -> Result<(), String> {
    pull_notes_core(&get_config_dir(&app)?, &project_id)
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
