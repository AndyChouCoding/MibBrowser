use super::snmp::ReadCredentials;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

const SERVICE: &str = "com.mibbrowser.desktop.profiles";
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL, host TEXT NOT NULL, port INTEGER NOT NULL,
    timeout_ms INTEGER NOT NULL, retries INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY, created TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    host TEXT NOT NULL, oid TEXT NOT NULL, operation TEXT NOT NULL, result_count INTEGER NOT NULL
);";

fn database(app: &tauri::AppHandle) -> Result<Connection, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Profile directory unavailable")?;
    std::fs::create_dir_all(&dir).map_err(|_| "Cannot create profile directory")?;
    let db = Connection::open(dir.join("workspace.sqlite"))
        .map_err(|_| "Cannot open workspace database")?;
    db.execute_batch(SCHEMA)
        .map_err(|_| "Cannot initialize workspace database")?;
    Ok(db)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    name: String,
    host: String,
    port: u16,
    timeout_ms: u64,
    retries: u8,
}

#[tauri::command]
pub async fn save_profile(
    app: tauri::AppHandle,
    settings: Settings,
    credentials: ReadCredentials,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if settings.name.trim().is_empty()
            || settings.host.trim().is_empty()
            || settings.port == 0
            || !(100..=120000).contains(&settings.timeout_ms)
            || settings.retries > 10
        {
            return Err("Enter a profile name and valid connection settings".into());
        }
        credentials.clone().into_auth()?;
        let mut db = database(&app)?;
        let tx = db.transaction().map_err(|_| "Workspace busy")?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO profiles VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                id,
                settings.name.trim(),
                settings.host.trim(),
                settings.port,
                settings.timeout_ms,
                settings.retries
            ],
        )
        .map_err(|_| "Cannot save profile. Choose an unused name.")?;
        let entry =
            keyring::Entry::new(SERVICE, &id).map_err(|_| "Credential store unavailable")?;
        let secret = serde_json::to_string(&credentials).map_err(|_| "Invalid credentials")?;
        entry
            .set_password(&secret)
            .map_err(|_| "Could not save credentials to the system credential store")?;
        if tx.commit().is_err() {
            entry.delete_credential().map_err(|_| {
                "Profile save failed. Remove the unused credential from the system store."
            })?;
            return Err("Could not commit profile".into());
        }
        Ok(())
    })
    .await
    .map_err(|_| "Profile worker unavailable")?
}

#[tauri::command]
pub fn profile_names(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let db = database(&app)?;
    let mut statement = db
        .prepare("SELECT name FROM profiles ORDER BY name COLLATE NOCASE")
        .map_err(|_| "Cannot list profiles")?;
    let rows = statement
        .query_map([], |row| row.get(0))
        .map_err(|_| "Cannot list profiles")?;
    rows.collect::<Result<_, _>>()
        .map_err(|_| "Cannot read profile names".into())
}

#[tauri::command]
pub async fn load_profile(
    app: tauri::AppHandle,
    name: String,
) -> Result<(Settings, ReadCredentials), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = database(&app)?;
        let (id, settings): (String, Settings) = db
            .query_row(
                "SELECT id, host, port, timeout_ms, retries FROM profiles WHERE name=?1",
                [&name],
                |row| {
                    Ok((
                        row.get(0)?,
                        Settings {
                            name: name.clone(),
                            host: row.get(1)?,
                            port: row.get(2)?,
                            timeout_ms: row.get(3)?,
                            retries: row.get(4)?,
                        },
                    ))
                },
            )
            .map_err(|_| "Profile unavailable")?;
        let secret = keyring::Entry::new(SERVICE, &id)
            .and_then(|e| e.get_password())
            .map_err(|_| "Cannot read credentials from the system credential store")?;
        Ok((
            settings,
            serde_json::from_str(&secret).map_err(|_| "Stored credentials are invalid")?,
        ))
    })
    .await
    .map_err(|_| "Profile worker unavailable")?
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    host: String,
    oid: String,
    operation: String,
    result_count: usize,
}

#[tauri::command]
pub fn record_query(app: tauri::AppHandle, item: HistoryItem) -> Result<(), String> {
    database(&app)?
        .execute(
            "INSERT INTO history(host,oid,operation,result_count) VALUES (?1,?2,?3,?4)",
            params![item.host, item.oid, item.operation, item.result_count],
        )
        .map_err(|_| "Cannot record query history")?;
    Ok(())
}

#[tauri::command]
pub fn query_history(app: tauri::AppHandle) -> Result<Vec<(String, HistoryItem)>, String> {
    let db = database(&app)?;
    let mut stmt = db.prepare("SELECT created,host,oid,operation,result_count FROM history ORDER BY id DESC LIMIT 100").map_err(|_| "Cannot load history")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                HistoryItem {
                    host: r.get(1)?,
                    oid: r.get(2)?,
                    operation: r.get(3)?,
                    result_count: r.get(4)?,
                },
            ))
        })
        .map_err(|_| "Cannot load history")?;
    rows.collect::<Result<_, _>>()
        .map_err(|_| "Cannot read history".into())
}
