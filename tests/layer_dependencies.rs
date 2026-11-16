//! Guardrails for direct layer dependencies; declarations and comments are not a substitute for a full Rust module graph analysis.
use std::{fs, path::Path};
fn check_dir(path: &Path, layer: &str, allowed: Option<&str>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("._"))
        {
            continue;
        }
        if path.is_dir() {
            check_dir(&path, layer, allowed);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for target in ["application", "services", "repository", "models"] {
            if Some(target) != allowed {
                let reference = format!("crate::{target}");
                assert!(
                    !source.contains(&reference),
                    "{} in {layer} must not reference {reference}",
                    path.display()
                );
            }
        }
        assert!(
            !source.contains("super::super::"),
            "{} bypasses layer boundaries",
            path.display()
        );
    }
}
#[test]
fn layers_reference_only_the_next_lower_layer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (layer, allowed) in [
        ("application", Some("services")),
        ("services", Some("repository")),
        ("repository", Some("models")),
        ("models", None),
    ] {
        check_dir(&root.join(layer), layer, allowed);
    }
}
