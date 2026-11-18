use crate::mib::model::MibModuleSummary;

#[tauri::command]
pub fn list_bundled_modules() -> Vec<MibModuleSummary> {
    crate::mib::bundled_module_summaries()
}
