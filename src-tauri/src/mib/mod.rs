pub mod engine;
pub mod model;

use model::MibModuleSummary;

pub fn bundled_module_summaries() -> Vec<MibModuleSummary> {
    mib_rs::lower::base_modules::base_module_names()
        .iter()
        .copied()
        .map(|name| MibModuleSummary {
            name: name.to_owned(),
            source: "bundled".to_owned(),
            enabled: true,
        })
        .collect()
}
