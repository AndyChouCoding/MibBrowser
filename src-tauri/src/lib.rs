mod commands;
mod mib;
mod snmp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::health::health_check,
            commands::mib::list_bundled_modules,
            commands::mib::load_mib_files,
            commands::snmp::supported_snmp_features,
            commands::snmp::validate_snmp_profile,
            commands::snmp::query_v2c,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run MIB Browser");
}
