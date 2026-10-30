//! The binary and the add-on files that describe it agree.

const CONFIG: &str = include_str!("../../config.yaml");
const CHANGELOG: &str = include_str!("../../CHANGELOG.md");
const DOCKERFILE: &str = include_str!("../../Dockerfile");
const MAIN: &str = include_str!("../src/main.rs");

fn config_value(key: &str) -> Option<&'static str> {
    CONFIG
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{key}: ")))
        .map(|v| v.trim().trim_matches('"'))
}

#[test]
fn the_add_on_version_is_the_crate_version_and_has_a_changelog_entry() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(config_value("version"), Some(version));
    assert!(CHANGELOG.contains(&format!("## {version} - ")));
}

#[test]
fn the_ingress_port_is_the_port_the_panel_listens_on() {
    assert_eq!(config_value("ingress"), Some("true"));
    // 8099 is the Supervisor's default, which the add-on linter wants left implicit.
    let port = config_value("ingress_port").unwrap_or("8099");
    assert!(MAIN.contains(&format!("const PANEL: &str = \"0.0.0.0:{port}\";")));
}

/// The Supervisor reads every `config.{json,yaml,yml}` under the add-on folder as an add-on
/// manifest and logs a warning for each one that is not.
#[test]
fn the_add_on_folder_holds_one_config_file() {
    fn walk(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "target") {
                    walk(&path, found);
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| ["config.json", "config.yaml", "config.yml"].contains(&n))
            {
                found.push(path);
            }
        }
    }
    let addon = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    walk(&addon, &mut found);
    let found: Vec<_> = found
        .iter()
        .map(|p| {
            p.strip_prefix(&addon)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(found, ["config.yaml"]);
}

#[test]
fn the_image_runs_the_connector_with_cloudflared_where_it_looks() {
    assert!(DOCKERFILE.contains("CMD [\"/usr/local/bin/connector\"]"));
    assert!(DOCKERFILE.contains("/usr/local/bin/cloudflared"));
    assert!(MAIN.contains("const CLOUDFLARED: &str = \"/usr/local/bin/cloudflared\";"));
}
