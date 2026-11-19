use std::path::PathBuf;

use mib_rs::{source, Loader, Mib};
use thiserror::Error;

use super::model::{MibModuleSummary, MibNodeSummary, MibSnapshot};

const MAX_IMPORT_FILES: usize = 256;
const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_SIZE: u64 = 64 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &["mib", "smi", "txt", "my"];

#[derive(Debug, Error)]
pub enum MibEngineError {
    #[error("at most {MAX_IMPORT_FILES} MIB files can be imported at once")]
    TooManyFiles,
    #[error("MIB path is not a regular file: {0}")]
    NotAFile(String),
    #[error("unsupported MIB file extension: {0}")]
    UnsupportedExtension(String),
    #[error("MIB file exceeds the 16 MiB limit: {0}")]
    FileTooLarge(String),
    #[error("selected MIB files exceed the 64 MiB total limit")]
    TotalTooLarge,
    #[error("MIB source error: {0}")]
    Source(#[from] std::io::Error),
    #[error("MIB load failed: {0}")]
    Load(String),
}

pub fn load_files(paths: Vec<PathBuf>) -> Result<MibSnapshot, MibEngineError> {
    if paths.len() > MAX_IMPORT_FILES {
        return Err(MibEngineError::TooManyFiles);
    }

    validate_paths(&paths)?;

    let loader = if paths.is_empty() {
        Loader::new().modules(
            mib_rs::lower::base_modules::base_module_names()
                .iter()
                .copied(),
        )
    } else {
        Loader::new().source(source::files(paths)?)
    };
    let mib = loader
        .load()
        .map_err(|error| MibEngineError::Load(error.to_string()))?;

    if let Some(issue) = mib.diagnostics().iter().find(|issue| {
        matches!(
            issue.code,
            mib_rs::types::DiagCode::ImportModuleNotFound | mib_rs::types::DiagCode::ImportNotFound
        )
    }) {
        return Err(MibEngineError::Load(issue.to_string()));
    }
    Ok(snapshot(&mib))
}

fn validate_paths(paths: &[PathBuf]) -> Result<(), MibEngineError> {
    let mut total_size = 0_u64;

    for path in paths {
        let display = path.display().to_string();
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(MibEngineError::NotAFile(display));
        }

        if let Some(extension) = path.extension() {
            let extension = extension.to_string_lossy();
            if !ALLOWED_EXTENSIONS
                .iter()
                .any(|allowed| extension.eq_ignore_ascii_case(allowed))
            {
                return Err(MibEngineError::UnsupportedExtension(display));
            }
        }

        if metadata.len() > MAX_FILE_SIZE {
            return Err(MibEngineError::FileTooLarge(display));
        }
        total_size = total_size.saturating_add(metadata.len());
        if total_size > MAX_TOTAL_SIZE {
            return Err(MibEngineError::TotalTooLarge);
        }
    }

    Ok(())
}

fn snapshot(mib: &Mib) -> MibSnapshot {
    let modules = mib
        .modules()
        .map(|module| MibModuleSummary {
            name: module.name().to_owned(),
            source: if module.is_base() {
                "bundled"
            } else {
                "imported"
            }
            .to_owned(),
            enabled: true,
        })
        .collect();

    let roots = mib.root_node().children().map(node_summary).collect();
    let node_count = mib.nodes().count();

    MibSnapshot {
        modules,
        roots,
        node_count,
    }
}

fn node_summary(node: mib_rs::Node<'_>) -> MibNodeSummary {
    MibNodeSummary {
        name: node.name().to_owned(),
        oid: node.oid().to_string(),
        kind: format!("{:?}", node.kind()).to_lowercase(),
        module: node.module().map(|module| module.name().to_owned()),
        description: node.description().to_owned(),
        syntax: node
            .object()
            .and_then(|obj| obj.ty())
            .map(|ty| ty.name().to_owned()),
        access: node.object().map(|obj| obj.access().to_string()),
        status: node.status().map(|status| status.to_string()),
        children: node.children().map(node_summary).collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const TEST_MIB: &[u8] = br#"
TEST-BROWSER-MIB DEFINITIONS ::= BEGIN
IMPORTS
    MODULE-IDENTITY, OBJECT-TYPE, enterprises
        FROM SNMPv2-SMI
    DisplayString
        FROM SNMPv2-TC;

testBrowserMib MODULE-IDENTITY
    LAST-UPDATED "202611180000Z"
    ORGANIZATION "MIB Browser"
    CONTACT-INFO "MIB Browser"
    DESCRIPTION "MIB Browser test module."
    ::= { enterprises 424242 }

testDeviceName OBJECT-TYPE
    SYNTAX DisplayString
    MAX-ACCESS read-only
    STATUS current
    DESCRIPTION "A device name used for parser tests."
    ::= { testBrowserMib 1 }

END
"#;

    #[test]
    fn creates_serializable_snapshot_from_resolved_mib() {
        let mib = Loader::new()
            .source(source::memory("TEST-BROWSER-MIB", TEST_MIB))
            .modules(["TEST-BROWSER-MIB"])
            .load()
            .expect("test MIB should resolve");

        let snapshot = snapshot(&mib);
        assert!(snapshot.node_count > 0);
        assert!(snapshot
            .modules
            .iter()
            .any(|module| module.name == "TEST-BROWSER-MIB" && module.source == "imported"));
        assert!(snapshot
            .roots
            .iter()
            .flat_map(flatten)
            .any(|node| node.name == "testDeviceName"));
    }

    #[test]
    fn loads_a_mib_from_a_real_file() {
        let path = temporary_path("mib");
        fs::write(&path, TEST_MIB).expect("fixture should be written");

        let result = load_files(vec![path.clone()]);
        fs::remove_file(path).expect("fixture should be removed");

        let snapshot = result.expect("file should load");
        assert!(snapshot
            .modules
            .iter()
            .any(|module| module.name == "TEST-BROWSER-MIB"));
    }

    #[test]
    fn loads_foundation_tree_without_files() {
        let workspace = load_files(Vec::new()).unwrap();
        assert!(workspace.modules.iter().any(|m| m.name == "SNMPv2-SMI"));
        assert!(workspace
            .roots
            .iter()
            .flat_map(flatten)
            .any(|n| n.name == "enterprises" && n.oid == "1.3.6.1.4.1"));
    }

    #[test]
    fn identifies_missing_import_module() {
        let path = temporary_path("mib");
        let source = String::from_utf8_lossy(TEST_MIB).replace("FROM SNMPv2-TC", "FROM ABSENT-TC");
        fs::write(&path, source).unwrap();
        let error = load_files(vec![path.clone()]).unwrap_err();
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains("ABSENT-TC"), "{error}");
    }

    #[test]
    fn rejects_unsupported_file_extensions() {
        let path = temporary_path("json");
        fs::write(&path, TEST_MIB).expect("fixture should be written");

        let result = load_files(vec![path.clone()]);
        fs::remove_file(path).expect("fixture should be removed");

        assert!(matches!(
            result,
            Err(MibEngineError::UnsupportedExtension(_))
        ));
    }

    fn temporary_path(extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mib-browser-{}-{}.{}",
            std::process::id(),
            std::thread::current()
                .name()
                .unwrap_or("test")
                .replace(':', "_"),
            extension
        ))
    }

    fn flatten(node: &MibNodeSummary) -> Vec<&MibNodeSummary> {
        let mut nodes = vec![node];
        for child in &node.children {
            nodes.extend(flatten(child));
        }
        nodes
    }
}
