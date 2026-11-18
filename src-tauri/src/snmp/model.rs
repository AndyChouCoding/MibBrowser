use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpFeatures {
    versions: Vec<&'static str>,
    operations: Vec<&'static str>,
}

impl SnmpFeatures {
    pub fn planned_mvp() -> Self {
        Self {
            versions: vec!["v1", "v2c", "v3"],
            operations: vec!["get", "getNext", "walk", "bulkWalk", "set"],
        }
    }
}
