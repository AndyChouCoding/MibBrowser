mod commands;
mod mib;
mod snmp;

#[tauri::command]
async fn export_results(
    app: tauri::AppHandle,
    format: String,
    content: String,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    if !matches!(format.as_str(), "csv" | "json") {
        return Err("Unsupported export format".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let choice = app
            .dialog()
            .file()
            .add_filter(format.to_uppercase(), &[&format])
            .set_file_name(format!("snmp-results.{format}"))
            .blocking_save_file();
        let Some(file) = choice else { return Ok(false) };
        std::fs::write(file.into_path().map_err(|e| e.to_string())?, content)
            .map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn desktop_builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(commands::write::PendingSet::default())
        .manage(commands::snmp::ActiveQuery::default())
        .invoke_handler(tauri::generate_handler![
            commands::health::health_check,
            export_results,
            commands::write::prepare_set,
            commands::write::finish_set,
            commands::storage::save_profile,
            commands::storage::load_profile,
            commands::storage::profile_names,
            commands::storage::record_query,
            commands::storage::query_history,
            commands::mib::list_bundled_modules,
            commands::mib::load_mib_files,
            commands::snmp::supported_snmp_features,
            commands::snmp::validate_snmp_profile,
            commands::snmp::query_device,
            commands::snmp::cancel_query,
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    desktop_builder()
        .run(tauri::generate_context!())
        .expect("failed to run MIB Browser");
}
