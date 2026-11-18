use crate::snmp::model::SnmpFeatures;

#[tauri::command]
pub fn supported_snmp_features() -> SnmpFeatures {
    SnmpFeatures::planned_mvp()
}
