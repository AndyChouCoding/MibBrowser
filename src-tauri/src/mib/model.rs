use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MibModuleSummary {
    pub name: String,
    pub source: String,
    pub enabled: bool,
}
