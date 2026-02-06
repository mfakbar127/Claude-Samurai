use serde_json::Value;
use std::path::PathBuf;
use tauri_plugin_updater::UpdaterExt;
use reqwest;
use uuid::Uuid;
use nanoid;

use crate::helper::{
    ensure_dir, extract_string_array, get_project_path_from_claude_json, home_dir,
    path_to_string, read_direct_servers, read_disabled_mcp_servers_from_claude_json,
    read_json_file, read_local_mcp_servers, read_mcpjson_servers, read_project_mcp_servers,
    write_json_file, write_json_file_serialize,
};

// Application configuration directory
const APP_CONFIG_DIR: &str = ".ccconfig";

pub async fn initialize_app_config() -> Result<(), String> {
    println!("initialize_app_config called");

    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);

    println!(
        "Checking if app config directory exists: {}",
        app_config_path.display()
    );

    // Create config directory if it doesn't exist
    if !app_config_path.exists() {
        println!("App config directory does not exist, creating...");
        ensure_dir(&app_config_path, "app config directory")?;
        println!(
            "App config directory created: {}",
            app_config_path.display()
        );
    } else {
        println!("App config directory already exists");
    }

    // Check if we need to backup Claude configs
    let claude_dir = home_dir.join(".claude");
    println!(
        "Checking if Claude directory exists: {}",
        claude_dir.display()
    );

    if claude_dir.exists() {
        // Check if we already have a backup
        let backup_dir = app_config_path.join("claude_backup");
        if backup_dir.exists() {
            println!("Claude backup already exists, skipping backup");
        } else {
            println!("Claude directory exists but no backup found, backing up...");
            if let Err(e) = backup_claude_configs_internal(&app_config_path, &claude_dir) {
                return Err(format!("Failed to backup Claude configs: {}", e));
            }
            println!("Claude configs backed up successfully");
        }
    } else {
        println!("Claude directory does not exist, skipping backup");
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ConfigFile {
    pub path: String,
    pub content: Value,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ConfigStore {
    pub id: String,
    pub title: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    pub settings: Value,
    pub using: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct McpServer {
    #[serde(flatten)]
    pub config: serde_json::Value,
    
    // Metadata fields
    #[serde(rename = "sourceType")]
    pub source_type: String,  // "mcpjson" | "direct"
    
    pub scope: String,  // "user" for now (will add "local", "project" later)
    
    #[serde(rename = "definedIn")]
    pub defined_in: String,  // File path where server is defined
    
    pub controllable: bool,  // true for mcpjson, false for direct
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct McpServerState {
    pub name: String,
    pub config: serde_json::Value,
    
    // Metadata
    #[serde(rename = "sourceType")]
    pub source_type: String,
    pub scope: String,
    #[serde(rename = "definedIn")]
    pub defined_in: String,
    pub controllable: bool,
    
    // State information
    pub state: String,  // "disabled" | "enabled" | "runtime-disabled"
    #[serde(rename = "inEnabledArray")]
    pub in_enabled_array: bool,
    #[serde(rename = "inDisabledArray")]
    pub in_disabled_array: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
#[serde(default)]
pub struct StoresData {
    pub configs: Vec<ConfigStore>,
    pub distinct_id: Option<String>,
    pub notification: Option<NotificationSettings>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NotificationSettings {
    pub enable: bool,
    pub enabled_hooks: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HooksConfigEntry {
    pub source: String, // "project_local" | "project" | "user"
    pub path: String,
    pub exists: bool,
    pub hooks: Option<Value>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn read_config_file(config_type: String) -> Result<ConfigFile, String> {
    let home_dir = home_dir()?;

    let path = match config_type.as_str() {
        "user" => home_dir.join(".claude/settings.json"),
        "enterprise_macos" => {
            PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json")
        }
        "enterprise_linux" => PathBuf::from("/etc/claude-code/managed-settings.json"),
        "enterprise_windows" => PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-settings.json"),
        "mcp_macos" => PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.json"),
        "mcp_linux" => PathBuf::from("/etc/claude-code/managed-mcp.json"),
        "mcp_windows" => PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-mcp.json"),
        _ => return Err("Invalid configuration type".to_string()),
    };

    let path_str = path_to_string(&path);
    let content = read_json_file(&path, "config file")?;
    Ok(ConfigFile {
        path: path_str,
        content,
        exists: path.exists(),
    })
}

#[tauri::command]
pub async fn write_config_file(config_type: String, content: Value) -> Result<(), String> {
    let home_dir = home_dir()?;

    let path = match config_type.as_str() {
        "user" => home_dir.join(".claude/settings.json"),
        _ => return Err("Cannot write to enterprise configuration files".to_string()),
    };

    write_json_file(&path, &content, "config file")?;
    Ok(())
}

#[tauri::command]
pub async fn list_config_files() -> Result<Vec<String>, String> {
    let mut configs = vec![];

    // User settings
    if let Some(home) = dirs::home_dir() {
        let user_settings = home.join(".claude/settings.json");
        if user_settings.exists() {
            configs.push("user".to_string());
        }
    }

    // Enterprise settings (read-only)
    if cfg!(target_os = "macos") {
        let enterprise_path =
            PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_macos".to_string());
        }

        let mcp_path = PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_macos".to_string());
        }
    } else if cfg!(target_os = "linux") {
        let enterprise_path = PathBuf::from("/etc/claude-code/managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_linux".to_string());
        }

        let mcp_path = PathBuf::from("/etc/claude-code/managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_linux".to_string());
        }
    } else if cfg!(target_os = "windows") {
        let enterprise_path = PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-settings.json");
        if enterprise_path.exists() {
            configs.push("enterprise_windows".to_string());
        }

        let mcp_path = PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-mcp.json");
        if mcp_path.exists() {
            configs.push("mcp_windows".to_string());
        }
    }

    Ok(configs)
}

#[tauri::command]
pub async fn check_app_config_exists() -> Result<bool, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    Ok(app_config_path.exists())
}

#[tauri::command]
pub async fn create_app_config_dir() -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);

    ensure_dir(&app_config_path, "app config directory")?;

    Ok(())
}

#[tauri::command]
pub async fn get_hooks_settings(_cwd: Option<String>) -> Result<Vec<HooksConfigEntry>, String> {
    let home = home_dir()?;
    let mut entries: Vec<HooksConfigEntry> = Vec::new();

    // Helper to build one entry; only returns Some when file exists and has a hooks key
    fn build_entry(source: &str, path: std::path::PathBuf) -> Option<HooksConfigEntry> {
        let path_str = path_to_string(&path);
        if !path.exists() {
            return None;
        }

        match read_json_file(&path, "settings file") {
            Ok(value) => {
                let hooks = value.get("hooks").cloned();
                if hooks.is_none() {
                    return None;
                }

                Some(HooksConfigEntry {
                    source: source.to_string(),
                    path: path_str,
                    exists: true,
                    hooks,
                    error: None,
                })
            }
            Err(e) => Some(HooksConfigEntry {
                source: source.to_string(),
                path: path_str,
                exists: true,
                hooks: None,
                error: Some(e),
            }),
        }
    }

    // Discover all known projects from ~/.claude.json and collect their hooks
    let claude_json_path = home.join(".claude.json");
    if claude_json_path.exists() {
        let claude_value = read_json_file(&claude_json_path, ".claude.json")?;

        if let Some(projects) = claude_value.get("projects").and_then(|p| p.as_object()) {
            for (project_path_str, _) in projects {
                let project_path = std::path::PathBuf::from(project_path_str);

                let local_settings_path = project_path.join(".claude/settings.local.json");
                if let Some(entry) = build_entry("project_local", local_settings_path) {
                    entries.push(entry);
                }

                let project_settings_path = project_path.join(".claude/settings.json");
                if let Some(entry) = build_entry("project", project_settings_path) {
                    entries.push(entry);
                }
            }
        }
    }

    // User/global settings (included if it has hooks)
    let user_settings_path = home.join(".claude/settings.json");
    if let Some(entry) = build_entry("user", user_settings_path) {
        entries.push(entry);
    }

    Ok(entries)
}

fn backup_claude_configs_internal(
    app_config_path: &std::path::Path,
    claude_dir: &std::path::Path,
) -> Result<(), String> {
    // Create backup directory
    let backup_dir = app_config_path.join("claude_backup");

    ensure_dir(&backup_dir, "backup directory")?;

    // Copy all files from .claude directory to backup
    for entry in std::fs::read_dir(claude_dir)
        .map_err(|e| format!("Failed to read Claude directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_name = source_path.file_name().ok_or("Invalid file name")?;
        let dest_path = backup_dir.join(file_name);

        if source_path.is_file() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("Failed to copy file {}: {}", source_path.display(), e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn backup_claude_configs() -> Result<(), String> {
    let home_dir = home_dir()?;
    let claude_dir = home_dir.join(".claude");
    let app_config_path = home_dir.join(APP_CONFIG_DIR);

    if !claude_dir.exists() {
        return Err("Claude configuration directory does not exist".to_string());
    }

    // Ensure app config directory exists
    ensure_dir(&app_config_path, "app config directory")?;

    backup_claude_configs_internal(&app_config_path, &claude_dir)
}

// Store management functions

#[tauri::command]
pub async fn get_stores() -> Result<Vec<ConfigStore>, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    let mut stores_data = read_stores_file(&stores_file)?;

    // Add default notification settings if they don't exist
    if stores_data.notification.is_none() {
        stores_data.notification = Some(NotificationSettings {
            enable: true,
            enabled_hooks: vec!["Notification".to_string()],
        });

        // Write back to stores file with notification settings added
        write_json_file_serialize(&stores_file, &stores_data, "stores file")?;
        println!("Added default notification settings to existing stores.json");
    }

    let mut stores_vec = stores_data.configs;
    // Sort by createdAt in ascending order (oldest first)
    stores_vec.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    Ok(stores_vec)
}

#[tauri::command]
pub async fn create_config(
    id: String,
    title: String,
    settings: Value,
) -> Result<ConfigStore, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    // Ensure app config directory exists
    ensure_dir(&app_config_path, "app config directory")?;

    // Read existing stores
    let mut stores_data = read_stores_file(&stores_file)?;
    if stores_data.notification.is_none() {
        stores_data.notification = Some(NotificationSettings {
            enable: true,
            enabled_hooks: vec!["Notification".to_string()],
        });
    }

    // Determine if this should be the active store (true if no other stores exist)
    let should_be_active = stores_data.configs.is_empty();

    // If this is the first config being created and there's an existing settings.json, create an Original Config store
    if should_be_active {
        let claude_settings_path = home_dir.join(".claude/settings.json");
        if claude_settings_path.exists() {
            // Read existing settings
            let settings_json = read_json_file(&claude_settings_path, "Claude settings")?;

            // Create an Original Config store with existing settings
            let original_store = ConfigStore {
                id: nanoid::nanoid!(6), // Generate a 6-character ID
                title: "Original Config".to_string(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| format!("Failed to get timestamp: {}", e))?
                    .as_secs(),
                settings: settings_json,
                using: false, // Original Config should not be active by default
            };

            // Add the Original Config store to the collection
            stores_data.configs.push(original_store);
            println!("Created Original Config store from existing settings.json");
        }
    }

    // If this is the first store (and therefore active), write its settings to the user's actual settings.json with partial update
    if should_be_active {
        let user_settings_path = home_dir.join(".claude/settings.json");

        // Create .claude directory if it doesn't exist
        if let Some(parent) = user_settings_path.parent() {
            ensure_dir(parent, ".claude directory")?;
        }

        // Read existing settings if file exists, otherwise start with empty object
        let mut existing_settings = read_json_file(&user_settings_path, "settings")?;

        // Merge the new settings into existing settings (partial update)
        if let Some(settings_obj) = settings.as_object() {
            if let Some(existing_obj) = existing_settings.as_object_mut() {
                // Update only the keys present in the stored settings
                for (key, value) in settings_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            } else {
                // If existing settings is not an object, replace it entirely
                existing_settings = settings.clone();
            }
        } else {
            // If stored settings is not an object, replace existing entirely
            existing_settings = settings.clone();
        }

        // Write the merged settings back to file
        write_json_file(&user_settings_path, &existing_settings, "user settings")?;
    }

    // Create new store
    let new_store = ConfigStore {
        id: id.clone(),
        title: title.clone(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?
            .as_secs(),
        settings,
        using: should_be_active,
    };

    // Add store to collection
    stores_data.configs.push(new_store.clone());

    // Write back to stores file
    write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

    // Automatically unlock CC extension when creating new config
    if let Err(e) = unlock_cc_ext().await {
        eprintln!("Warning: Failed to unlock CC extension: {}", e);
    }

    Ok(new_store)
}

#[tauri::command]
pub async fn delete_config(store_id: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    if !stores_file.exists() {
        return Err("Stores file does not exist".to_string());
    }

    // Read existing stores
    let mut stores_data = read_stores_file(&stores_file)?;

    // Find and remove store by ID
    let original_len = stores_data.configs.len();
    stores_data.configs.retain(|store| store.id != store_id);

    if stores_data.configs.len() == original_len {
        return Err("Store not found".to_string());
    }

    // Write back to file
    write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

    Ok(())
}

#[tauri::command]
pub async fn set_using_config(store_id: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    if !stores_file.exists() {
        return Err("Stores file does not exist".to_string());
    }

    // Read existing stores
    let mut stores_data = read_stores_file(&stores_file)?;

    // Find the store and check if it exists
    let store_found = stores_data.configs.iter().any(|store| store.id == store_id);
    if !store_found {
        return Err("Store not found".to_string());
    }

    // Set all stores to not using, then set the selected one to using
    let mut selected_store_settings: Option<Value> = None;
    for store in &mut stores_data.configs {
        if store.id == store_id {
            store.using = true;
            selected_store_settings = Some(store.settings.clone());
        } else {
            store.using = false;
        }
    }

    // Write the selected store's settings to the user's actual settings.json with partial update
    if let Some(settings) = selected_store_settings {
        let user_settings_path = home_dir.join(".claude/settings.json");

        // Create .claude directory if it doesn't exist
        if let Some(parent) = user_settings_path.parent() {
            ensure_dir(parent, ".claude directory")?;
        }

        // Read existing settings if file exists, otherwise start with empty object
        let mut existing_settings = read_json_file(&user_settings_path, "settings")?;

        // Merge the new settings into existing settings (partial update)
        if let Some(settings_obj) = settings.as_object() {
            if let Some(existing_obj) = existing_settings.as_object_mut() {
                // Update only the keys present in the stored settings
                for (key, value) in settings_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            } else {
                // If existing settings is not an object, replace it entirely
                existing_settings = settings.clone();
            }
        } else {
            // If stored settings is not an object, replace existing entirely
            existing_settings = settings.clone();
        }

        // Write the merged settings back to file
        write_json_file(&user_settings_path, &existing_settings, "user settings")?;
    }

    // Write back to stores file
    write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

    Ok(())
}

#[tauri::command]
pub async fn reset_to_original_config() -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    // Set all stores to not using
    if stores_file.exists() {
        let mut stores_data = read_stores_file(&stores_file)?;

        // Set all stores to not using
        for store in &mut stores_data.configs {
            store.using = false;
        }

        // Write back to stores file
        write_json_file_serialize(&stores_file, &stores_data, "stores file")?;
    }

    // Clear env field in settings.json
    let user_settings_path = home_dir.join(".claude/settings.json");

    // Create .claude directory if it doesn't exist
    if let Some(parent) = user_settings_path.parent() {
        ensure_dir(parent, ".claude directory")?;
    }

    // Read existing settings if file exists, otherwise start with empty object
    let mut existing_settings = read_json_file(&user_settings_path, "settings")?;

    // Set env to empty object
    if let Some(existing_obj) = existing_settings.as_object_mut() {
        existing_obj.insert("env".to_string(), serde_json::json!({}));
    }

    // Write the merged settings back to file
    write_json_file(&user_settings_path, &existing_settings, "user settings")?;

    Ok(())
}

#[tauri::command]
pub async fn get_current_store() -> Result<Option<ConfigStore>, String> {
    let stores = get_stores().await?;
    Ok(stores.into_iter().find(|store| store.using))
}

#[tauri::command]
pub async fn get_store(store_id: String) -> Result<ConfigStore, String> {
    let stores = get_stores().await?;
    stores
        .into_iter()
        .find(|store| store.id == store_id)
        .ok_or_else(|| format!("Store with id '{}' not found", store_id))
}

#[tauri::command]
pub async fn update_config(
    store_id: String,
    title: String,
    settings: Value,
) -> Result<ConfigStore, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    if !stores_file.exists() {
        return Err("Stores file does not exist".to_string());
    }

    // Read existing stores
    let mut stores_data = read_stores_file(&stores_file)?;

    // Find the store by ID
    let store_index = stores_data
        .configs
        .iter()
        .position(|store| store.id == store_id)
        .ok_or_else(|| format!("Store with id '{}' not found", store_id))?;

    // // Check if new title conflicts with existing stores (excluding current one)
    // for existing_store in &stores_data.configs {
    //     if existing_store.id != store_id && existing_store.title == title {
    //         return Err("Store with this title already exists".to_string());
    //     }
    // }

    // Update the store
    let store = &mut stores_data.configs[store_index];
    store.title = title.clone();
    store.settings = settings.clone();

    // If this store is currently in use, also update the user's settings.json with partial update
    if store.using {
        let user_settings_path = home_dir.join(".claude/settings.json");

        // Create .claude directory if it doesn't exist
        if let Some(parent) = user_settings_path.parent() {
            ensure_dir(parent, ".claude directory")?;
        }

        // Read existing settings if file exists, otherwise start with empty object
        let mut existing_settings = read_json_file(&user_settings_path, "settings")?;

        // Merge the new settings into existing settings (partial update)
        if let Some(settings_obj) = settings.as_object() {
            if let Some(existing_obj) = existing_settings.as_object_mut() {
                // Update only the keys present in the stored settings
                for (key, value) in settings_obj {
                    existing_obj.insert(key.clone(), value.clone());
                }
            } else {
                // If existing settings is not an object, replace it entirely
                existing_settings = settings.clone();
            }
        } else {
            // If stored settings is not an object, replace existing entirely
            existing_settings = settings.clone();
        }

        // Write the merged settings back to file
        write_json_file(&user_settings_path, &existing_settings, "user settings")?;
    }

    // Write back to stores file
    write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

    // Automatically unlock CC extension when updating config
    if let Err(e) = unlock_cc_ext().await {
        eprintln!("Warning: Failed to unlock CC extension: {}", e);
    }

    Ok(stores_data.configs[store_index].clone())
}

#[tauri::command]
pub async fn open_config_path() -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);

    // Ensure the directory exists
    if !app_config_path.exists() {
        ensure_dir(&app_config_path, "config directory")?;
    }

    // Open the directory in the system's file manager
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&app_config_path)
            .spawn()
            .map_err(|e| format!("Failed to open config directory: {}", e))?;
    }

    Ok(())
}

// MCP Server management functions

// Helper: Read and parse stores file (returns default when file missing)
fn read_stores_file(path: &std::path::Path) -> Result<StoresData, String> {
    let value = read_json_file(path, "stores file")?;
    serde_json::from_value(value).map_err(|e| format!("Failed to parse stores file: {}", e))
}

// Helper: Write serializable value as JSON file
// Helper: Get settings file path based on cwd and preference
fn get_settings_path(cwd: Option<&str>, prefer_local: bool) -> Result<PathBuf, String> {
    let home_dir = home_dir()?;
    
    if let Some(cwd_str) = cwd {
        if let Ok(Some(project_path)) = get_project_path_from_claude_json(cwd_str) {
            if prefer_local {
                // Write to ./.claude/settings.local.json (highest priority, gitignored)
                return Ok(project_path.join(".claude/settings.local.json"));
            } else {
                // Read from project settings (check local first, then project)
                let local_settings_path = project_path.join(".claude/settings.local.json");
                if local_settings_path.exists() {
                    return Ok(local_settings_path);
                }
                let project_settings_path = project_path.join(".claude/settings.json");
                if project_settings_path.exists() {
                    return Ok(project_settings_path);
                }
            }
        }
    }
    
    // Fallback to user-global settings
    Ok(home_dir.join(".claude/settings.json"))
}

// Helper: Create McpServer struct
fn create_mcp_server(
    config: Value,
    source_type: &str,
    scope: &str,
    defined_in: String,
    controllable: bool,
) -> McpServer {
    McpServer {
        config,
        source_type: source_type.to_string(),
        scope: scope.to_string(),
        defined_in,
        controllable,
    }
}

// Helper: Check if plugin install should be included based on scope and cwd
fn should_include_install(install: &PluginInstallInfo, cwd: Option<&str>) -> bool {
    match cwd {
        Some(cwd_str) if !cwd_str.is_empty() => {
            install.scope == "user"
                || (install.scope == "local" && install.project_path.as_deref() == Some(cwd_str))
        }
        _ => true, // Include all when cwd is None or empty (Global view)
    }
}

#[tauri::command]
pub async fn get_global_mcp_servers() -> Result<std::collections::HashMap<String, McpServer>, String> {
    let home_dir = home_dir()?;
    let mut result = std::collections::HashMap::new();
    
    // 1. Read from ~/.mcp.json (MCPJSON servers - user scope)
    if let Ok(mcpjson_servers) = read_mcpjson_servers(&home_dir) {
        for (name, config) in mcpjson_servers {
            result.insert(
                name.clone(),
                create_mcp_server(
                    config,
                    "mcpjson",
                    "user",
                    path_to_string(&home_dir.join(".mcp.json")),
                    true,
                ),
            );
        }
    }
    
    // 2. Read from ~/.claude.json (Direct servers - user scope)
    if let Ok(direct_servers) = read_direct_servers(&home_dir) {
        for (name, config) in direct_servers {
            // Don't override if already exists from .mcp.json
            result.entry(name.clone()).or_insert_with(|| {
                create_mcp_server(
                    config,
                    "direct",
                    "user",
                    path_to_string(&home_dir.join(".claude.json")),
                    false,
                )
            });
        }
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn check_mcp_server_exists(server_name: String) -> Result<bool, String> {
    let mcp_servers = get_global_mcp_servers().await?;
    Ok(mcp_servers.contains_key(&server_name))
}

#[tauri::command]
pub async fn update_global_mcp_server(
    server_name: String,
    server_config: Value,
) -> Result<(), String> {
    let home_dir = home_dir()?;
    let mcp_json_path = home_dir.join(".mcp.json");

    // Read existing .mcp.json or create new structure
    let mut json_value = read_json_file(&mcp_json_path, ".mcp.json")?;

    // Update mcpServers object (same structure as .claude.json)
    let mcp_servers = json_value
        .as_object_mut()
        .unwrap()
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    // Update the specific server
    mcp_servers.insert(server_name, server_config);

    // Write back to file
    write_json_file(&mcp_json_path, &json_value, ".mcp.json")?;

    Ok(())
}

#[tauri::command]
pub async fn delete_global_mcp_server(server_name: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let mcp_json_path = home_dir.join(".mcp.json");

    if !mcp_json_path.exists() {
        return Err("MCP configuration file does not exist".to_string());
    }

    // Read existing .mcp.json
    let mut json_value = read_json_file(&mcp_json_path, ".mcp.json")?;

    // Check if mcpServers exists
    let mcp_servers = json_value
        .as_object_mut()
        .unwrap()
        .get_mut("mcpServers")
        .and_then(|servers| servers.as_object_mut())
        .ok_or("No mcpServers found in .mcp.json")?;

    // Check if the server exists
    if !mcp_servers.contains_key(&server_name) {
        return Err(format!("MCP server '{}' not found", server_name));
    }

    // Remove the server
    mcp_servers.remove(&server_name);

    // If mcpServers is now empty, we can optionally remove the entire mcpServers object
    if mcp_servers.is_empty() {
        json_value.as_object_mut().unwrap().remove("mcpServers");
    }

    // Write back to file
    write_json_file(&mcp_json_path, &json_value, ".mcp.json")?;

    // Also remove from settings.json enabled/disabled arrays
    remove_mcp_from_settings(&server_name).await?;

    Ok(())
}

// Helper function to remove MCP server from settings arrays
async fn remove_mcp_from_settings(server_name: &str) -> Result<(), String> {
    let home_dir = home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    if !settings_path.exists() {
        return Ok(()); // Nothing to remove if settings doesn't exist
    }

    let mut settings = read_json_file(&settings_path, "settings.json")?;
    let settings_obj = settings.as_object_mut()
        .ok_or("Settings is not an object")?;

    // Remove from enabledMcpjsonServers
    if let Some(enabled) = settings_obj.get_mut("enabledMcpjsonServers") {
        if let Some(enabled_arr) = enabled.as_array_mut() {
            enabled_arr.retain(|v| v.as_str() != Some(server_name));
        }
    }

    // Remove from disabledMcpjsonServers
    if let Some(disabled) = settings_obj.get_mut("disabledMcpjsonServers") {
        if let Some(disabled_arr) = disabled.as_array_mut() {
            disabled_arr.retain(|v| v.as_str() != Some(server_name));
        }
    }

    write_json_file(&settings_path, &settings, "settings.json")?;

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct McpEnabledState {
    #[serde(rename = "enabledMcpjsonServers")]
    pub enabled_mcp_json_servers: Vec<String>,
    #[serde(rename = "disabledMcpjsonServers")]
    pub disabled_mcp_json_servers: Vec<String>,
    #[serde(rename = "disabledMcpServers")]
    pub disabled_mcp_servers: Vec<String>,  // For Direct servers
}

// Helper: Read settings from a specific file path
fn read_settings_from_file(settings_path: &std::path::Path) -> Result<McpEnabledState, String> {
    if !settings_path.exists() {
        return Ok(McpEnabledState {
            enabled_mcp_json_servers: vec![],
            disabled_mcp_json_servers: vec![],
            disabled_mcp_servers: vec![],
        });
    }

    let settings = read_json_file(settings_path, "settings file")?;

    Ok(McpEnabledState {
        enabled_mcp_json_servers: extract_string_array(&settings, "enabledMcpjsonServers"),
        disabled_mcp_json_servers: extract_string_array(&settings, "disabledMcpjsonServers"),
        disabled_mcp_servers: extract_string_array(&settings, "disabledMcpServers"),
    })
}

// Helper: Merge disabled MCP servers from .claude.json into state
fn merge_disabled_mcp_servers(mut state: McpEnabledState, cwd: Option<&str>) -> Result<McpEnabledState, String> {
    state.disabled_mcp_servers = read_disabled_mcp_servers_from_claude_json(cwd)?;
    Ok(state)
}

#[tauri::command]
pub async fn get_mcp_enabled_state(cwd: Option<String>) -> Result<McpEnabledState, String> {
    let settings_path = get_settings_path(cwd.as_deref(), false)?;
    let state = read_settings_from_file(&settings_path)?;
    merge_disabled_mcp_servers(state, cwd.as_deref())
}

#[tauri::command]
pub async fn toggle_mcp_server_state(server_name: String, enabled: bool, cwd: Option<String>) -> Result<(), String> {
    // Determine target settings file based on cwd (prefer local for writing)
    let settings_path = get_settings_path(cwd.as_deref(), true)?;

    // Log the action
    let project_info = if let Some(ref cwd_str) = cwd {
        format!("project: {}", cwd_str)
    } else {
        "global".to_string()
    };
    let action = if enabled { "enabled" } else { "disabled" };
    println!("🔧 MCP server {} {} - file: {}, {}", 
        action, 
        server_name, 
        settings_path.display(), 
        project_info
    );

    // Ensure settings directory exists
    let settings_dir = settings_path.parent().ok_or("Failed to get settings directory")?;
    if !settings_dir.exists() {
        ensure_dir(settings_dir, "settings directory")?;
    }

    // Read existing settings or create new
    let mut settings = read_json_file(&settings_path, "settings file")?;
    let settings_obj = settings.as_object_mut()
        .ok_or("Settings is not an object")?;

    // Ensure both arrays exist
    settings_obj
        .entry("enabledMcpjsonServers".to_string())
        .or_insert_with(|| Value::Array(vec![]));
    settings_obj
        .entry("disabledMcpjsonServers".to_string())
        .or_insert_with(|| Value::Array(vec![]));

    // Remove from enabled array
    if let Some(enabled_arr) = settings_obj
        .get_mut("enabledMcpjsonServers")
        .and_then(|v| v.as_array_mut())
    {
        enabled_arr.retain(|v: &Value| v.as_str() != Some(&server_name));
        if enabled {
            enabled_arr.push(Value::String(server_name.clone()));
        }
    }

    // Remove from disabled array
    if let Some(disabled_arr) = settings_obj
        .get_mut("disabledMcpjsonServers")
        .and_then(|v| v.as_array_mut())
    {
        disabled_arr.retain(|v: &Value| v.as_str() != Some(&server_name));
        if !enabled {
            disabled_arr.push(Value::String(server_name));
        }
    }

    // Write back to file
    write_json_file(&settings_path, &settings, "settings file")?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_direct_mcp_server(
    server_name: String,
    enabled: bool,
    cwd: Option<String>
) -> Result<(), String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");
    
    // Log the action
    let project_info = if let Some(ref cwd_str) = cwd {
        format!("project: {}", cwd_str)
    } else {
        "global".to_string()
    };
    let action = if enabled { "enabled" } else { "disabled" };
    println!("🔧 MCP server {} {} - file: {}, {}", 
        action, 
        server_name, 
        claude_json_path.display(), 
        project_info
    );
    
    let mut json_value = read_json_file(&claude_json_path, ".claude.json")?;
    let json_obj = json_value.as_object_mut().ok_or(".claude.json is not an object")?;
    
    // Determine target object: project-specific or root level
    let target_obj = if let Some(ref cwd_str) = cwd {
        // Write to .projects[cwd].disabledMcpServers
        let projects = json_obj
            .entry("projects".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or("projects is not an object")?;
        
        let project = projects
            .entry(cwd_str.clone())
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or("project entry is not an object")?;
        
        project
    } else {
        // Write to root level
        json_obj
    };
    
    // Update disabledMcpServers array
    let disabled_arr = target_obj
        .entry("disabledMcpServers".to_string())
        .or_insert_with(|| Value::Array(vec![]))
        .as_array_mut()
        .ok_or("disabledMcpServers is not an array")?;
    
    disabled_arr.retain(|v| v.as_str() != Some(&server_name));
    
    if !enabled {
        disabled_arr.push(Value::String(server_name));
    }
    
    // Write back
    write_json_file(&claude_json_path, &json_value, ".claude.json")?;
    
    Ok(())
}

// Helper: Read MCP servers from enabled plugins.
// When cwd is None (Global): include all installs (user + every project's local).
// When cwd is Some(path): include only user-scope installs + local-scope installs for that project.
fn read_plugin_mcp_servers(cwd: Option<&str>) -> Result<Vec<(String, serde_json::Map<String, Value>, String, String)>, String> {
    let home_dir = home_dir()?;
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");
    
    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;
    
    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;
    
    let mut enabled_cache: std::collections::HashMap<PathBuf, std::collections::HashMap<String, bool>> =
        std::collections::HashMap::new();
    let mut result = Vec::new();
    
    for (plugin_name, installs) in installed.plugins {
        for install in installs {
            // Filter by scope when a project is selected
            if !should_include_install(&install, cwd) {
                continue;
            }
            let enabled = if let Some(path) =
                enabled_plugins_settings_path(&home_dir, &install.scope, install.project_path.as_ref())
            {
                let map = enabled_cache
                    .entry(path.clone())
                    .or_insert_with(|| read_enabled_plugins(&path).unwrap_or_default());
                map.get(&plugin_name).copied().unwrap_or(true)
            } else {
                true
            };
            if !enabled {
                continue;
            }
            
            let packages = detect_packages(&install.install_path)?;
            if !packages.has_mcp {
                continue;
            }
            
            // Read .mcp.json from plugin directory
            let plugin_mcp_path = std::path::Path::new(&install.install_path).join(".mcp.json");
            if !plugin_mcp_path.exists() {
                continue;
            }
            
            let json_value = read_json_file(&plugin_mcp_path, "plugin .mcp.json")?;
            
            // Try standard format: mcpServers as object (server name -> config)
            if let Some(mcp_servers) = json_value.get("mcpServers").and_then(|s| s.as_object()) {
                for (server_name, server_config) in mcp_servers {
                    if let Some(obj) = server_config.as_object() {
                        result.push((
                            server_name.clone(),
                            obj.clone(),
                            plugin_name.clone(),
                            install.scope.clone(),
                        ));
                    }
                }
                continue;
            }
            
            // Try alternative format: mcpServers as array of configs
            if let Some(arr) = json_value.get("mcpServers").and_then(|s| s.as_array()) {
                for (i, item) in arr.iter().enumerate() {
                    if let Some(obj) = item.as_object() {
                        let name = obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| format!("{}-{}", plugin_name, i));
                        result.push((
                            name,
                            obj.clone(),
                            plugin_name.clone(),
                            install.scope.clone(),
                        ));
                    }
                }
                continue;
            }
            
            // Try alternative format: top-level object is the single server config (no mcpServers wrapper)
            if json_value.get("mcpServers").is_none() {
                if let Some(obj) = json_value.as_object() {
                    result.push((
                        plugin_name.clone(),
                        obj.clone(),
                        plugin_name.clone(),
                        install.scope.clone(),
                    ));
                }
            }
        }
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn get_mcp_servers_with_state(cwd: Option<String>) -> Result<Vec<McpServerState>, String> {
    let mut servers_map: std::collections::HashMap<String, McpServer> = std::collections::HashMap::new();
    let home_dir = home_dir()?;
    
    // Priority 1 (lowest): User-global from ~/.mcp.json and ~/.claude.json
    if let Ok(user_mcpjson) = read_mcpjson_servers(&home_dir) {
        for (name, config) in user_mcpjson {
            servers_map.insert(
                name.clone(),
                create_mcp_server(
                    config,
                    "mcpjson",
                    "user",
                    path_to_string(&home_dir.join(".mcp.json")),
                    true,
                ),
            );
        }
    }
    
    if let Ok(user_direct) = read_direct_servers(&home_dir) {
        for (name, config) in user_direct {
            servers_map.entry(name.clone()).or_insert_with(|| {
                create_mcp_server(
                    config,
                    "direct",
                    "user",
                    path_to_string(&home_dir.join(".claude.json")),
                    false,
                )
            });
        }
    }
    
    // Priority 1.5: Plugin MCP servers (between user-global and project)
    if let Ok(plugin_servers) = read_plugin_mcp_servers(cwd.as_deref()) {
        for (name, config, plugin_name, plugin_scope) in plugin_servers {
            servers_map.entry(name.clone()).or_insert_with(|| {
                create_mcp_server(
                    Value::Object(config),
                    "plugin",
                    &format!("plugin-{}", plugin_scope),
                    format!("Plugin: {} ({})", plugin_name, plugin_scope),
                    true,
                )
            });
        }
    }
    
    // Priority 2: Project scope (if cwd provided)
    if let Some(ref cwd_str) = cwd {
        if let Ok(project_servers) = read_project_mcp_servers(cwd_str) {
            for (name, config) in project_servers {
                servers_map.insert(
                    name.clone(),
                    create_mcp_server(
                        config,
                        "direct",
                        "project",
                        format!("~/.claude.json .projects[{}]", cwd_str),
                        false,
                    ),
                );
            }
        }
    }
    
    // Priority 3 (highest): Local scope (if cwd provided and project path exists)
    if let Some(ref cwd_str) = cwd {
        if let Ok(Some(project_path)) = get_project_path_from_claude_json(cwd_str) {
            if let Ok(local_servers) = read_local_mcp_servers(&project_path) {
                for (name, config) in local_servers {
                    servers_map.insert(
                        name.clone(),
                        create_mcp_server(
                            config,
                            "mcpjson",
                            "local",
                            path_to_string(&project_path.join(".mcp.json")),
                            true,
                        ),
                    );
                }
            }
        }
    }
    
    // Get enabled/disabled state and compute final state
    let state = get_mcp_enabled_state(cwd.clone()).await?;
    
    let mut result = Vec::new();
    
    for (name, server) in servers_map {
        let in_enabled = state.enabled_mcp_json_servers.contains(&name);
        let in_disabled = state.disabled_mcp_json_servers.contains(&name);
        
        // Compute state based on source type and arrays
        let computed_state = if server.source_type == "direct" {
            // For Direct servers, check disabledMcpServers
            if state.disabled_mcp_servers.contains(&name) {
                "disabled"
            } else {
                "enabled"
            }
        } else {
            // For MCPJSON servers, use three-state logic
            if in_disabled && !in_enabled {
                "disabled"  // Completely disabled
            } else if in_enabled && in_disabled {
                "runtime-disabled"  // Configured but temporarily disabled
            } else {
                "enabled"  // Default or explicitly enabled
            }
        };
        
        result.push(McpServerState {
            name: name.clone(),
            config: server.config,
            source_type: server.source_type,
            scope: server.scope,
            defined_in: server.defined_in,
            controllable: server.controllable,
            state: computed_state.to_string(),
            in_enabled_array: in_enabled,
            in_disabled_array: in_disabled,
        });
    }
    
    // Sort by name for consistent ordering
    result.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(result)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    println!("🔍 Checking for updates...");
    println!("📱 App version: {}", app.package_info().version);
    println!("🏷️  App identifier: {}", app.package_info().name);

    match app.updater() {
        Ok(updater) => {
            println!("✅ Updater initialized successfully");
            println!("📡 Checking update endpoint: https://github.com/djyde/ccmate-release/releases/latest/download/latest.json");

            match updater.check().await {
                Ok(Some(update)) => {
                    println!("🎉 Update available!");
                    println!("📦 Current version: {}", update.current_version);
                    println!("🚀 New version: {}", update.version);
                    println!("📝 Release notes: {:?}", update.body);
                    println!("📅 Release date: {:?}", update.date);
                    println!("🎯 Target platform: {:?}", update.target);

                    Ok(UpdateInfo {
                        available: true,
                        version: Some(update.version.clone()),
                        body: update.body.clone(),
                        date: update.date.map(|d| d.to_string()),
                    })
                }
                Ok(None) => {
                    println!("✅ No updates available - you're on the latest version");

                    Ok(UpdateInfo {
                        available: false,
                        version: None,
                        body: None,
                        date: None,
                    })
                }
                Err(e) => {
                    println!("❌ Error checking for updates: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn rebuild_tray_menu_command(app: tauri::AppHandle) -> Result<(), String> {
    crate::tray::rebuild_tray_menu(app).await
}

#[tauri::command]
pub async fn unlock_cc_ext() -> Result<(), String> {
    let home_dir = home_dir()?;
    let claude_config_path = home_dir.join(".claude/config.json");

    // Ensure .claude directory exists
    if let Some(parent) = claude_config_path.parent() {
        ensure_dir(parent, ".claude directory")?;
    }

    if claude_config_path.exists() {
        // File exists, check if primaryApiKey key exists
        let content = std::fs::read_to_string(&claude_config_path)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;

        let mut json_value: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;

        // Check if primaryApiKey exists
        if json_value.get("primaryApiKey").is_none() {
            // Add primaryApiKey to existing config
            if let Some(obj) = json_value.as_object_mut() {
                obj.insert("primaryApiKey".to_string(), Value::String("xxx".to_string()));
            }

            // Write back to file
            let json_content = serde_json::to_string_pretty(&json_value)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

            std::fs::write(&claude_config_path, json_content)
                .map_err(|e| format!("Failed to write config.json: {}", e))?;

            println!("Added primaryApiKey to existing config.json");
        } else {
            println!("primaryApiKey already exists in config.json, no action needed");
        }
    } else {
        // File doesn't exist, create it with primaryApiKey
        let config = serde_json::json!({
            "primaryApiKey": "xxx"
        });

        let json_content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        std::fs::write(&claude_config_path, json_content)
            .map_err(|e| format!("Failed to write config.json: {}", e))?;

        println!("Created new config.json with primaryApiKey");
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct UsageData {
    pub input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectUsageRecord {
    pub uuid: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub usage: Option<UsageData>,
}

#[tauri::command]
pub async fn read_project_usage_files() -> Result<Vec<ProjectUsageRecord>, String> {
    let home_dir = home_dir()?;
    let projects_dir = home_dir.join(".claude/projects");

    println!("🔍 Looking for projects directory: {}", projects_dir.display());

    if !projects_dir.exists() {
        println!("❌ Projects directory does not exist");
        return Ok(vec![]);
    }

    println!("✅ Projects directory exists");

    let mut all_records = Vec::new();
    let mut files_processed = 0;
    let mut lines_processed = 0;

    // Recursively find all .jsonl files in the projects directory and subdirectories
    fn find_jsonl_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|ext| ext == "jsonl").unwrap_or(false) {
                files.push(path);
            } else if path.is_dir() {
                // Recursively search subdirectories
                if let Err(e) = find_jsonl_files(&path, files) {
                    println!("Warning: {}", e);
                }
            }
        }
        Ok(())
    }

    let mut jsonl_files = Vec::new();
    find_jsonl_files(&projects_dir, &mut jsonl_files)?;

    for path in jsonl_files {
        files_processed += 1;
        // println!("📄 Processing file: {}", path.display());

        // Read the JSONL file
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

        // Process each line in the JSONL file
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            lines_processed += 1;

            // Parse the JSON line
            let json_value: Value = serde_json::from_str(line)
                .map_err(|e| format!("Failed to parse JSON line: {}", e))?;

            // Extract the required fields
            let uuid = json_value.get("uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = json_value.get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract model field (optional) - check both top-level and nested in message field
            let model = if let Some(model_str) = json_value.get("model")
                .and_then(|v| v.as_str()) {
                Some(model_str.to_string())
            } else if let Some(message_obj) = json_value.get("message") {
                message_obj.get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            };

            // Extract usage data (optional) - check both top-level and nested in message field
            let usage = if let Some(usage_obj) = json_value.get("usage") {
                Some(UsageData {
                    input_tokens: usage_obj.get("input_tokens").and_then(|v| v.as_u64()),
                    cache_read_input_tokens: usage_obj.get("cache_read_input_tokens").and_then(|v| v.as_u64()),
                    output_tokens: usage_obj.get("output_tokens").and_then(|v| v.as_u64()),
                })
            } else if let Some(message_obj) = json_value.get("message") {
                if let Some(usage_obj) = message_obj.get("usage") {
                    Some(UsageData {
                        input_tokens: usage_obj.get("input_tokens").and_then(|v| v.as_u64()),
                        cache_read_input_tokens: usage_obj.get("cache_read_input_tokens").and_then(|v| v.as_u64()),
                        output_tokens: usage_obj.get("output_tokens").and_then(|v| v.as_u64()),
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Only include records with valid uuid, timestamp, and valid usage data
            if !uuid.is_empty() && !timestamp.is_empty() {
                // Check if usage data exists and has meaningful token values
                if let Some(ref usage_data) = usage {
                    let input_tokens = usage_data.input_tokens.unwrap_or(0);
                    let output_tokens = usage_data.output_tokens.unwrap_or(0);

                    // Only include if input_tokens + output_tokens > 0
                    if input_tokens + output_tokens > 0 {
                        all_records.push(ProjectUsageRecord {
                            uuid,
                            timestamp,
                            model,
                            usage,
                        });
                    }
                }
            }
        }
    }

    println!("📊 Summary: Processed {} files, {} lines, found {} records", files_processed, lines_processed, all_records.len());
    Ok(all_records)
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MemoryFile {
    pub path: String,
    pub content: String,
    pub exists: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MemoryEntry {
    pub name: String,
    pub path: String,
    pub content: String,
    pub exists: bool,
    #[serde(rename = "source")]
    pub source: String, // "global" | "project"
    #[serde(rename = "projectPath", skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub disabled: bool,
}

fn global_memory_paths(home_dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let active = home_dir.join(".claude/CLAUDE.md");
    let disabled = home_dir.join(".claude/CLAUDE.md.disabled");
    (active, disabled)
}

fn project_memory_paths(project_path: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = std::path::Path::new(project_path);
    let active = base.join("CLAUDE.md");
    let disabled = base.join("CLAUDE.md.disabled");
    (active, disabled)
}

fn read_memory_entry_from_paths(
    active_path: &std::path::Path,
    disabled_path: &std::path::Path,
    name: String,
    source: String,
    project_path: Option<String>,
) -> Result<MemoryEntry, String> {
    let (content_path, disabled) = if active_path.is_file() {
        (active_path, false)
    } else if disabled_path.is_file() {
        (disabled_path, true)
    } else {
        // No file yet – treat as non-existing, empty memory
        return Ok(MemoryEntry {
            name,
            path: path_to_string(active_path),
            content: String::new(),
            exists: false,
            source,
            project_path,
            disabled: false,
        });
    };

    let content = std::fs::read_to_string(content_path).map_err(|e| {
        format!(
            "Failed to read memory file {}: {}",
            content_path.display(),
            e
        )
    })?;

    Ok(MemoryEntry {
        name,
        path: path_to_string(content_path),
        content,
        exists: true,
        source,
        project_path,
        disabled,
    })
}

fn get_project_paths_for_memory(
    home_dir: &std::path::Path,
) -> Result<Vec<String>, String> {
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(vec![]);
    }

    let json_value = read_json_file(&claude_json_path, ".claude.json")?;

    let projects_obj = json_value
        .get("projects")
        .and_then(|projects| projects.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    Ok(projects_obj.keys().cloned().collect())
}

fn resolve_memory_paths(
    source: &str,
    home_dir: &std::path::Path,
    project_path: &Option<String>,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    match source {
        "global" => Ok(global_memory_paths(home_dir)),
        "project" => {
            let project = project_path
                .as_ref()
                .ok_or_else(|| "Project path is required for project memory".to_string())?;
            Ok(project_memory_paths(project))
        }
        _ => Err("Unsupported source for memory file".to_string()),
    }
}

#[tauri::command]
pub async fn read_claude_memory() -> Result<MemoryFile, String> {
    let home_dir = home_dir()?;
    let claude_md_path = home_dir.join(".claude/CLAUDE.md");

    let path_str = path_to_string(&claude_md_path);

    if claude_md_path.exists() {
        let content = std::fs::read_to_string(&claude_md_path)
            .map_err(|e| format!("Failed to read CLAUDE.md file: {}", e))?;

        Ok(MemoryFile {
            path: path_str,
            content,
            exists: true,
        })
    } else {
        Ok(MemoryFile {
            path: path_str,
            content: String::new(),
            exists: false,
        })
    }
}

#[tauri::command]
pub async fn write_claude_memory(content: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let (active_path, disabled_path) = global_memory_paths(&home_dir);

    // Ensure .claude directory exists
    if let Some(parent) = active_path.parent() {
        ensure_dir(parent, ".claude directory")?;
    }

    // Always write enabled global memory for this legacy command
    std::fs::write(&active_path, content)
        .map_err(|e| format!("Failed to write CLAUDE.md file: {}", e))?;

    // Remove disabled file if it exists to keep state consistent
    if disabled_path.exists() {
        std::fs::remove_file(&disabled_path)
            .map_err(|e| format!("Failed to remove disabled CLAUDE.md file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_claude_memory_files() -> Result<Vec<MemoryEntry>, String> {
    let home_dir = home_dir()?;
    let mut entries = Vec::new();

    // Global memory – always include an entry to mirror previous behavior
    let (global_active, global_disabled) = global_memory_paths(&home_dir);
    let global_entry = read_memory_entry_from_paths(
        &global_active,
        &global_disabled,
        "global".to_string(),
        "global".to_string(),
        None,
    )?;
    entries.push(global_entry);

    // Project memories – based on .claude.json projects keys
    let project_paths = get_project_paths_for_memory(&home_dir)?;
    for project_path in project_paths {
        let (active, disabled) = project_memory_paths(&project_path);

        // Only include project entries if a file actually exists
        if !active.is_file() && !disabled.is_file() {
            continue;
        }

        let name = std::path::Path::new(&project_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&project_path)
            .to_string();

        let entry = read_memory_entry_from_paths(
            &active,
            &disabled,
            name,
            "project".to_string(),
            Some(project_path.clone()),
        )?;

        entries.push(entry);
    }

    // Sort: global first, then projects by name
    entries.sort_by(|a, b| {
        let order_a = if a.source == "global" { 0 } else { 1 };
        let order_b = if b.source == "global" { 0 } else { 1 };

        if order_a != order_b {
            order_a.cmp(&order_b)
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(entries)
}

#[tauri::command]
pub async fn write_claude_memory_file(
    source: String,
    project_path: Option<String>,
    content: String,
    disabled: bool,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    let (active_path, disabled_path) =
        resolve_memory_paths(source.as_str(), &home_dir, &project_path)?;

    // Ensure parent directory exists
    if let Some(parent) = active_path.parent() {
        ensure_dir(parent, "memory directory")?;
    }

    if disabled {
        // Write to disabled path and remove active if it exists
        std::fs::write(&disabled_path, content)
            .map_err(|e| format!("Failed to write disabled memory file: {}", e))?;
        if active_path.exists() {
            std::fs::remove_file(&active_path)
                .map_err(|e| format!("Failed to remove active memory file: {}", e))?;
        }
    } else {
        // Write to active path and remove disabled if it exists
        std::fs::write(&active_path, content)
            .map_err(|e| format!("Failed to write memory file: {}", e))?;
        if disabled_path.exists() {
            std::fs::remove_file(&disabled_path)
                .map_err(|e| format!("Failed to remove disabled memory file: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_claude_memory_file(
    source: String,
    project_path: Option<String>,
    disabled: bool,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    let (active_path, disabled_path) =
        resolve_memory_paths(source.as_str(), &home_dir, &project_path)?;

    let (from, to) = if disabled {
        // Disable: rename active -> disabled
        (&active_path, &disabled_path)
    } else {
        // Enable: rename disabled -> active
        (&disabled_path, &active_path)
    };

    if !from.exists() {
        return Err(format!(
            "Memory file {} does not exist",
            from.display()
        ));
    }

    std::fs::rename(from, to)
        .map_err(|e| format!("Failed to toggle memory file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_memory_file(
    source: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    let (active_path, disabled_path) =
        resolve_memory_paths(source.as_str(), &home_dir, &project_path)?;

    let mut removed_any = false;

    if active_path.exists() {
        std::fs::remove_file(&active_path)
            .map_err(|e| format!("Failed to delete memory file {}: {}", active_path.display(), e))?;
        removed_any = true;
    }

    if disabled_path.exists() {
        std::fs::remove_file(&disabled_path).map_err(|e| {
            format!(
                "Failed to delete disabled memory file {}: {}",
                disabled_path.display(),
                e
            )
        })?;
        removed_any = true;
    }

    if !removed_any {
        return Err("No memory file found to delete".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn install_and_restart(app: tauri::AppHandle) -> Result<(), String> {
    println!("🚀 Starting update installation process...");

    match app.updater() {
        Ok(updater) => {
            println!("✅ Updater ready for installation");
            println!("📡 Re-checking for updates to get download info...");

            match updater.check().await {
                Ok(Some(update)) => {
                    println!("📥 Starting download and installation...");
                    println!("🎯 Update version: {}", update.version);
                    println!("🎯 Update target: {:?}", update.target);

                    // Download and install the update
                    match update.download_and_install(
                        |chunk_length, content_length| {
                            let progress = if let Some(total) = content_length {
                                (chunk_length as f64 / total as f64) * 100.0
                            } else {
                                0.0
                            };
                            println!("⬇️  Download progress: {:.1}% ({} bytes)", progress, chunk_length);
                        },
                        || {
                            println!("✅ Download completed! Preparing to restart...");
                        }
                    ).await {
                        Ok(_) => {
                            println!("🔄 Update installed successfully! Restarting application in 500ms...");

                            // Schedule restart after a short delay to allow the response to be sent
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                println!("🔄 Restarting now!");
                                app_handle.restart();
                            });
                            Ok(())
                        }
                        Err(e) => {
                            println!("❌ Failed to install update: {}", e);
                            Err(format!("Failed to install update: {}", e))
                        }
                    }
                }
                Ok(None) => {
                    println!("ℹ️  No update available for installation");
                    Err("No update available".to_string())
                }
                Err(e) => {
                    println!("❌ Error checking for updates before installation: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get updater for installation: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

// Get or create distinct_id from stores.json
async fn get_or_create_distinct_id() -> Result<String, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    // Ensure app config directory exists
    ensure_dir(&app_config_path, "app config directory")?;

    // Read existing stores.json or create new one
    let mut stores_data = if stores_file.exists() {
        read_stores_file(&stores_file)?
    } else {
        StoresData {
            configs: vec![],
            distinct_id: None,
            notification: Some(NotificationSettings {
                enable: true,
                enabled_hooks: vec!["Notification".to_string()],
            }),
        }
    };

    // Return existing distinct_id or create new one
    if let Some(ref id) = stores_data.distinct_id {
        Ok(id.clone())
    } else {
        // Generate new UUID
        let new_id = Uuid::new_v4().to_string();
        stores_data.distinct_id = Some(new_id.clone());

        // Write back to stores.json
        write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

        println!("Created new distinct_id: {}", new_id);
        Ok(new_id)
    }
}

// Get operating system name in PostHog format
fn get_os_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS";
    #[cfg(target_os = "windows")]
    return "Windows";
    #[cfg(target_os = "linux")]
    return "Linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "Unknown";
}

// Get operating system version
fn get_os_version() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| format!("Failed to get macOS version: {}", e))?;

        let version = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse macOS version: {}", e))?;

        Ok(version.trim().to_string())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
            .map_err(|e| format!("Failed to get Windows version: {}", e))?;

        let version_str = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse Windows version: {}", e))?;

        // Extract version number from "Microsoft Windows [Version 10.0.19045.2364]"
        if let Some(start) = version_str.find("Version ") {
            let version_part = &version_str[start + 8..];
            let version = version_part.trim_end_matches("]").trim().to_string();
            Ok(version)
        } else {
            Ok("Unknown".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        // Try to read from /etc/os-release first
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("VERSION_ID=") {
                    let version = line.split('=').nth(1)
                        .unwrap_or("Unknown")
                        .trim_matches('"');
                    return Ok(version.to_string());
                }
            }
        }

        // Fallback to uname
        use std::process::Command;
        let output = Command::new("uname")
            .arg("-r")
            .output()
            .map_err(|e| format!("Failed to get Linux kernel version: {}", e))?;

        let version = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse Linux version: {}", e))?;

        Ok(version.trim().to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Ok("Unknown".to_string())
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectConfig {
    pub path: String,
    pub config: serde_json::Value,
}

#[tauri::command]
pub async fn read_claude_projects() -> Result<Vec<ProjectConfig>, String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&claude_json_path)
        .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

    let json_value: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse .claude.json: {}", e))?;

    let projects_obj = json_value.get("projects")
        .and_then(|projects| projects.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    let mut result = Vec::new();
    for (path, config) in projects_obj {
        let project_config = ProjectConfig {
            path: path.clone(),
            config: config.clone(),
        };
        result.push(project_config);
    }

    Ok(result)
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ClaudeConfigFile {
    pub path: String,
    pub content: Value,
    pub exists: bool,
}

#[tauri::command]
pub async fn read_claude_config_file() -> Result<ClaudeConfigFile, String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    let path_str = path_to_string(&claude_json_path);

    if claude_json_path.exists() {
        let content = std::fs::read_to_string(&claude_json_path)
            .map_err(|e| format!("Failed to read .claude.json: {}", e))?;

        let json_content: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(ClaudeConfigFile {
            path: path_str,
            content: json_content,
            exists: true,
        })
    } else {
        Ok(ClaudeConfigFile {
            path: path_str,
            content: Value::Object(serde_json::Map::new()),
            exists: false,
        })
    }
}

#[tauri::command]
pub async fn write_claude_config_file(content: Value) -> Result<(), String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    let json_content = serde_json::to_string_pretty(&content)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(&claude_json_path, json_content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn track(event: String, properties: serde_json::Value, app: tauri::AppHandle) -> Result<(), String> {
    println!("📊 Tracking event: {}", event);

    // Get distinct_id
    let distinct_id = get_or_create_distinct_id().await?;

    // Get app version
    let app_version = app.package_info().version.to_string();

    // Get OS information
    let os_name = get_os_name();
    let os_version = get_os_version().unwrap_or_else(|_| "Unknown".to_string());

    // Prepare request payload
    let mut payload = serde_json::json!({
        "api_key": "phc_zlfJLeYsreOvash1EhL6IO6tnP00exm75OT50SjnNcy",
        "event": event,
        "properties": {
            "distinct_id": distinct_id,
            "app_version": app_version,
            "$os": os_name,
            "$os_version": os_version
        }
    });

    // Merge additional properties
    if let Some(props_obj) = payload["properties"].as_object_mut() {
        if let Some(additional_props) = properties.as_object() {
            for (key, value) in additional_props {
                props_obj.insert(key.clone(), value.clone());
            }
        }
    }

    // Add timestamp if not provided
    if !payload["properties"].as_object().unwrap().contains_key("timestamp") {
        let timestamp = chrono::Utc::now().to_rfc3339();
        payload["properties"]["timestamp"] = serde_json::Value::String(timestamp);
    }

    println!("📤 Sending to PostHog: {}", serde_json::to_string_pretty(&payload).unwrap());

    // Send request to PostHog
    let client = reqwest::Client::new();
    let response = client
        .post("https://us.i.posthog.comxxxx/capture/")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to PostHog: {}", e))?;

    if response.status().is_success() {
        println!("✅ Event tracked successfully");
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        println!("❌ Failed to track event: {} - {}", status, error_text);
        Err(format!("PostHog API error: {} - {}", status, error_text))
    }
}

// Hook management functions

/// Get the latest hook command based on the current operating system
fn get_latest_hook_command() -> serde_json::Value {
    if cfg!(target_os = "windows") {
        serde_json::json!({
            "__ccmate__": true,
            "type": "command",
            "command": "powershell -Command \"try { Invoke-RestMethod -Uri http://localhost:59948/claude_code/hooks -Method POST -ContentType 'application/json' -Body $input -ErrorAction Stop } catch { '' }\""
        })
    } else {
        serde_json::json!({
            "__ccmate__": true,
            "type": "command",
            "command": "curl -s -X POST http://localhost:59948/claude_code/hooks -H 'Content-Type: application/json' --data-binary @- 2>/dev/null || echo"
        })
    }
}

/// Update existing ccmate hooks for specified events (doesn't add new ones)
fn update_existing_hooks(hooks_obj: &mut serde_json::Map<String, serde_json::Value>, events: &[&str]) -> Result<bool, String> {
    let latest_hook_command = get_latest_hook_command();
    let latest_command_str = latest_hook_command.get("command")
        .and_then(|cmd| cmd.as_str())
        .unwrap_or("");

    let mut hook_updated = false;

    for event in events {
        if let Some(event_hooks) = hooks_obj.get_mut(*event).and_then(|h| h.as_array_mut()) {
            // Find and update existing ccmate hooks only
            for entry in event_hooks.iter_mut() {
                if let Some(hooks_array) = entry.get_mut("hooks").and_then(|h| h.as_array_mut()) {
                    for hook in hooks_array.iter_mut() {
                        if hook.get("__ccmate__").is_some() {
                            // Compare only the command string, not the entire JSON object
                            if let Some(existing_command) = hook.get("command").and_then(|cmd| cmd.as_str()) {
                                if existing_command != latest_command_str {
                                    // Update only the command field, preserve other properties
                                    hook["command"] = serde_json::Value::String(latest_command_str.to_string());
                                    hook_updated = true;
                                    println!("🔄 Updated {} hook command: {}", event, latest_command_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(hook_updated)
}

/// Update or add ccmate hooks for specified events
fn update_or_add_hooks(hooks_obj: &mut serde_json::Map<String, serde_json::Value>, events: &[&str]) -> Result<bool, String> {
    let latest_hook_command = get_latest_hook_command();
    let mut hook_updated = false;

    for event in events {
        if let Some(event_hooks) = hooks_obj.get_mut(*event).and_then(|h| h.as_array_mut()) {
            // Find and update existing ccmate hooks
            for entry in event_hooks.iter_mut() {
                if let Some(hooks_array) = entry.get_mut("hooks").and_then(|h| h.as_array_mut()) {
                    for hook in hooks_array.iter_mut() {
                        if hook.get("__ccmate__").is_some() {
                            // Update the command to the latest version
                            if hook.get("command") != latest_hook_command.get("command") {
                                *hook = latest_hook_command.clone();
                                hook_updated = true;
                            }
                        }
                    }
                }
            }

            // If no ccmate hooks found, add one
            let ccmate_hook_exists = event_hooks.iter().any(|entry| {
                if let Some(hooks_array) = entry.get("hooks").and_then(|h| h.as_array()) {
                    hooks_array.iter().any(|hook| hook.get("__ccmate__").is_some())
                } else {
                    false
                }
            });

            if !ccmate_hook_exists {
                let ccmate_hook_entry = serde_json::json!({
                    "hooks": [latest_hook_command.clone()]
                });
                event_hooks.push(ccmate_hook_entry);
                hook_updated = true;
            }
        } else {
            // Create event hooks array with ccmate hook
            let ccmate_hook_entry = serde_json::json!({
                "hooks": [latest_hook_command.clone()]
            });
            hooks_obj.insert(event.to_string(), serde_json::Value::Array(vec![ccmate_hook_entry]));
            hook_updated = true;
        }
    }

    Ok(hook_updated)
}

#[tauri::command]
pub async fn get_notification_settings() -> Result<Option<NotificationSettings>, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    if !stores_file.exists() {
        return Ok(None);
    }

    let stores_data = read_stores_file(&stores_file)?;

    Ok(stores_data.notification)
}

#[tauri::command]
pub async fn update_claude_code_hook() -> Result<(), String> {
    let home_dir = home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    if !settings_path.exists() {
        // If settings file doesn't exist, just add the hooks
        return add_claude_code_hook().await;
    }

    // Read existing settings
    let mut settings = read_json_file(&settings_path, "settings.json")?;

    // Ensure hooks object exists
    let hooks_obj = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    // Update existing hooks for Notification, Stop, and PreToolUse events (only update, don't add new ones)
    let events = ["Notification", "Stop", "PreToolUse"];
    let hook_updated = update_existing_hooks(hooks_obj, &events)?;

    if hook_updated {
        // Write back to settings file
        // Create .claude directory if it doesn't exist
        if let Some(parent) = settings_path.parent() {
            ensure_dir(parent, ".claude directory")?;
        }

        write_json_file(&settings_path, &settings, "settings.json")?;

        println!("✅ Claude Code hooks updated successfully");
    } else {
        println!("ℹ️  Claude Code hooks are already up to date - no updates needed");
    }

    Ok(())
}

#[tauri::command]
pub async fn add_claude_code_hook() -> Result<(), String> {
    let home_dir = home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    // Read existing settings or create new structure
    let mut settings = read_json_file(&settings_path, "settings.json")?;

    // Ensure hooks object exists
    let hooks_obj = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();

    // Add hooks for Notification, Stop, and PreToolUse events
    let events = ["Notification", "Stop", "PreToolUse"];
    update_or_add_hooks(hooks_obj, &events)?;

    // Write back to settings file
    // Create .claude directory if it doesn't exist
    if let Some(parent) = settings_path.parent() {
        ensure_dir(parent, ".claude directory")?;
    }

    write_json_file(&settings_path, &settings, "settings.json")?;
    println!("✅ Claude Code hooks added successfully");
    Ok(())
}

#[tauri::command]
pub async fn remove_claude_code_hook() -> Result<(), String> {
    let home_dir = home_dir()?;
    let settings_path = home_dir.join(".claude/settings.json");

    if !settings_path.exists() {
        return Ok(()); // Settings file doesn't exist, nothing to remove
    }

    // Read existing settings
    let mut settings = read_json_file(&settings_path, "settings.json")?;

    // Check if hooks object exists
    if let Some(hooks_obj) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        let events = ["Notification", "Stop", "PreToolUse"];

        for event in events {
            if let Some(event_hooks) = hooks_obj.get_mut(event).and_then(|h| h.as_array_mut()) {
                // Remove hooks that have __ccmate__ key from nested hooks arrays
                let mut new_event_hooks = Vec::new();
                for entry in event_hooks.iter() {
                    if let Some(hooks_array) = entry.get("hooks").and_then(|h| h.as_array()) {
                        // Filter out hooks that have __ccmate__ key
                        let filtered_hooks: Vec<serde_json::Value> = hooks_array.iter()
                            .filter(|hook| hook.get("__ccmate__").is_none())
                            .cloned()
                            .collect();

                        // Keep the entry only if it still has hooks
                        if !filtered_hooks.is_empty() {
                            let mut new_entry = entry.clone();
                            new_entry["hooks"] = serde_json::Value::Array(filtered_hooks);
                            new_event_hooks.push(new_entry);
                        }
                    } else {
                        // Keep entries that don't have a hooks array
                        new_event_hooks.push(entry.clone());
                    }
                }
                *event_hooks = new_event_hooks;

                // If the event hooks array is empty, remove the entire event entry
                if event_hooks.is_empty() {
                    hooks_obj.remove(event);
                }
            }
        }

        // If hooks object is empty, remove it entirely
        if hooks_obj.is_empty() {
            settings.as_object_mut().unwrap().remove("hooks");
        }
    }

    // Write back to settings file
    write_json_file(&settings_path, &settings, "settings.json")?;
    println!("✅ Claude Code hooks removed successfully");
    Ok(())
}

#[tauri::command]
pub async fn update_notification_settings(settings: NotificationSettings) -> Result<(), String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let stores_file = app_config_path.join("stores.json");

    if !stores_file.exists() {
        // Create stores.json with notification settings if it doesn't exist
        let stores_data = StoresData {
            configs: vec![],
            distinct_id: None,
            notification: Some(settings.clone()),
        };

        // Ensure app config directory exists
        ensure_dir(&app_config_path, "app config directory")?;

        write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

        println!("Created stores.json with notification settings");
        return Ok(());
    }

    // Read existing stores
    let mut stores_data = read_stores_file(&stores_file)?;

    // Update notification settings
    stores_data.notification = Some(settings);

    // Write back to stores file
    write_json_file_serialize(&stores_file, &stores_data, "stores file")?;

    println!("✅ Notification settings updated successfully");
    Ok(())
}



#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CommandFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
    pub disabled: bool,
}

#[tauri::command]
pub async fn read_claude_commands() -> Result<Vec<CommandFile>, String> {
    let home_dir = home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");

    if !commands_dir.exists() {
        return Ok(vec![]);
    }

    let mut command_files = Vec::new();

    // Read all .md and .md.disabled files in the commands directory
    let entries = std::fs::read_dir(&commands_dir)
        .map_err(|e| format!("Failed to read commands directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            let file_name_str = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Check if it's a .md or .md.disabled file
            let (is_command_file, is_disabled) = if file_name_str.ends_with(".md.disabled") {
                (true, true)
            } else if file_name_str.ends_with(".md") {
                (true, false)
            } else {
                (false, false)
            };

            if is_command_file {
                // Extract the command name (without .md or .md.disabled)
                let command_name = if is_disabled {
                    file_name_str.trim_end_matches(".md.disabled").to_string()
                } else {
                    file_name_str.trim_end_matches(".md").to_string()
                };

                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read command file {}: {}", path.display(), e))?;

                command_files.push(CommandFile {
                    name: command_name,
                    content,
                    exists: true,
                    disabled: is_disabled,
                });
            }
        }
    }

    // Sort commands alphabetically by name
    command_files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(command_files)
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SkillFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
    #[serde(rename = "source")]
    pub source: String, // "global" | "plugin" | "project"
    #[serde(rename = "pluginName", skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(rename = "projectPath", skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub disabled: bool,
}

fn skill_base_dir_for_source(
    home_dir: &std::path::Path,
    source: &str,
    project_path: Option<&String>,
) -> Result<std::path::PathBuf, String> {
    if source == "global" {
        Ok(home_dir.join(".claude/skills"))
    } else if source == "project" {
        let project = project_path
            .ok_or_else(|| "Project path is required for project skills".to_string())?;
        Ok(std::path::PathBuf::from(project).join(".claude/skills"))
    } else {
        Err("Unsupported skill source".to_string())
    }
}

fn collect_user_skills(home_dir: &std::path::Path) -> Result<Vec<SkillFile>, String> {
    let skills_dir = home_dir.join(".claude/skills");

    if !skills_dir.exists() {
        return Ok(vec![]);
    }

    let mut skills = Vec::new();
    let entries = std::fs::read_dir(&skills_dir)
        .map_err(|e| format!("Failed to read skills directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let skill_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        let skill_md = path.join("SKILL.md");
        let skill_md_disabled = path.join("SKILL.md.disabled");

        let (content_path, disabled) = if skill_md.is_file() {
            (skill_md, false)
        } else if skill_md_disabled.is_file() {
            (skill_md_disabled, true)
        } else {
            continue;
        };

        let content = std::fs::read_to_string(&content_path)
            .map_err(|e| format!("Failed to read SKILL.md for {}: {}", skill_name, e))?;
        skills.push(SkillFile {
            name: skill_name,
            content,
            exists: true,
            source: "global".to_string(),
            plugin_name: None,
            project_path: None,
            disabled,
        });
    }

    Ok(skills)
}

fn collect_plugin_skills(home_dir: &std::path::Path) -> Result<Vec<SkillFile>, String> {
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");

    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;

    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;

    let mut enabled_cache: std::collections::HashMap<
        PathBuf,
        std::collections::HashMap<String, bool>,
    > = std::collections::HashMap::new();
    let mut skills = Vec::new();

    for (plugin_name, installs) in installed.plugins {
        for install in installs {
            let enabled = if let Some(path) = enabled_plugins_settings_path(
                home_dir,
                &install.scope,
                install.project_path.as_ref(),
            ) {
                let map = enabled_cache
                    .entry(path.clone())
                    .or_insert_with(|| read_enabled_plugins(&path).unwrap_or_default());
                map.get(&plugin_name).copied().unwrap_or(true)
            } else {
                true
            };

            if !enabled {
                continue;
            }

            let packages = detect_packages(&install.install_path)?;

            if !packages.has_skills {
                continue;
            }

            let skills_root = std::path::Path::new(&install.install_path).join("skills");

            if !skills_root.exists() || !skills_root.is_dir() {
                continue;
            }

            let entries = std::fs::read_dir(&skills_root)
                .map_err(|e| format!("Failed to read skills directory: {}", e))?;

            for entry in entries {
                let entry =
                    entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let skill_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_string();
                let skill_md = path.join("SKILL.md");

                if !skill_md.is_file() {
                    continue;
                }

                let content = std::fs::read_to_string(&skill_md).map_err(|e| {
                    format!(
                        "Failed to read SKILL.md for plugin skill {}: {}",
                        skill_name, e
                    )
                })?;

                skills.push(SkillFile {
                    name: skill_name,
                    content,
                    exists: true,
                    source: "plugin".to_string(),
                    plugin_name: Some(plugin_name.clone()),
                    project_path: install.project_path.clone(),
                    disabled: false,
                });
            }
        }
    }

    Ok(skills)
}

fn collect_project_skills(home_dir: &std::path::Path) -> Result<Vec<SkillFile>, String> {
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");

    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;

    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;

    let mut project_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for installs in installed.plugins.values() {
        for install in installs {
            if let Some(ref project_path) = install.project_path {
                project_paths.insert(project_path.clone());
            }
        }
    }

    let mut skills = Vec::new();

    for project_path in project_paths {
        let project_skills_dir =
            std::path::Path::new(&project_path).join(".claude/skills");

        if !project_skills_dir.exists() || !project_skills_dir.is_dir() {
            continue;
        }

        let entries = std::fs::read_dir(&project_skills_dir).map_err(|e| {
            format!(
                "Failed to read project skills directory {}: {}",
                project_skills_dir.display(),
                e
            )
        })?;

        for entry in entries {
            let entry =
                entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let skill_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();
            let skill_md = path.join("SKILL.md");
            let skill_md_disabled = path.join("SKILL.md.disabled");

            let (content_path, disabled) = if skill_md.is_file() {
                (skill_md, false)
            } else if skill_md_disabled.is_file() {
                (skill_md_disabled, true)
            } else {
                continue;
            };

            let content = std::fs::read_to_string(&content_path).map_err(|e| {
                format!(
                    "Failed to read SKILL.md for project skill {} (project {}): {}",
                    skill_name, project_path, e
                )
            })?;

            skills.push(SkillFile {
                name: skill_name,
                content,
                exists: true,
                source: "project".to_string(),
                plugin_name: None,
                project_path: Some(project_path.clone()),
                disabled,
            });
        }
    }

    Ok(skills)
}

#[tauri::command]
pub async fn list_claude_skills() -> Result<Vec<SkillFile>, String> {
    let home_dir = home_dir()?;
    let mut skills = Vec::new();

    skills.extend(collect_user_skills(&home_dir)?);
    skills.extend(collect_plugin_skills(&home_dir)?);
    skills.extend(collect_project_skills(&home_dir)?);

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

#[tauri::command]
pub async fn toggle_claude_skill(
    name: String,
    source: String,
    project_path: Option<String>,
    disabled: bool,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    if source == "plugin" {
        return Err("Cannot toggle plugin skills from this interface".to_string());
    }

    let base_dir =
        skill_base_dir_for_source(&home_dir, &source, project_path.as_ref())?;

    let skill_dir = base_dir.join(&name);

    if !skill_dir.exists() || !skill_dir.is_dir() {
        return Err(format!(
            "Skill directory {} does not exist",
            skill_dir.display()
        ));
    }

    let (source_path, target_path) = if disabled {
        // Disable: rename SKILL.md to SKILL.md.disabled
        (
            skill_dir.join("SKILL.md"),
            skill_dir.join("SKILL.md.disabled"),
        )
    } else {
        // Enable: rename SKILL.md.disabled to SKILL.md
        (
            skill_dir.join("SKILL.md.disabled"),
            skill_dir.join("SKILL.md"),
        )
    };

    if !source_path.exists() {
        return Err(format!(
            "Skill file {} does not exist",
            source_path.display()
        ));
    }

    std::fs::rename(&source_path, &target_path)
        .map_err(|e| format!("Failed to toggle skill file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn write_claude_skill(
    name: String,
    source: String,
    project_path: Option<String>,
    content: String,
    disabled: bool,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    if source == "plugin" {
        return Err("Cannot write plugin skills from this interface".to_string());
    }

    let base_dir =
        skill_base_dir_for_source(&home_dir, &source, project_path.as_ref())?;

    // Ensure base .claude/skills directory exists
    ensure_dir(&base_dir, "skills directory")?;

    let skill_dir = base_dir.join(&name);
    ensure_dir(&skill_dir, "skill directory")?;

    let active_path = skill_dir.join("SKILL.md");
    let disabled_path = skill_dir.join("SKILL.md.disabled");

    if disabled {
        // Write to SKILL.md.disabled and remove SKILL.md if it exists
        std::fs::write(&disabled_path, content.clone())
            .map_err(|e| format!("Failed to write disabled skill file: {}", e))?;
        if active_path.exists() {
            std::fs::remove_file(&active_path)
                .map_err(|e| format!("Failed to remove active skill file: {}", e))?;
        }
    } else {
        // Write to SKILL.md and remove SKILL.md.disabled if it exists
        std::fs::write(&active_path, content.clone())
            .map_err(|e| format!("Failed to write skill file: {}", e))?;
        if disabled_path.exists() {
            std::fs::remove_file(&disabled_path)
                .map_err(|e| format!("Failed to remove disabled skill file: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_skill(
    name: String,
    source: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    if source == "plugin" {
        return Err("Cannot delete plugin skills from this interface".to_string());
    }

    let base_dir =
        skill_base_dir_for_source(&home_dir, &source, project_path.as_ref())?;

    let skill_dir = base_dir.join(&name);

    if !skill_dir.exists() || !skill_dir.is_dir() {
        return Ok(());
    }

    let active_path = skill_dir.join("SKILL.md");
    let disabled_path = skill_dir.join("SKILL.md.disabled");

    if active_path.exists() {
        std::fs::remove_file(&active_path)
            .map_err(|e| format!("Failed to delete skill file: {}", e))?;
    }

    if disabled_path.exists() {
        std::fs::remove_file(&disabled_path)
            .map_err(|e| format!("Failed to delete disabled skill file: {}", e))?;
    }

    // Attempt to remove skill directory if empty
    if skill_dir.read_dir().map_err(|e| format!("Failed to read skill directory: {}", e))?.next().is_none() {
        let _ = std::fs::remove_dir(&skill_dir);
    }

    Ok(())
}

#[tauri::command]
pub async fn write_claude_command(command_name: String, content: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");
    let command_file_path = commands_dir.join(format!("{}.md", command_name));

    // Ensure .claude/commands directory exists
    ensure_dir(&commands_dir, ".claude/commands directory")?;

    std::fs::write(&command_file_path, content)
        .map_err(|e| format!("Failed to write command file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_command(command_name: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");
    let command_file_path = commands_dir.join(format!("{}.md", command_name));

    if command_file_path.exists() {
        std::fs::remove_file(&command_file_path)
            .map_err(|e| format!("Failed to delete command file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_claude_command(command_name: String, disabled: bool) -> Result<(), String> {
    let home_dir = home_dir()?;
    let commands_dir = home_dir.join(".claude/commands");

    let (source_path, target_path) = if disabled {
        // Disable: rename from .md to .md.disabled
        (
            commands_dir.join(format!("{}.md", command_name)),
            commands_dir.join(format!("{}.md.disabled", command_name)),
        )
    } else {
        // Enable: rename from .md.disabled to .md
        (
            commands_dir.join(format!("{}.md.disabled", command_name)),
            commands_dir.join(format!("{}.md", command_name)),
        )
    };

    if !source_path.exists() {
        return Err(format!(
            "Command file {} does not exist",
            source_path.display()
        ));
    }

    std::fs::rename(&source_path, &target_path)
        .map_err(|e| format!("Failed to toggle command file: {}", e))?;

    Ok(())
}

// Agent management functions

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AgentFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
    pub disabled: bool,
}

#[derive(serde::Serialize)]
pub struct PluginAgentFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    #[serde(rename = "pluginScope")]
    pub plugin_scope: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

#[tauri::command]
pub async fn read_claude_agents() -> Result<Vec<AgentFile>, String> {
    let home_dir = home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");

    if !agents_dir.exists() {
        return Ok(vec![]);
    }

    let mut agent_files = Vec::new();

    // Read all .md and .md.disabled files in the agents directory
    let entries = std::fs::read_dir(&agents_dir)
        .map_err(|e| format!("Failed to read agents directory: {}", e))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            let file_name_str = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Check if it's a .md or .md.disabled file
            let (is_agent_file, is_disabled) = if file_name_str.ends_with(".md.disabled")
            {
                (true, true)
            } else if file_name_str.ends_with(".md") {
                (true, false)
            } else {
                (false, false)
            };

            if is_agent_file {
                // Extract the agent name (without .md or .md.disabled)
                let agent_name = if is_disabled {
                    file_name_str
                        .trim_end_matches(".md.disabled")
                        .to_string()
                } else {
                    file_name_str.trim_end_matches(".md").to_string()
                };

                let content = std::fs::read_to_string(&path).map_err(|e| {
                    format!(
                        "Failed to read agent file {}: {}",
                        path.display(),
                        e
                    )
                })?;

                agent_files.push(AgentFile {
                    name: agent_name,
                    content,
                    exists: true,
                    disabled: is_disabled,
                });
            }
        }
    }

    // Sort agents alphabetically by name
    agent_files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(agent_files)
}

#[tauri::command]
pub async fn write_claude_agent(agent_name: String, content: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");
    let agent_file_path = agents_dir.join(format!("{}.md", agent_name));

    // Ensure .claude/agents directory exists
    ensure_dir(&agents_dir, ".claude/agents directory")?;

    std::fs::write(&agent_file_path, content)
        .map_err(|e| format!("Failed to write agent file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_claude_agent(agent_name: String) -> Result<(), String> {
    let home_dir = home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");
    let active_path = agents_dir.join(format!("{}.md", agent_name));
    let disabled_path = agents_dir.join(format!("{}.md.disabled", agent_name));

    if active_path.exists() {
        std::fs::remove_file(&active_path)
            .map_err(|e| format!("Failed to delete agent file: {}", e))?;
    }

    if disabled_path.exists() {
        std::fs::remove_file(&disabled_path)
            .map_err(|e| format!("Failed to delete disabled agent file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_claude_agent(
    agent_name: String,
    disabled: bool,
) -> Result<(), String> {
    let home_dir = home_dir()?;
    let agents_dir = home_dir.join(".claude/agents");

    let (source_path, target_path) = if disabled {
        // Disable: rename from .md to .md.disabled
        (
            agents_dir.join(format!("{}.md", agent_name)),
            agents_dir.join(format!("{}.md.disabled", agent_name)),
        )
    } else {
        // Enable: rename from .md.disabled to .md
        (
            agents_dir.join(format!("{}.md.disabled", agent_name)),
            agents_dir.join(format!("{}.md", agent_name)),
        )
    };

    if !source_path.exists() {
        return Err(format!(
            "Agent file {} does not exist",
            source_path.display()
        ));
    }

    std::fs::rename(&source_path, &target_path)
        .map_err(|e| format!("Failed to toggle agent file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn read_plugin_agents() -> Result<Vec<PluginAgentFile>, String> {
    let home_dir = home_dir()?;
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");
    
    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;
    
    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;
    
    let mut enabled_cache: std::collections::HashMap<PathBuf, std::collections::HashMap<String, bool>> =
        std::collections::HashMap::new();
    let mut result = Vec::new();
    
    for (plugin_name, installs) in installed.plugins {
        for install in installs {
            let enabled = if let Some(path) =
                enabled_plugins_settings_path(&home_dir, &install.scope, install.project_path.as_ref())
            {
                let map = enabled_cache
                    .entry(path.clone())
                    .or_insert_with(|| read_enabled_plugins(&path).unwrap_or_default());
                map.get(&plugin_name).copied().unwrap_or(true)
            } else {
                true
            };
            if !enabled {
                continue;
            }
            
            let packages = detect_packages(&install.install_path)?;
            
            // Skip if plugin doesn't have agents
            if !packages.has_agents {
                continue;
            }
            
            // Walk the agents directory
            let agents_dir = std::path::Path::new(&install.install_path).join("agents");
            
            if !agents_dir.exists() || !agents_dir.is_dir() {
                continue;
            }
            
            // Read all .md files in the agents directory
            let entries = std::fs::read_dir(&agents_dir)
                .map_err(|e| format!("Failed to read agents directory: {}", e))?;
            
            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let path = entry.path();
                
                if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
                    let agent_name = path.file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    let content = std::fs::read_to_string(&path)
                        .map_err(|e| format!("Failed to read agent file {}: {}", path.display(), e))?;
                    
                    result.push(PluginAgentFile {
                        name: agent_name,
                        content,
                        exists: true,
                        plugin_name: plugin_name.clone(),
                        plugin_scope: install.scope.clone(),
                        source_path: path_to_string(&path),
                    });
                }
            }
        }
    }
    
    // Sort agents alphabetically by name
    result.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(result)
}

// Plugin management structures and functions

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PluginInstallInfo {
    pub scope: String,
    #[serde(rename = "installPath")]
    pub install_path: String,
    pub version: String,
    #[serde(rename = "installedAt")]
    pub installed_at: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    #[serde(rename = "gitCommitSha")]
    pub git_commit_sha: String,
    #[serde(rename = "projectPath", skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstalledPluginsFile {
    pub plugins: std::collections::HashMap<String, Vec<PluginInstallInfo>>,
}

#[derive(serde::Serialize, Clone)]
pub struct PluginPackages {
    #[serde(rename = "hasAgents")]
    pub has_agents: bool,
    #[serde(rename = "hasSkills")]
    pub has_skills: bool,
    #[serde(rename = "hasCommands")]
    pub has_commands: bool,
    #[serde(rename = "hasMcp")]
    pub has_mcp: bool,
}

#[derive(serde::Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub scope: String,
    pub version: String,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
    pub enabled: bool,
    pub packages: PluginPackages,
    #[serde(rename = "installPath")]
    pub install_path: String,
    #[serde(rename = "installedAt")]
    pub installed_at: String,
}

#[derive(serde::Serialize)]
pub struct PluginCommandFile {
    pub name: String,
    pub content: String,
    pub exists: bool,
    pub disabled: bool,
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    #[serde(rename = "pluginScope")]
    pub plugin_scope: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

fn detect_packages(install_path: &str) -> Result<PluginPackages, String> {
    let path = std::path::Path::new(install_path);
    
    // Check if install path exists
    if !path.exists() {
        println!("Warning: Install path does not exist: {}", install_path);
        return Ok(PluginPackages {
            has_agents: false,
            has_skills: false,
            has_commands: false,
            has_mcp: false,
        });
    }
    
    // List contents of install path for debugging
    if let Ok(entries) = std::fs::read_dir(path) {
        println!("Contents of {}: ", install_path);
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                let name = entry.file_name();
                let type_str = if file_type.is_dir() { "DIR" } else { "FILE" };
                println!("  {} - {:?}", type_str, name);
            }
        }
    }
    
    let agents_path = path.join("agents");
    let skills_path = path.join("skills");
    let commands_path = path.join("commands");
    let mcp_path = path.join(".mcp.json");
    
    let has_agents = agents_path.exists() && agents_path.is_dir();
    let has_skills = skills_path.exists() && skills_path.is_dir();
    let has_commands = commands_path.exists() && commands_path.is_dir();
    let has_mcp = mcp_path.exists() && mcp_path.is_file();
    
    println!("Package detection for {}: agents={}, skills={}, commands={}, mcp={}", 
             install_path, has_agents, has_skills, has_commands, has_mcp);
    
    Ok(PluginPackages {
        has_agents,
        has_skills,
        has_commands,
        has_mcp,
    })
}

fn read_enabled_plugins(settings_path: &std::path::Path) -> Result<std::collections::HashMap<String, bool>, String> {
    let settings = read_json_file(settings_path, "settings")?;
    
    let mut result = std::collections::HashMap::new();
    
    if let Some(enabled_plugins) = settings.get("enabledPlugins").and_then(|v| v.as_object()) {
        for (key, value) in enabled_plugins {
            if let Some(enabled) = value.as_bool() {
                result.insert(key.clone(), enabled);
            }
        }
    }
    
    Ok(result)
}

fn enabled_plugins_settings_path(
    home_dir: &std::path::Path,
    scope: &str,
    project_path: Option<&String>,
) -> Option<PathBuf> {
    if scope == "local" {
        project_path.map(|p| PathBuf::from(p).join(".claude/settings.local.json"))
    } else {
        Some(home_dir.join(".claude/settings.json"))
    }
}

#[tauri::command]
pub async fn read_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    let home_dir = home_dir()?;
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");
    
    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;
    
    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;
    
    let mut enabled_cache: std::collections::HashMap<PathBuf, std::collections::HashMap<String, bool>> =
        std::collections::HashMap::new();
    let mut result = Vec::new();
    
    for (plugin_name, installs) in installed.plugins {
        for install in installs {
            let enabled = if let Some(path) =
                enabled_plugins_settings_path(&home_dir, &install.scope, install.project_path.as_ref())
            {
                let map = enabled_cache
                    .entry(path.clone())
                    .or_insert_with(|| read_enabled_plugins(&path).unwrap_or_default());
                map.get(&plugin_name).copied().unwrap_or(true)
            } else {
                true
            };
            
            let packages = detect_packages(&install.install_path)?;
            
            println!("Plugin: {} | Scope: {} | ProjectPath: {:?}", 
                     plugin_name, install.scope, install.project_path);
            
            result.push(PluginInfo {
                name: plugin_name.clone(),
                scope: install.scope,
                version: install.version,
                project_path: install.project_path,
                enabled,
                packages,
                install_path: install.install_path,
                installed_at: install.installed_at.clone(),
            });
        }
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn toggle_plugin(
    plugin_name: String,
    enabled: bool,
    scope: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let home_dir = home_dir()?;

    let settings_path =
        plugin_settings_path(&home_dir, &scope, project_path.as_ref()).map_err(|e| e)?;

    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            ensure_dir(parent, "directory")?;
        }
    }

    let mut settings = read_json_file(&settings_path, "settings")?;

    update_enabled_plugins(&mut settings, plugin_name, enabled)?;

    write_json_file(&settings_path, &settings, "settings")?;
    Ok(())
}

fn plugin_settings_path(
    home_dir: &std::path::Path,
    scope: &str,
    project_path: Option<&String>,
) -> Result<std::path::PathBuf, String> {
    if scope == "local" {
        if let Some(proj_path) = project_path {
            Ok(std::path::PathBuf::from(proj_path).join(".claude/settings.local.json"))
        } else {
            Err("Project path required for local scope".to_string())
        }
    } else {
        Ok(home_dir.join(".claude/settings.json"))
    }
}

fn update_enabled_plugins(
    settings: &mut serde_json::Value,
    plugin_name: String,
    enabled: bool,
) -> Result<(), String> {
    let settings_obj = settings
        .as_object_mut()
        .ok_or_else(|| "Settings is not an object".to_string())?;

    let enabled_plugins = settings_obj
        .entry("enabledPlugins".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| "enabledPlugins is not an object".to_string())?;

    enabled_plugins.insert(plugin_name, Value::Bool(enabled));
    Ok(())
}

#[tauri::command]
pub async fn read_plugin_commands() -> Result<Vec<PluginCommandFile>, String> {
    let home_dir = home_dir()?;
    let plugins_file_path = home_dir.join(".claude/plugins/installed_plugins.json");
    
    if !plugins_file_path.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&plugins_file_path)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;
    
    let installed: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;
    
    let mut enabled_cache: std::collections::HashMap<PathBuf, std::collections::HashMap<String, bool>> =
        std::collections::HashMap::new();
    let mut result = Vec::new();
    
    for (plugin_name, installs) in installed.plugins {
        for install in installs {
            let enabled = if let Some(path) = enabled_plugins_settings_path(
                &home_dir,
                &install.scope,
                install.project_path.as_ref(),
            ) {
                let map = enabled_cache
                    .entry(path.clone())
                    .or_insert_with(|| read_enabled_plugins(&path).unwrap_or_default());
                map.get(&plugin_name).copied().unwrap_or(true)
            } else {
                true
            };
            if !enabled {
                continue;
            }
            
            let packages = detect_packages(&install.install_path)?;
            
            // Skip if plugin doesn't have commands
            if !packages.has_commands {
                continue;
            }
            
            let commands_dir = std::path::Path::new(&install.install_path).join("commands");
            
            if !commands_dir.exists() || !commands_dir.is_dir() {
                continue;
            }
            
            let entries = std::fs::read_dir(&commands_dir)
                .map_err(|e| format!("Failed to read commands directory: {}", e))?;
            
            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(command_file) =
                        read_command_file(&path, &plugin_name, &install.scope)?
                    {
                        result.push(command_file);
                    }
                }
            }
        }
    }
    
    // Sort commands alphabetically by name
    result.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(result)
}

fn read_command_file(
    path: &std::path::Path,
    plugin_name: &str,
    scope: &str,
) -> Result<Option<PluginCommandFile>, String> {
    let file_name_str = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    let (is_command_file, is_disabled) = if file_name_str.ends_with(".md.disabled") {
        (true, true)
    } else if file_name_str.ends_with(".md") {
        (true, false)
    } else {
        (false, false)
    };

    if !is_command_file {
        return Ok(None);
    }

    let command_name = if is_disabled {
        file_name_str.trim_end_matches(".md.disabled").to_string()
    } else {
        file_name_str.trim_end_matches(".md").to_string()
    };

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read command file {}: {}", path.display(), e))?;

    Ok(Some(PluginCommandFile {
        name: command_name,
        content,
        exists: true,
        disabled: is_disabled,
        plugin_name: plugin_name.to_string(),
        plugin_scope: scope.to_string(),
        source_path: path_to_string(path),
    }))
}

// -----------------------------------------------------------------------------
// Security Packs (Security Templates) – install/uninstall & manifest
// -----------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SkillTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CommandTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct McpTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "serverName")]
    pub server_name: String,
    #[serde(rename = "serverConfig")]
    pub server_config: Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SecurityTemplatesFile {
    pub agents: Vec<AgentTemplate>,
    pub skills: Vec<SkillTemplate>,
    pub commands: Vec<CommandTemplate>,
    pub mcp: Vec<McpTemplate>,
    // Reserved for future use – currently unused in phase 1
    pub plugins: Vec<Value>,
    pub hooks: Vec<Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SkillFilePayload {
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    pub content: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SecurityPackInstallPayload {
    #[serde(rename = "type")]
    pub template_type: String, // "agent" | "command" | "skill" | "mcp"
    pub id: String,
    pub content: Option<String>,                    // for agents/commands
    #[serde(rename = "skillFiles")]
    pub skill_files: Option<Vec<SkillFilePayload>>, // for skills (full directory)
    #[serde(rename = "serverName")]
    pub server_name: Option<String>,                // for MCP
    #[serde(rename = "serverConfig")]
    pub server_config: Option<Value>,               // for MCP
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct InstalledSecurityPackItem {
    #[serde(rename = "type")]
    pub template_type: String,
    pub id: String,
    #[serde(rename = "targetPath")]
    pub target_path: String,
    #[serde(rename = "installedAt")]
    pub installed_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstalledSecurityPacksFile {
    pub version: u32,
    pub items: Vec<InstalledSecurityPackItem>,
}

fn security_packs_manifest_path() -> Result<std::path::PathBuf, String> {
    let home_dir = home_dir()?;
    let app_config_path = home_dir.join(APP_CONFIG_DIR);
    let security_packs_dir = app_config_path.join("security_packs");
    ensure_dir(&security_packs_dir, "security packs directory")?;
    Ok(security_packs_dir.join("installed.json"))
}

fn read_security_packs_manifest() -> Result<InstalledSecurityPacksFile, String> {
    let path = security_packs_manifest_path()?;
    if !path.exists() {
        return Ok(InstalledSecurityPacksFile {
            version: 1,
            items: Vec::new(),
        });
    }

    let value = read_json_file(&path, "security packs manifest")?;
    let manifest: InstalledSecurityPacksFile = serde_json::from_value(value)
        .map_err(|e| format!("Failed to parse security packs manifest: {}", e))?;
    Ok(manifest)
}

fn write_security_packs_manifest(manifest: &InstalledSecurityPacksFile) -> Result<(), String> {
    let path = security_packs_manifest_path()?;
    write_json_file_serialize(&path, manifest, "security packs manifest")
}

fn load_security_templates_from_assets() -> Result<SecurityTemplatesFile, String> {
    // The JSON file lives under the frontend src assets directory.
    // We include it at compile time so the backend can serve it to the UI.
    let raw = include_str!("../../src/assets/security_packs/security_templates.json");
    serde_json::from_str(raw)
        .map_err(|e| format!("Failed to parse security_templates.json: {}", e))
}

fn install_file_template(
    home_dir: &std::path::Path,
    template_type: &str,
    id: &str,
    content: String,
    subdirectory: &str,
) -> Result<std::path::PathBuf, String> {
    let target_dir = home_dir.join(format!(".claude/{}", subdirectory));
    ensure_dir(&target_dir, &format!(".claude/{} directory", subdirectory))?;
    let target = target_dir.join(format!("{}.md", id));
    
    if target.exists() {
        return Err(format!(
            "{} file already exists: {}",
            template_type,
            target.display()
        ));
    }
    
    std::fs::write(&target, content)
        .map_err(|e| format!("Failed to write {} file {}: {}", template_type, target.display(), e))?;
    
    Ok(target)
}

#[tauri::command]
pub async fn get_security_templates() -> Result<SecurityTemplatesFile, String> {
    load_security_templates_from_assets()
}

#[tauri::command]
pub async fn get_installed_security_templates() -> Result<Vec<InstalledSecurityPackItem>, String> {
    let manifest = read_security_packs_manifest()?;
    Ok(manifest.items)
}

#[tauri::command]
pub async fn install_security_template(
    payload: SecurityPackInstallPayload,
) -> Result<(), String> {
    let home_dir = home_dir()?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut manifest = read_security_packs_manifest()?;

    match payload.template_type.as_str() {
        "agent" => {
            let content = payload
                .content
                .ok_or_else(|| "Agent install payload missing content".to_string())?;
            let target = install_file_template(&home_dir, "agent", &payload.id, content, "agents")?;

            manifest.items.push(InstalledSecurityPackItem {
                template_type: "agent".to_string(),
                id: payload.id,
                target_path: path_to_string(&target),
                installed_at: now,
            });
        }
        "command" => {
            let content = payload
                .content
                .ok_or_else(|| "Command install payload missing content".to_string())?;
            let target = install_file_template(&home_dir, "command", &payload.id, content, "commands")?;

            manifest.items.push(InstalledSecurityPackItem {
                template_type: "command".to_string(),
                id: payload.id,
                target_path: path_to_string(&target),
                installed_at: now,
            });
        }
        "skill" => {
            let skill_files = payload
                .skill_files
                .ok_or_else(|| "Skill install payload missing skillFiles".to_string())?;
            let skills_root = home_dir.join(".claude/skills");
            ensure_dir(&skills_root, ".claude/skills directory")?;
            let target_dir = skills_root.join(&payload.id);
            if target_dir.exists() {
                return Err(format!(
                    "Skill directory already exists: {}",
                    target_dir.display()
                ));
            }
            ensure_dir(&target_dir, "skill directory")?;

            for file in skill_files {
                let rel = std::path::Path::new(&file.relative_path);
                // Prevent directory traversal outside the skill root
                if rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                    return Err(format!(
                        "Invalid skill file path (parent dir not allowed): {}",
                        file.relative_path
                    ));
                }
                let full_path = target_dir.join(rel);
                if let Some(parent) = full_path.parent() {
                    ensure_dir(parent, "skill file parent directory")?;
                }
                std::fs::write(&full_path, &file.content).map_err(|e| {
                    format!(
                        "Failed to write skill file {}: {}",
                        full_path.display(),
                        e
                    )
                })?;
            }

            manifest.items.push(InstalledSecurityPackItem {
                template_type: "skill".to_string(),
                id: payload.id,
                target_path: path_to_string(&target_dir),
                installed_at: now,
            });
        }
        "mcp" => {
            let server_name = payload
                .server_name
                .ok_or_else(|| "MCP install payload missing serverName".to_string())?;
            let server_config = payload
                .server_config
                .ok_or_else(|| "MCP install payload missing serverConfig".to_string())?;

            // Reuse existing helper to write into ~/.mcp.json
            update_global_mcp_server(server_name.clone(), server_config).await?;

            manifest.items.push(InstalledSecurityPackItem {
                template_type: "mcp".to_string(),
                id: server_name,
                target_path: String::from("mcp"),
                installed_at: now,
            });
        }
        other => {
            return Err(format!("Unsupported security template type: {}", other));
        }
    }

    write_security_packs_manifest(&manifest)?;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_security_template(
    template_type: String,
    id: String,
) -> Result<(), String> {
    let mut manifest = read_security_packs_manifest()?;
    let mut remaining: Vec<InstalledSecurityPackItem> = Vec::new();

    for item in manifest.items.into_iter() {
        if item.template_type == template_type && item.id == id {
            match template_type.as_str() {
                "agent" | "command" => {
                    let path = std::path::PathBuf::from(&item.target_path);
                    if path.exists() {
                        std::fs::remove_file(&path).map_err(|e| {
                            format!("Failed to remove file {}: {}", path.display(), e)
                        })?;
                    }
                }
                "skill" => {
                    let path = std::path::PathBuf::from(&item.target_path);
                    if path.exists() {
                        std::fs::remove_dir_all(&path).map_err(|e| {
                            format!("Failed to remove skill directory {}: {}", path.display(), e)
                        })?;
                    }
                }
                "mcp" => {
                    delete_global_mcp_server(item.id.clone()).await?;
                }
                _ => {
                    // Unknown type – ignore but drop from manifest
                }
            }
        } else {
            remaining.push(item);
        }
    }

    manifest.items = remaining;
    write_security_packs_manifest(&manifest)?;
    Ok(())
}

