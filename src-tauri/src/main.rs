#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::{env, fs, path::PathBuf, process::Command};

#[derive(Debug, Serialize)]
struct UsbStorageInfo {
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
    total_label: String,
    used_label: String,
    available_label: String,
}

#[derive(Debug, Serialize)]
struct UsbDataPaths {
    config: String,
    backups: String,
    exports: String,
    local_db: String,
    logs: String,
}

#[derive(Debug, Serialize)]
struct UsbKeyStatus {
    found: bool,
    root: Option<String>,
    admin_tool: Option<String>,
    config: Option<String>,
    storage: Option<UsbStorageInfo>,
    data_paths: Option<UsbDataPaths>,
}

#[derive(Debug, Serialize)]
struct AdminToolOutput {
    command: String,
    status: i32,
    stdout: String,
    stderr: String,
    key_root: String,
}

fn find_usb_key_root() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("ASSISTANT_ADMIN_KEY_ROOT") {
        let path = PathBuf::from(explicit);
        if path.join("bin/admin_tool").is_file() {
            return Some(path);
        }
    }

    let volumes = PathBuf::from("/Volumes");
    let entries = fs::read_dir(volumes).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("AssistantAdminKey");
        if candidate.join("bin/admin_tool").is_file() {
            return Some(candidate);
        }
    }
    None
}

fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.1} GB", value / GB)
    } else if value >= MB {
        format!("{:.0} MB", value / MB)
    } else {
        format!("{} B", bytes)
    }
}

fn usb_storage(root: &PathBuf) -> Option<UsbStorageInfo> {
    let output = Command::new("df").arg("-k").arg(root).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() < 4 {
        return None;
    }
    let total_bytes = columns.get(1)?.parse::<u64>().ok()?.saturating_mul(1024);
    let used_bytes = columns.get(2)?.parse::<u64>().ok()?.saturating_mul(1024);
    let available_bytes = columns.get(3)?.parse::<u64>().ok()?.saturating_mul(1024);
    Some(UsbStorageInfo {
        total_bytes,
        used_bytes,
        available_bytes,
        total_label: format_bytes(total_bytes),
        used_label: format_bytes(used_bytes),
        available_label: format_bytes(available_bytes),
    })
}

fn usb_data_paths(root: &PathBuf) -> UsbDataPaths {
    UsbDataPaths {
        config: root.join("config/admin.env").display().to_string(),
        backups: root.join("data/backups").display().to_string(),
        exports: root.join("data/exports").display().to_string(),
        local_db: root.join("data/local-db").display().to_string(),
        logs: root.join("logs").display().to_string(),
    }
}

#[tauri::command]
fn find_usb_key() -> UsbKeyStatus {
    let Some(root) = find_usb_key_root() else {
        return UsbKeyStatus {
            found: false,
            root: None,
            admin_tool: None,
            config: None,
            storage: None,
            data_paths: None,
        };
    };

    UsbKeyStatus {
        found: true,
        admin_tool: Some(root.join("bin/admin_tool").display().to_string()),
        config: Some(root.join("config/admin.env").display().to_string()),
        storage: usb_storage(&root),
        data_paths: Some(usb_data_paths(&root)),
        root: Some(root.display().to_string()),
    }
}

fn load_env_file(root: &PathBuf, command: &mut Command) {
    let path = root.join("config/admin.env");
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        command.env(key.trim(), value.trim());
    }
}

#[tauri::command]
fn run_admin_tool(args: Vec<String>) -> Result<AdminToolOutput, String> {
    if args.is_empty() {
        return Err("admin_tool args are required".to_string());
    }

    let root = find_usb_key_root().ok_or_else(|| {
        "AssistantAdminKey USB not found. Insert the flash drive or set ASSISTANT_ADMIN_KEY_ROOT.".to_string()
    })?;
    let tool = root.join("bin/admin_tool");
    if !tool.is_file() {
        return Err(format!("admin_tool not found at {}", tool.display()));
    }

    let mut command = Command::new(&tool);
    command.args(&args).current_dir(&root);
    load_env_file(&root, &mut command);

    let output = command
        .output()
        .map_err(|err| format!("Failed to run admin_tool: {err}"))?;

    Ok(AdminToolOutput {
        command: format!("{} {}", tool.display(), args.join(" ")),
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        key_root: root.display().to_string(),
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![find_usb_key, run_admin_tool])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
