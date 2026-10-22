//! One-click registration of Pinboard in AI apps' MCP configuration files.
//! Only the `pinboard` entry is written; every other setting, comment and key order is kept.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    Codex,
    Claude,
    #[serde(rename = "claude_cli")]
    ClaudeCli,
    Antigravity,
    Gemini,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Registration {
    /// The entry starts this Pinboard with the current data folder.
    Registered,
    /// A `pinboard` entry exists but points elsewhere, e.g. after moving the app.
    Outdated,
    Missing,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub client: Client,
    pub path: String,
    pub registration: Registration,
}

const NAME: &str = "pinboard";
const BACKUP_SUFFIX: &str = "pinboard.bak";

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

/// Every file the client reads. Claude Desktop from the Microsoft Store keeps a virtualized copy.
pub fn config_paths(client: Client) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match client {
        Client::Codex => {
            let root = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".codex")));
            paths.extend(root.map(|r| r.join("config.toml")));
        }
        Client::Gemini => paths.extend(home().map(|h| h.join(".gemini").join("settings.json"))),
        Client::ClaudeCli => {
            let root = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from).or_else(home);
            paths.extend(root.map(|h| h.join(".claude.json")));
        }
        Client::Antigravity => paths.extend(home().map(|h| h.join(".gemini").join("config").join("mcp_config.json"))),
        Client::Claude => {
            #[cfg(windows)]
            {
                paths.extend(std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Claude").join("claude_desktop_config.json")));
                if let Some(packages) = std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Packages")) {
                    for entry in fs::read_dir(packages).into_iter().flatten().flatten() {
                        if entry.file_name().to_string_lossy().starts_with("Claude_") {
                            paths.push(entry.path().join("LocalCache").join("Roaming").join("Claude").join("claude_desktop_config.json"));
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            paths.extend(home().map(|h| h.join(".config").join("Claude").join("claude_desktop_config.json")));
        }
    }
    if client == Client::Claude { paths.extend(config_paths(Client::ClaudeCli)); }
    paths
}

fn is_toml(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "toml")
}

fn matches(command: Option<&str>, args: Option<Vec<String>>, exe: &str, expected: &[String]) -> Registration {
    match command {
        None => Registration::Missing,
        Some(c) if c == exe && args.as_deref() == Some(expected) => Registration::Registered,
        Some(_) => Registration::Outdated,
    }
}

pub fn toml_registration(text: &str, exe: &str, args: &[String]) -> Registration {
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else { return Registration::Missing };
    let entry = doc.get("mcp_servers").and_then(|s| s.get(NAME));
    let Some(entry) = entry else { return Registration::Missing };
    if entry.get("enabled").and_then(|v| v.as_bool()) == Some(false) || entry.get("url").is_some() {
        return Registration::Outdated;
    }
    let found = entry.get("args").and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect());
    matches(Some(entry.get("command").and_then(|c| c.as_str()).unwrap_or("")), found, exe, args)
}

pub fn json_registration(text: &str, exe: &str, args: &[String]) -> Registration {
    let Ok(root) = serde_json::from_str::<Value>(text) else { return Registration::Missing };
    let Some(entry) = root.get("mcpServers").and_then(|s| s.get(NAME)) else { return Registration::Missing };
    if entry.get("disabled").and_then(Value::as_bool) == Some(true)
        || entry.get("enabled").and_then(Value::as_bool) == Some(false)
        || entry.get("type").and_then(Value::as_str).is_some_and(|t| t != "stdio")
        || ["url", "serverUrl", "httpUrl"].iter().any(|k| entry.get(k).is_some()) {
        return Registration::Outdated;
    }
    let found = entry.get("args").and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect());
    matches(Some(entry.get("command").and_then(Value::as_str).unwrap_or("")), found, exe, args)
}

/// Adds or repoints `[mcp_servers.pinboard]`. Keys the user added to the entry (env, timeouts) stay.
pub fn upsert_toml(text: &str, exe: &str, args: &[String]) -> Result<String, String> {
    use toml_edit::{value, Array, DocumentMut, Item, Table};
    // Parser diagnostics can quote lines containing credentials from unrelated entries.
    let mut doc = text.parse::<DocumentMut>().map_err(|_| "Invalid TOML configuration".to_string())?;
    if !doc.contains_key("mcp_servers") {
        let mut servers = Table::new();
        servers.set_implicit(true);
        doc.insert("mcp_servers", Item::Table(servers));
    }
    let servers = doc["mcp_servers"].as_table_mut().ok_or("mcp_servers is not a table")?;
    if !servers.contains_key(NAME) {
        servers.insert(NAME, Item::Table(Table::new()));
    }
    let entry = servers[NAME].as_table_mut().ok_or("mcp_servers.pinboard is not a table")?;
    entry["command"] = value(exe);
    entry["args"] = value(args.iter().map(String::as_str).collect::<Array>());
    entry["enabled"] = value(true);
    entry.remove("url");
    if !entry.contains_key("startup_timeout_sec") { entry["startup_timeout_sec"] = value(15); }
    if !entry.contains_key("tool_timeout_sec") { entry["tool_timeout_sec"] = value(25); }
    Ok(doc.to_string())
}

/// Adds or repoints `mcpServers.pinboard`, keeping key order and other entries.
pub fn upsert_json(text: &str, exe: &str, args: &[String]) -> Result<String, String> {
    let mut root: Value = if text.trim().is_empty() { json!({}) } else { serde_json::from_str(text).map_err(|_| "Invalid JSON configuration".to_string())? };
    let object = root.as_object_mut().ok_or("the configuration is not a JSON object")?;
    let servers = object.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers.as_object_mut().ok_or("mcpServers is not a JSON object")?;
    match servers.get_mut(NAME).and_then(Value::as_object_mut) {
        Some(entry) => {
            entry.insert("command".into(), json!(exe));
            entry.insert("args".into(), json!(args));
        }
        None => { servers.insert(NAME.into(), json!({ "command": exe, "args": args })); }
    }
    let entry = servers[NAME].as_object_mut().unwrap();
    entry.insert("type".into(), json!("stdio"));
    if entry.contains_key("disabled") { entry.insert("disabled".into(), json!(false)); }
    if entry.contains_key("enabled") { entry.insert("enabled".into(), json!(true)); }
    for key in ["url", "serverUrl", "httpUrl"] { entry.remove(key); }
    Ok(serde_json::to_string_pretty(&root).map_err(|e| e.to_string())? + "\n")
}

fn read(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text.strip_prefix('\u{feff}').map(String::from).unwrap_or(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(tr!("설정 파일을 읽지 못했습니다: {e}", "Could not read the configuration file: {e}"; "无法读取配置文件：{e}", "設定ファイルを読み込めませんでした：{e}"; "Impossible de lire le fichier de configuration : {e}", "No se pudo leer el archivo de configuración: {e}")),
    }
}

pub fn status(client: Client, exe: &str, args: &[String]) -> ClientStatus {
    let paths = config_paths(client);
    let registration = paths.iter()
        .map(|p| {
            let text = read(p).unwrap_or_default();
            if is_toml(p) { toml_registration(&text, exe, args) } else { json_registration(&text, exe, args) }
        })
        .min_by_key(|r| match r { Registration::Missing => 0, Registration::Outdated => 1, Registration::Registered => 2 })
        .unwrap_or(Registration::Missing);
    let path = paths.first().map(|p| p.display().to_string()).unwrap_or_default();
    ClientStatus { client, path, registration }
}

/// Writes every configuration file of the client. The previous file is kept as `*.pinboard.bak`
/// and the new one replaces it atomically, so a parse error never truncates the user's settings.
pub fn register(client: Client, exe: &str, args: &[String]) -> Result<ClientStatus, String> {
    let paths = config_paths(client);
    if paths.is_empty() {
        return Err(tr!("설정 파일 위치를 찾지 못했습니다.", "Could not find the configuration folder."; "找不到配置文件位置。", "設定ファイルの場所が見つかりませんでした。"; "Impossible de trouver le dossier de configuration.", "No se encontró la carpeta de configuración."));
    }
    // Parse every location before changing any file (Desktop may have a Store copy).
    let mut prepared = Vec::new();
    for path in &paths {
        let text = read(path)?;
        let next = if is_toml(path) { upsert_toml(&text, exe, args) } else { upsert_json(&text, exe, args) }
            .map_err(|e| tr!("설정 파일 형식을 해석하지 못했습니다. ‘연결 설정 보기’로 직접 추가하세요: {e}", "Could not parse the configuration file. Add the entry manually from ‘Show connection settings’: {e}"; "无法解析配置文件。请通过“查看连接设置”手动添加：{e}", "設定ファイルを解析できませんでした。「接続設定を表示」から手動で追加してください：{e}"; "Impossible d’analyser le fichier de configuration. Ajoutez l’entrée manuellement depuis « Afficher les paramètres de connexion » : {e}", "No se pudo interpretar el archivo de configuración. Añade la entrada manualmente desde «Ver configuración de conexión»: {e}"))?;
        prepared.push((path, text, next));
    }
    for (path, text, next) in prepared {
        if next == text { continue; }
        // A running client can update its configuration while registration is being prepared.
        if read(path)? != text { return Err("Configuration changed during registration; try again.".into()); }
        let write = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
            if path.exists() {
                let previous = fs::read(path)?;
                crate::store::write_atomic(&path.with_extension(format!("{}.{BACKUP_SUFFIX}", path.extension().unwrap_or_default().to_string_lossy())), &previous)?;
            }
            crate::store::write_atomic(path, next.as_bytes())
        };
        write().map_err(|e| tr!("설정 파일을 저장하지 못했습니다: {e}", "Could not save the configuration file: {e}"; "无法保存配置文件：{e}", "設定ファイルを保存できませんでした：{e}"; "Impossible d’enregistrer le fichier de configuration : {e}", "No se pudo guardar el archivo de configuración: {e}"))?;
    }
    Ok(status(client, exe, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Apps\Pinboard\Pinboard.exe";
    fn args() -> Vec<String> { vec!["--mcp".into(), "--mcp-data-dir".into(), r"C:\Data\pinboard".into()] }

    #[test]
    fn toml_keeps_comments_and_other_servers_and_is_idempotent() {
        let original = "# my settings\nmodel = \"gpt\"\n\n[mcp_servers.github]\ncommand = \"gh\" # keep\n";
        let next = upsert_toml(original, EXE, &args()).unwrap();
        assert!(next.starts_with("# my settings\nmodel = \"gpt\"\n\n[mcp_servers.github]\ncommand = \"gh\" # keep\n"));
        assert!(next.contains("[mcp_servers.pinboard]"));
        assert!(!next.contains("[mcp_servers]\n"));
        assert_eq!(toml_registration(&next, EXE, &args()), Registration::Registered);
        assert_eq!(upsert_toml(&next, EXE, &args()).unwrap(), next);
        let parsed: toml_edit::DocumentMut = next.parse().unwrap();
        assert_eq!(parsed["mcp_servers"]["pinboard"]["startup_timeout_sec"].as_integer(), Some(15));
    }

    #[test]
    fn toml_repoints_moved_app_but_keeps_user_keys() {
        let old = "[mcp_servers.pinboard]\ncommand = \"D:\\\\old\\\\Pinboard.exe\"\nargs = []\ntool_timeout_sec = 60\n\n[mcp_servers.pinboard.env]\nA = \"1\"\n";
        assert_eq!(toml_registration(old, EXE, &args()), Registration::Outdated);
        let next = upsert_toml(old, EXE, &args()).unwrap();
        assert_eq!(toml_registration(&next, EXE, &args()), Registration::Registered);
        let parsed: toml_edit::DocumentMut = next.parse().unwrap();
        assert_eq!(parsed["mcp_servers"]["pinboard"]["tool_timeout_sec"].as_integer(), Some(60));
        assert_eq!(parsed["mcp_servers"]["pinboard"]["env"]["A"].as_str(), Some("1"));
    }

    #[test]
    fn json_keeps_key_order_and_other_servers() {
        let original = r#"{"theme":"dark","mcpServers":{"zeta":{"command":"z"}},"alpha":1}"#;
        let next = upsert_json(original, EXE, &args()).unwrap();
        let keys: Vec<_> = serde_json::from_str::<Value>(&next).unwrap().as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["theme", "mcpServers", "alpha"]);
        assert!(next.contains("\"zeta\""));
        assert_eq!(json_registration(&next, EXE, &args()), Registration::Registered);
        assert_eq!(upsert_json(&next, EXE, &args()).unwrap(), next);
        assert_eq!(json_registration(&upsert_json("", EXE, &args()).unwrap(), EXE, &args()), Registration::Registered);
    }

    #[test]
    fn register_backs_up_replaces_and_leaves_invalid_files_untouched() {
        // The only test that reads CODEX_HOME, so setting it here cannot race other tests.
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("CODEX_HOME", dir.path());
        let config = dir.path().join("config.toml");
        fs::write(&config, "model = \"gpt\"\n").unwrap();
        assert_eq!(register(Client::Codex, EXE, &args()).unwrap().registration, Registration::Registered);
        assert_eq!(fs::read_to_string(dir.path().join("config.toml.pinboard.bak")).unwrap(), "model = \"gpt\"\n");
        let written = fs::read_to_string(&config).unwrap();
        assert!(written.starts_with("model = \"gpt\"\n") && written.contains("[mcp_servers.pinboard]"));
        assert!(!dir.path().join("config.pinboard.tmp").exists());
        let modified = fs::metadata(&config).unwrap().modified().unwrap();
        register(Client::Codex, EXE, &args()).unwrap();
        assert_eq!(fs::metadata(&config).unwrap().modified().unwrap(), modified);
        fs::write(&config, "[broken").unwrap();
        assert!(register(Client::Codex, EXE, &args()).is_err());
        assert_eq!(fs::read_to_string(&config).unwrap(), "[broken");
        std::env::remove_var("CODEX_HOME");
    }

    #[test]
    fn invalid_files_are_rejected_instead_of_overwritten() {
        assert!(upsert_json("{ // comment\n}", EXE, &args()).is_err());
        assert!(upsert_json("[]", EXE, &args()).is_err());
        assert!(upsert_toml("mcp_servers = 3", EXE, &args()).is_err());
        assert!(upsert_toml("[broken", EXE, &args()).is_err());
    }

    #[test]
    fn explicit_connect_reenables_local_servers_and_removes_remote_transport() {
        let valid = upsert_toml("", EXE, &args()).unwrap();
        let disabled = valid.replace("enabled = true", "enabled = false");
        assert_eq!(toml_registration(&disabled, EXE, &args()), Registration::Outdated);
        let repaired = upsert_toml(&disabled, EXE, &args()).unwrap();
        assert_eq!(toml_registration(&repaired, EXE, &args()), Registration::Registered);
        let remote = format!("{valid}\nurl = \"https://example.invalid/mcp\"\n");
        assert_eq!(toml_registration(&remote, EXE, &args()), Registration::Outdated);
        assert!(!upsert_toml(&remote, EXE, &args()).unwrap().contains("url ="));

        for extra in [json!({"disabled": true}), json!({"enabled": false}), json!({"type": "http", "serverUrl": "https://example.invalid"})] {
            let mut root: Value = serde_json::from_str(&upsert_json("", EXE, &args()).unwrap()).unwrap();
            root["mcpServers"][NAME].as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            root["mcpServers"][NAME]["env"] = json!({"KEEP": "yes"});
            assert_eq!(json_registration(&root.to_string(), EXE, &args()), Registration::Outdated);
            let next = upsert_json(&root.to_string(), EXE, &args()).unwrap();
            assert_eq!(json_registration(&next, EXE, &args()), Registration::Registered);
            let parsed: Value = serde_json::from_str(&next).unwrap();
            assert_eq!(parsed["mcpServers"][NAME]["env"]["KEEP"], "yes");
            assert_eq!(parsed["mcpServers"][NAME]["type"], "stdio");
        }
    }

    #[test]
    fn new_client_identifiers_round_trip() {
        for (id, client) in [("claude_cli", Client::ClaudeCli), ("antigravity", Client::Antigravity)] {
            assert_eq!(serde_json::from_str::<Client>(&format!("\"{id}\"")).unwrap(), client);
            assert_eq!(serde_json::to_string(&client).unwrap(), format!("\"{id}\""));
        }
    }
}
