pub mod engine;
pub mod model;

use model::MibModuleSummary;

pub fn bundled_module_summaries() -> Vec<MibModuleSummary> {
    ["SNMPv2-SMI", "SNMPv2-TC", "SNMPv2-CONF", "SNMPv2-MIB"]
        .into_iter()
        .map(|name| MibModuleSummary {
            name: name.to_owned(),
            source: "bundled".to_owned(),
            enabled: true,
        })
        .collect()
}
