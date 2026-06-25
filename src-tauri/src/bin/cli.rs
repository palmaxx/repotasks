use clap::{Parser, Subcommand};
use std::env;
use std::path::{Path, PathBuf};
use repotasks_lib::commands;
use repotasks_lib::models::RepoConfig;

#[derive(Parser)]
#[command(name = "rtasks", version, about = "RepoTasks CLI helper")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import a project path
    Add {
        /// Absolute or relative path to the project
        path: String,
    },
    /// Add a note to a project
    Note {
        /// The note text
        text: String,
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
        /// Whether this is a todo (default is a note)
        #[arg(long, short)]
        todo: bool,
    },
    /// View notes for a project
    View {
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
    },
    /// Toggle a todo's done status by its line number
    Toggle {
        /// The line number of the todo (use `view` to see line numbers)
        line: usize,
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
    },
    /// Check git sync status
    Status {
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
    },
    /// Commit and push changes
    Push {
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
    },
    /// Pull latest changes
    Pull {
        /// Optional project folder name or path. If omitted, uses current directory.
        folder: Option<String>,
    },
    /// List all tracked projects
    List,
}

fn get_config_dir() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().expect("Could not determine config directory");
    base_dirs.config_dir().join("io.github.palmaxx.repotasks")
}

fn resolve_project_id(config_dir: &Path, folder: Option<&String>) -> Result<String, String> {
    if let Some(folder_str) = folder {
        let projects = commands::load_projects(config_dir)?;
        
        // 1. Try exact name match
        let matches_by_name: Vec<_> = projects.iter().filter(|p| p.name == *folder_str).collect();
        if matches_by_name.len() == 1 {
            return Ok(matches_by_name[0].id.clone());
        } else if matches_by_name.len() > 1 {
            return Err(format!("Multiple projects found with name '{}'. Please run from the directory or use exact path.", folder_str));
        }

        // 2. Try exact path match
        let abs_path = if Path::new(folder_str).is_absolute() {
            folder_str.clone()
        } else {
            env::current_dir().map_err(|e| e.to_string())?.join(folder_str).to_string_lossy().to_string()
        };

        if let Some(p) = projects.iter().find(|p| p.path == abs_path) {
            return Ok(p.id.clone());
        }

        Err(format!("No project found with name or path '{}'", folder_str))
    } else {
        // Try current directory
        let cwd = env::current_dir().map_err(|e| e.to_string())?;
        let config_path = cwd.join(".repotasks.json");
        if config_path.exists() {
            let data = std::fs::read_to_string(config_path).map_err(|e| e.to_string())?;
            let cfg: RepoConfig = serde_json::from_str(&data).map_err(|e| e.to_string())?;
            Ok(cfg.id)
        } else {
            Err("No .repotasks.json found in current directory. Please specify a folder name.".to_string())
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let config_dir = get_config_dir();

    match &cli.command {
        Commands::Add { path } => {
            let abs_path = if Path::new(path).is_absolute() {
                path.clone()
            } else {
                env::current_dir().unwrap().join(path).to_string_lossy().to_string()
            };
            match commands::import_project_core(&config_dir, &abs_path) {
                Ok(p) => println!("Added project: {} ({})", p.name, p.path),
                Err(e) => eprintln!("Error adding project: {}", e),
            }
        }
        Commands::Note { text, folder, todo } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    match commands::add_entry_core(&config_dir, &id, text, *todo) {
                        Ok(_) => println!("Successfully added entry."),
                        Err(e) => eprintln!("Error adding entry: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::View { folder } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    match commands::read_notes_core(&config_dir, &id) {
                        Ok(entries) => {
                            for entry in entries {
                                let marker = if entry.kind == repotasks_lib::models::EntryKind::Todo {
                                    if entry.done { "[x]" } else { "[ ]" }
                                } else {
                                    "-"
                                };
                                let time = entry.timestamp.unwrap_or_default();
                                println!("{:4}: {} {} {}", entry.line, marker, time, entry.text);
                            }
                        }
                        Err(e) => eprintln!("Error reading notes: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Toggle { line, folder } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    match commands::toggle_todo_core(&config_dir, &id, *line) {
                        Ok(_) => println!("Successfully toggled line {}.", line),
                        Err(e) => eprintln!("Error toggling todo: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Status { folder } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    match commands::check_git_sync_status_core(&config_dir, &id) {
                        Ok(status) => {
                            if !status.is_git {
                                println!("Not a git repository or NOTES.md is untracked.");
                            } else if !status.has_remote {
                                println!("No remote configured.");
                            } else {
                                println!("Ahead: {}, Behind: {}", status.ahead, status.behind);
                                if status.has_uncommitted_notes {
                                    println!("NOTES.md has unstaged/uncommitted changes.");
                                }
                            }
                        }
                        Err(e) => eprintln!("Error checking status: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Push { folder } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    println!("Committing and pushing...");
                    match commands::commit_and_push_core(&config_dir, &id) {
                        Ok(_) => println!("Successfully committed and pushed."),
                        Err(e) => eprintln!("Error pushing: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Pull { folder } => {
            match resolve_project_id(&config_dir, folder.as_ref()) {
                Ok(id) => {
                    println!("Pulling changes...");
                    match commands::pull_notes_core(&config_dir, &id) {
                        Ok(_) => println!("Successfully pulled changes."),
                        Err(e) => eprintln!("Error pulling: {}", e),
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::List => {
            match commands::load_projects(&config_dir) {
                Ok(projects) => {
                    if projects.is_empty() {
                        println!("No projects found.");
                    } else {
                        for p in projects {
                            let pin = if p.pinned { "[Pinned] " } else { "" };
                            println!("- {}{} ({})", pin, p.name, p.path);
                        }
                    }
                }
                Err(e) => eprintln!("Error loading projects: {}", e),
            }
        }
    }
}
