use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MibModuleSummary {
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MibNodeSummary {
    pub name: String,
    pub oid: String,
    pub kind: String,
    pub module: Option<String>,
    pub description: String,
    pub syntax: Option<String>,
    pub indexes: Vec<String>,
    pub enumeration: Vec<String>,
    pub access: Option<String>,
    pub status: Option<String>,
    pub children: Vec<MibNodeSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MibSnapshot {
    pub modules: Vec<MibModuleSummary>,
    pub roots: Vec<MibNodeSummary>,
    pub node_count: usize,
}
