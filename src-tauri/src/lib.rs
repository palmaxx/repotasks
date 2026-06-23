pub mod commands;
pub mod models;
mod notes;

use tauri::{
    menu::{Menu, MenuItem, Submenu, IsMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIcon},
    Manager, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

// Removed toggle_capture

/// Bring the main board window to the foreground.
fn show_main(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // ---- System tray ------------------------------------------------
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("RepoTasks")
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "board" {
                        show_main(app);
                    } else if event.id.as_ref() == "quit" {
                        app.exit(0);
                    } else {
                        // handle dynamic menu events
                        let id = event.id.as_ref();
                        if let Some(rest) = id.strip_prefix("folder:") {
                            if let Ok(config_dir) = commands::get_config_dir(app) {
                                if let Ok(project) = commands::find_project(&config_dir, rest) {
                                    let _ = app.opener().open_path(project.path, None::<&str>);
                                }
                            }
                        } else if let Some(rest) = id.strip_prefix("editor:") {
                            if let Ok(config_dir) = commands::get_config_dir(app) {
                                if let Ok(project) = commands::find_project(&config_dir, rest) {
                                    let note_path = std::path::Path::new(&project.path).join(commands::NOTE_FILE);
                                    let _ = app.opener().open_path(note_path.to_string_lossy().to_string(), None::<&str>);
                                }
                            }
                        } else if let Some(rest) = id.strip_prefix("pull:") {
                            if let Ok(config_dir) = commands::get_config_dir(app) {
                                let _ = commands::pull_notes_core(&config_dir, rest);
                                // The background task will update the tray on next tick
                            }
                        }
                    }
                })
                .on_tray_icon_event(|tray: &TrayIcon, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            let _ = rebuild_tray_menu(app.handle());

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(300));

                    if let Ok(config_dir) = commands::get_config_dir(&app_handle) {
                        if let Ok(projects) = commands::load_projects(&config_dir) {
                            let mut unpulled_count = 0;
                            for p in projects {
                                if let Ok(status) = commands::check_git_sync_status_core(&config_dir, &p.id) {
                                    if status.behind > 0 {
                                        unpulled_count += 1;
                                    }
                                }
                            }
                            
                            if unpulled_count > 0 {
                                let body = if unpulled_count == 1 {
                                    "1 project has unpulled remote changes.".to_string()
                                } else {
                                    format!("{} projects have unpulled remote changes.", unpulled_count)
                                };
                                
                                let _ = app_handle.notification()
                                    .builder()
                                    .title("RepoTasks Sync")
                                    .body(&body)
                                    .show();
                            }
                        }
                    }
                    let _ = rebuild_tray_menu(&app_handle);
                }
            });

            // ---- Keep the app alive in the tray when windows are "closed" ----
            if let Some(main) = app.get_webview_window("main") {
                let m = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let _ = m.hide();
                        api.prevent_close();
                    }
                });
            }
// Removed capture window logic

            // ---- Global shortcut: Ctrl+Alt+Space toggles quick-add ----------
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };
                let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
                let for_handler = toggle;
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, scut, event| {
                            if scut == &for_handler && event.state() == ShortcutState::Pressed {
                                show_main(app);
                            }
                        })
                        .build(),
                )?;
                if let Err(e) = app.global_shortcut().register(toggle) {
                    eprintln!("RepoTasks: could not register global shortcut: {e}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::import_project,
            commands::remove_project,
            commands::add_entry,
            commands::read_notes,
            commands::set_pinned,
            commands::toggle_todo,
            commands::update_entry,
            commands::delete_entry,
            commands::open_folder,
            commands::open_in_editor,
            commands::check_git_sync_status,
            commands::commit_and_push,
            commands::pull_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn rebuild_tray_menu(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let board = MenuItem::with_id(app, "board", "Open Board", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let mut items: Vec<&dyn IsMenuItem<_>> = vec![&board];
    let mut project_submenus = Vec::new();

    if let Ok(config_dir) = commands::get_config_dir(app) {
        if let Ok(projects) = commands::load_projects(&config_dir) {
            for p in projects {
                let status = commands::check_git_sync_status_core(&config_dir, &p.id).unwrap_or(commands::GitSyncStatus {
                    is_git: false,
                    has_remote: false,
                    ahead: 0,
                    behind: 0,
                    has_uncommitted_notes: false,
                });
                
                let mut label = p.name.clone();
                if status.is_git {
                    if status.behind > 0 {
                        label.push_str(&format!(" ({}↓)", status.behind));
                    } else if status.ahead > 0 {
                        label.push_str(&format!(" ({}↑)", status.ahead));
                    } else if status.has_uncommitted_notes {
                        label.push_str(" (*)");
                    }
                }

                let folder_item = MenuItem::with_id(app, format!("folder:{}", p.id), "Open Folder", true, None::<&str>)?;
                let editor_item = MenuItem::with_id(app, format!("editor:{}", p.id), "Edit NOTES.md", true, None::<&str>)?;
                let pull_item = MenuItem::with_id(app, format!("pull:{}", p.id), "Pull Notes", true, None::<&str>)?;
                
                let submenu = Submenu::with_items(
                    app,
                    label,
                    true,
                    &[&folder_item, &editor_item, &pull_item],
                )?;
                project_submenus.push(submenu);
            }
        }
    }

    for sub in &project_submenus {
        items.push(sub);
    }
    items.push(&quit);

    let menu = Menu::with_items(app, &items)?;
    
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }

    Ok(())
}
