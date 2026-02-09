mod commands;
mod helper;
mod tray;
mod hook_server;

use commands::*;
use hook_server::start_hook_server;

fn configure_macos_window<R: tauri::Runtime>(app: &tauri::App<R>) {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
        }
    }
}

fn build_app_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    use tauri::menu::{MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

    let app_name = app.package_info().name.clone();
    let separator = PredefinedMenuItem::separator(app)?;

    SubmenuBuilder::new(app, &app_name)
        .item(&PredefinedMenuItem::about(app, Some(&app_name), None)?)
        .item(&separator)
        .item(&PredefinedMenuItem::services(app, None)?)
        .item(&separator)
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .item(&separator)
        .item(
            &MenuItemBuilder::with_id("quit", format!("Quit {}", app_name))
                .accelerator("CmdOrCtrl+Q")
                .build(app)?,
        )
        .build()
}

fn build_file_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
    open_config_item: &tauri::menu::MenuItem<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    use tauri::menu::{PredefinedMenuItem, SubmenuBuilder};

    let separator = PredefinedMenuItem::separator(app)?;

    SubmenuBuilder::new(app, "File")
        .item(open_config_item)
        .item(&separator)
        .item(&PredefinedMenuItem::close_window(app, None)?)
        .build()
}

fn build_edit_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    use tauri::menu::{PredefinedMenuItem, SubmenuBuilder};

    let separator = PredefinedMenuItem::separator(app)?;

    SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .item(&separator)
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&separator)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .build()
}

fn build_window_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
    minimize_item: &tauri::menu::MenuItem<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    use tauri::menu::{PredefinedMenuItem, SubmenuBuilder};

    let separator = PredefinedMenuItem::separator(app)?;

    SubmenuBuilder::new(app, "Window")
        .item(minimize_item)
        .item(&PredefinedMenuItem::minimize(app, None)?)
        .item(&separator)
        .item(&PredefinedMenuItem::fullscreen(app, None)?)
        .build()
}

fn build_help_menu<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    use tauri::menu::SubmenuBuilder;

    SubmenuBuilder::new(app, "Help").build()
}

fn spawn_initialize_app_config_task() {
    println!("Setting up app...");
    tauri::async_runtime::spawn(async move {
        println!("Initializing app config...");
        match commands::initialize_app_config().await {
            Ok(()) => println!("App config initialized successfully"),
            Err(e) => eprintln!("Failed to initialize app config: {}", e),
        }
    });
}

fn spawn_update_claude_hooks_task() {
    tauri::async_runtime::spawn(async move {
        println!("Updating Claude Code hooks to latest version...");
        match commands::update_claude_code_hook().await {
            Ok(()) => println!("✅ Claude Code hooks updated/checked successfully"),
            Err(e) => eprintln!("Failed to update Claude Code hooks: {}", e),
        }
    });
}

fn spawn_hook_server_task(app_handle: tauri::AppHandle) {
    println!("Starting hook server...");
    tauri::async_runtime::spawn(async move {
        match start_hook_server(app_handle).await {
            Ok(()) => println!("Hook server started successfully"),
            Err(e) => eprintln!("Failed to start hook server: {}", e),
        }
    });
}

fn handle_app_menu_event<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    event_id: &str,
) {
    use tauri::Manager;

    match event_id {
        "open_config_path" => {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::open_config_path().await {
                    eprintln!("Failed to open config path: {}", e);
                }
            });
        }
        "minimize_window" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        "quit" => {
            app_handle.exit(0);
        }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            configure_macos_window(app);

            // Create application menu
            use tauri::menu::{MenuBuilder, MenuItemBuilder};

            let open_config_item = MenuItemBuilder::with_id("open_config_path", "Open config path")
                .accelerator("CmdOrCtrl+Shift+O")
                .build(app)?;

            // Custom minimize item for Cmd+W
            let minimize_item = MenuItemBuilder::with_id("minimize_window", "Minimize")
                .accelerator("Cmd+W")
                .build(app)?;

            let app_menu = build_app_menu(app)?;
            let file_menu = build_file_menu(app, &open_config_item)?;
            let edit_menu = build_edit_menu(app)?;
            let window_menu = build_window_menu(app, &minimize_item)?;
            let help_menu = build_help_menu(app)?;

            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&window_menu)
                .item(&help_menu)
                .build()?;

            app.set_menu(menu)?;

            // Initialize system tray
            if let Err(e) = tray::create_tray(&app.handle()) {
                eprintln!("Failed to create system tray: {}", e);
            }

            // Handle menu events (both app menu and tray menu)
            app.on_menu_event(|app_handle, event| {
                let event_id = event.id().0.as_str();

                // Try to handle as tray menu event first
                if tray::handle_tray_menu_event(&app_handle, event_id) {
                    return;
                }

                handle_app_menu_event(&app_handle, event_id);
            });

            spawn_initialize_app_config_task();
            spawn_update_claude_hooks_task();
            spawn_hook_server_task(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_config_file,
            write_config_file,
            list_config_files,
            check_app_config_exists,
            create_app_config_dir,
            backup_claude_configs,
            get_stores,
            get_store,
            create_config,
            update_config,
            delete_config,
            set_using_config,
            reset_to_original_config,
            get_current_store,
            open_config_path,
            get_global_mcp_servers,
            update_global_mcp_server,
            delete_global_mcp_server,
            check_mcp_server_exists,
            get_mcp_enabled_state,
            toggle_mcp_server_state,
            toggle_direct_mcp_server,
            get_mcp_servers_with_state,
            read_claude_projects,
            read_claude_config_file,
            write_claude_config_file,
            check_for_updates,
            install_and_restart,
            rebuild_tray_menu_command,
            unlock_cc_ext,
            read_project_usage_files,
            read_claude_memory,
            write_claude_memory,
            list_claude_memory_files,
            write_claude_memory_file,
            toggle_claude_memory_file,
            delete_claude_memory_file,
            track,
            get_notification_settings,
            update_notification_settings,
            add_claude_code_hook,
            update_claude_code_hook,
            remove_claude_code_hook,
            read_claude_commands,
            write_claude_command,
            delete_claude_command,
            toggle_claude_command,
            read_claude_agents,
            write_claude_agent,
            delete_claude_agent,
            toggle_claude_agent,
            read_installed_plugins,
            toggle_plugin,
            read_plugin_commands,
            read_plugin_agents,
            list_claude_skills,
            read_known_marketplaces,
            toggle_claude_skill,
            write_claude_skill,
            delete_claude_skill,
            get_hooks_settings,
            get_security_templates,
            get_installed_security_templates,
            install_security_template,
            uninstall_security_template
        ])
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent the window from closing and hide it instead
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .on_page_load(|window, _| {
            #[cfg(target_os = "macos")]
            {
                // Ensure window is shown when page loads
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            use tauri::Manager;
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                // Handle dock icon click - show and focus the main window
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}
