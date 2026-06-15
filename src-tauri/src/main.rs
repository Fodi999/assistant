#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::{env, fs, path::PathBuf, process::Command};

#[derive(Debug, Serialize)]
struct UsbKeyStatus {
    found: bool,
    root: Option<String>,
    admin_tool: Option<String>,
    config: Option<String>,
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

#[tauri::command]
fn find_usb_key() -> UsbKeyStatus {
    let Some(root) = find_usb_key_root() else {
        return UsbKeyStatus {
            found: false,
            root: None,
            admin_tool: None,
            config: None,
        };
    };

    UsbKeyStatus {
        found: true,
        admin_tool: Some(root.join("bin/admin_tool").display().to_string()),
        config: Some(root.join("config/admin.env").display().to_string()),
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
