use crate::snmp::model::{ProfileInput, SnmpFeatures, ValidatedProfile};

#[tauri::command]
pub fn supported_snmp_features() -> SnmpFeatures {
    SnmpFeatures::planned_mvp()
}

#[tauri::command]
pub fn validate_snmp_profile(profile: ProfileInput) -> Result<ValidatedProfile, String> {
    profile.validate().map_err(|error| error.to_string())
}
