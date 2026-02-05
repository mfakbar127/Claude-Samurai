use serde_json::Value;
use std::path::PathBuf;

/// Get home directory
pub(crate) fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())
}

/// Ensure directory exists, creating if needed
pub(crate) fn ensure_dir(path: &std::path::Path, dir_name: &str) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Failed to create {}: {}", dir_name, e))?;
    Ok(())
}

/// Path to string (lossy)
pub(crate) fn path_to_string(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Read JSON file and extract mcpServers object
pub(crate) fn read_json_file_mcp_servers(
    file_path: &std::path::Path,
    file_name: &str,
) -> Result<serde_json::Map<String, Value>, String> {
    if !file_path.exists() {
        return Ok(serde_json::Map::new());
    }

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_name, e))?;
    let json_value: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", file_name, e))?;

    Ok(json_value
        .get("mcpServers")
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new))
}

/// Read JSON file and parse it
pub(crate) fn read_json_file(
    file_path: &std::path::Path,
    file_name: &str,
) -> Result<Value, String> {
    if !file_path.exists() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_name, e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", file_name, e))
}

/// Write JSON file with pretty formatting
pub(crate) fn write_json_file(
    file_path: &std::path::Path,
    value: &Value,
    file_name: &str,
) -> Result<(), String> {
    let json_content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize {}: {}", file_name, e))?;
    std::fs::write(file_path, json_content)
        .map_err(|e| format!("Failed to write {}: {}", file_name, e))?;
    Ok(())
}

/// Write serializable value as JSON file
pub(crate) fn write_json_file_serialize<T: serde::Serialize>(
    file_path: &std::path::Path,
    value: &T,
    file_name: &str,
) -> Result<(), String> {
    let json_value = serde_json::to_value(value)
        .map_err(|e| format!("Failed to serialize {}: {}", file_name, e))?;
    write_json_file(file_path, &json_value, file_name)
}

/// Extract string array from JSON value
pub(crate) fn extract_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(Vec::new)
}

/// Get absolute project path from ~/.claude.json projects[cwd]
pub(crate) fn get_project_path_from_claude_json(cwd: &str) -> Result<Option<PathBuf>, String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(None);
    }

    let json_value = read_json_file(&claude_json_path, ".claude.json")?;

    if let Some(projects) = json_value.get("projects").and_then(|p| p.as_object()) {
        if projects.contains_key(cwd) {
            return Ok(Some(PathBuf::from(cwd)));
        }
    }

    Ok(None)
}

/// Read from project-local ./.mcp.json (LOCAL scope - highest priority)
pub(crate) fn read_local_mcp_servers(
    project_path: &std::path::Path,
) -> Result<serde_json::Map<String, Value>, String> {
    let local_mcp_path = project_path.join(".mcp.json");
    read_json_file_mcp_servers(&local_mcp_path, "local .mcp.json")
}

/// Read from ~/.claude.json .projects[cwd].mcpServers (PROJECT scope)
pub(crate) fn read_project_mcp_servers(
    cwd: &str,
) -> Result<serde_json::Map<String, Value>, String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    let json_value = read_json_file(&claude_json_path, ".claude.json")?;

    Ok(json_value
        .get("projects")
        .and_then(|p| p.get(cwd))
        .and_then(|proj| proj.get("mcpServers"))
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new))
}

/// Read MCPJSON servers from ~/.mcp.json
pub(crate) fn read_mcpjson_servers(
    home_dir: &std::path::Path,
) -> Result<serde_json::Map<String, Value>, String> {
    let mcp_json_path = home_dir.join(".mcp.json");
    read_json_file_mcp_servers(&mcp_json_path, ".mcp.json")
}

/// Read Direct servers from ~/.claude.json
pub(crate) fn read_direct_servers(
    home_dir: &std::path::Path,
) -> Result<serde_json::Map<String, Value>, String> {
    let claude_json_path = home_dir.join(".claude.json");
    read_json_file_mcp_servers(&claude_json_path, ".claude.json")
}

/// Read disabledMcpServers from ~/.claude.json (root or .projects[cwd])
pub(crate) fn read_disabled_mcp_servers_from_claude_json(
    cwd: Option<&str>,
) -> Result<Vec<String>, String> {
    let home_dir = home_dir()?;
    let claude_json_path = home_dir.join(".claude.json");

    if !claude_json_path.exists() {
        return Ok(vec![]);
    }

    let json_value = read_json_file(&claude_json_path, ".claude.json")?;

    // Check project-specific disabledMcpServers first (higher priority)
    if let Some(cwd_str) = cwd {
        if let Some(disabled) = json_value
            .get("projects")
            .and_then(|p| p.get(cwd_str))
            .and_then(|proj| proj.get("disabledMcpServers"))
            .and_then(|v| v.as_array())
        {
            return Ok(disabled
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect());
        }
    }

    // Fallback to root level disabledMcpServers
    Ok(json_value
        .get("disabledMcpServers")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(Vec::new))
}
