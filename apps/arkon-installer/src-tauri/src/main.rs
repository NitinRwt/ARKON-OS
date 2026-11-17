// ARKON Installer — Tauri host process (arkon-installer-app)
//
// Architecture:
//   arkon-installer (HTML/JS SPA, served by Tauri) ←→ [these Tauri commands] ←→ arkon-installd (D-Bus)
//
// BUILD NOTE: This crate requires WebKitGTK on Linux.  It is not part of the Cargo workspace and
// cannot be built on the Windows development host.  Uncomment the [tauri] dep in Cargo.toml when
// building on the Linux live-image host or in a container.
//
// SAFETY RULES enforced here:
//   API rule 1: No command accepts a shell fragment, kernel argument, or image reference from JS.
//   API rule 6: Callers supply only identifiers from the most-recent StorageReport.
//   The Tauri allowlist (tauri.conf.json) grants no shell-execute or file-system-write permission.

// When Tauri is not available (e.g. this Windows dev machine), provide stubs so `cargo check` passes.
#[cfg(not(feature = "tauri"))]
fn main() {
    eprintln!("arkon-installer-app: Tauri feature not enabled. Build on a Linux host with WebKitGTK.");
    eprintln!("Browser preview: open apps/arkon-installer/src/index.html directly in a browser.");
    std::process::exit(1);
}

#[cfg(feature = "tauri")]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd_get_state,
            cmd_scan_storage,
            cmd_create_plan,
            cmd_set_configuration,
            cmd_confirm,
            cmd_execute,
            cmd_cancel,
            cmd_get_failure,
            cmd_begin_cleanup,
            cmd_finish_cleanup,
            cmd_reboot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running arkon-installer-app");
}

// ── Tauri command stubs (compiled only with the tauri feature) ────────────
//
// Each command is a thin bridge: validate input types, forward to arkon-installd via D-Bus,
// return a typed JSON result.  Commands never construct OS commands, shell fragments, or
// kernel arguments — those are fixed inside arkon-installd.

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_get_state() -> serde_json::Value {
    // Forward: org.arkonos.Installer1.GetState()
    serde_json::json!({"state":"start"})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_scan_storage() -> serde_json::Value {
    // Forward: org.arkonos.Installer1.ScanStorage()
    serde_json::json!({"scan_id":"mock","disks":[]})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_create_plan(
    mode: String,          // one of erase_disk | free_space | dual_boot | reinstall_keep_home
    scan_id: String,       // must equal the current scan_id
    target_disk_id: String,// Disk.id from StorageReport
    encryption: bool,
) -> serde_json::Value {
    // Input validation: mode is a closed set, disk_id came from scan.
    let _ = (mode, scan_id, target_disk_id, encryption);
    serde_json::json!({"plan_id":"mock-plan","plan_hash":"mock-hash"})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_set_configuration(
    plan_id: String,
    username: String,       // validated against [a-z0-9_-]{1,32}
    full_name: String,
    password_hash: String,  // hashed by JS before crossing the bridge
    hostname: String,
    locale: String,
    keymap: String,
    timezone: String,
    language: String,
    mok_choice: String,     // accept | decline | not_applicable
) -> serde_json::Value {
    let _ = (plan_id, username, full_name, password_hash, hostname, locale, keymap, timezone, language, mok_choice);
    serde_json::json!({"ok":true})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_confirm(
    plan_id: String,
    plan_hash: String,
    phrase: Option<String>,  // only for destructive modes
    bitlocker_ack: bool,
) -> serde_json::Value {
    let _ = (plan_id, plan_hash, phrase, bitlocker_ack);
    serde_json::json!({"token":"mock-token"})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_execute(plan_id: String, token: String) -> serde_json::Value {
    let _ = (plan_id, token);
    serde_json::json!({"ok":true})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_cancel() -> serde_json::Value {
    serde_json::json!({"ok":true})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_get_failure() -> serde_json::Value {
    serde_json::json!({"class":"none"})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_begin_cleanup() -> serde_json::Value {
    serde_json::json!({"ok":true})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_finish_cleanup() -> serde_json::Value {
    serde_json::json!({"ok":true})
}

#[cfg(feature = "tauri")]
#[tauri::command]
fn cmd_reboot() -> serde_json::Value {
    serde_json::json!({"ok":true})
}
