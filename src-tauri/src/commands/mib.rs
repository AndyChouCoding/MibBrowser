use std::path::PathBuf;

use crate::mib::model::{MibModuleSummary, MibSnapshot};

#[tauri::command]
pub fn list_bundled_modules() -> Vec<MibModuleSummary> {
    crate::mib::bundled_module_summaries()
}

#[tauri::command]
pub async fn load_mib_files(paths: Vec<String>) -> Result<MibSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = paths.into_iter().map(PathBuf::from).collect();
        crate::mib::engine::load_files(paths).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("MIB worker failed: {error}"))?
}
