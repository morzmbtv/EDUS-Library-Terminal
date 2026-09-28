//! Independently installed, hash-checked frontend releases. No embedded kiosk UI.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};
pub const API_VERSION: &str = "1";
pub const FRONTEND_VERSION: &str = "2.0.0-rc.1";
#[derive(Deserialize)]
struct Pointer {
    build_hash: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deployment {
    schema_version: u32,
    product: String,
    frontend_path: String,
}
/// Only the administrator-installed fixed sibling layout is accepted. A malformed
/// portable configuration never silently falls back to another installation.
pub fn deployment_root(executable: &Path) -> Result<Option<PathBuf>, &'static str> {
    let Some(binary_dir) = executable.parent() else {
        return Ok(None);
    };
    if !binary_dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("backend"))
    {
        return Ok(None);
    }
    let Some(root) = binary_dir.parent() else {
        return Ok(None);
    };
    let config = root.join("deployment.json");
    no_links(&config)?;
    let metadata = match std::fs::metadata(&config) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("DEPLOYMENT_CONFIG_INVALID"),
    };
    if !metadata.is_file() || metadata.len() > 4096 {
        return Err("DEPLOYMENT_CONFIG_INVALID");
    }
    let bytes = std::fs::read(&config).map_err(|_| "DEPLOYMENT_CONFIG_INVALID")?;
    let deployment: Deployment =
        serde_json::from_slice(&bytes).map_err(|_| "DEPLOYMENT_CONFIG_INVALID")?;
    if deployment.schema_version != 1
        || deployment.product != "EDUS-Library-Portable"
        || deployment.frontend_path != "frontend"
    {
        return Err("DEPLOYMENT_CONFIG_INVALID");
    }
    let frontend = root.join("frontend");
    no_links(&frontend)?;
    Ok(Some(frontend))
}
#[derive(Deserialize)]
pub struct Manifest {
    pub frontend_version: String,
    pub required_local_api_version: String,
    pub build_hash: String,
    pub build_time: String,
    pub files: Vec<File>,
}
#[derive(Deserialize)]
pub struct File {
    pub path: String,
    pub sha256: String,
}
pub struct Release {
    root: PathBuf,
    files: HashMap<String, String>,
}
fn digest_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.contains(':')
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn no_links(path: &Path) -> Result<(), &'static str> {
    for part in path.ancestors() {
        let m = match std::fs::symlink_metadata(part) {
            Ok(m) => m,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("FRONTEND_UNSAFE_PATH"),
        };
        {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err("FRONTEND_UNSAFE_PATH");
                }
            }
            if m.file_type().is_symlink() {
                return Err("FRONTEND_UNSAFE_PATH");
            }
        }
    }
    Ok(())
}
pub fn load(base: &Path) -> Result<Release, &'static str> {
    load_selected(base, "active.json")
}
fn load_selected(base: &Path, pointer_file: &str) -> Result<Release, &'static str> {
    no_links(base)?;
    let root = if base.join("manifest.json").exists()
        || base.join("frontend-manifest.json").exists()
    {
        base.to_owned()
    } else {
        let bytes = std::fs::read(base.join(pointer_file)).map_err(|_| "FRONTEND_MISSING")?;
        if bytes.len() > 1024 {
            return Err("FRONTEND_INVALID");
        }
        let pointer: Pointer = serde_json::from_slice(&bytes).map_err(|_| "FRONTEND_INVALID")?;
        if !digest_valid(&pointer.build_hash) {
            return Err("FRONTEND_UNSAFE_PATH");
        }
        base.join("versions")
            .join(pointer.build_hash)
            .join("wwwroot")
    };
    no_links(&root)?;
    let manifest = if root.join("manifest.json").exists() {
        root.join("manifest.json")
    } else {
        root.join("frontend-manifest.json")
    };
    no_links(&manifest)?;
    let raw = std::fs::read(manifest).map_err(|_| "FRONTEND_MISSING")?;
    if raw.len() > 1024 * 1024 {
        return Err("FRONTEND_INVALID");
    }
    let m: Manifest = serde_json::from_slice(&raw).map_err(|_| "FRONTEND_INVALID")?;
    if m.required_local_api_version != API_VERSION || m.frontend_version != FRONTEND_VERSION {
        return Err("FRONTEND_INCOMPATIBLE");
    }
    if !digest_valid(&m.build_hash) || m.build_time.is_empty() || m.files.is_empty() {
        return Err("FRONTEND_INVALID");
    }
    if base != root
        && root
            .parent()
            .and_then(Path::file_name)
            .and_then(|p| p.to_str())
            != Some(m.build_hash.as_str())
    {
        return Err("FRONTEND_INVALID");
    }
    let mut files = HashMap::new();
    for file in m.files {
        if !relative(&file.path)
            || !digest_valid(&file.sha256)
            || files
                .insert(file.path, file.sha256.to_lowercase())
                .is_some()
        {
            return Err("FRONTEND_INVALID");
        }
    }
    if !files.contains_key("index.html") {
        return Err("FRONTEND_MISSING");
    }
    Ok(Release { root, files })
}
pub fn read_asset(base: &Path, path: &str) -> Result<Vec<u8>, &'static str> {
    let result = load(base)?.read(path);
    if result.as_ref().err() == Some(&"FRONTEND_ASSET_NOT_FOUND") && path.starts_with("assets/") {
        // An already loaded index may request a lazy chunk during atomic activation.
        // Only the preserved, independently verified previous release can satisfy it.
        return load_selected(base, "previous.json")?.read(path);
    }
    result
}
impl Release {
    pub fn read(&self, path: &str) -> Result<Vec<u8>, &'static str> {
        let expected = self.files.get(path).ok_or("FRONTEND_ASSET_NOT_FOUND")?;
        if !relative(path) {
            return Err("FRONTEND_UNSAFE_PATH");
        }
        let file = self.root.join(path);
        no_links(&file)?;
        let bytes = std::fs::read(file).map_err(|_| "FRONTEND_ASSET_NOT_FOUND")?;
        if format!("{:x}", Sha256::digest(&bytes)) != *expected {
            return Err("FRONTEND_HASH_MISMATCH");
        }
        Ok(bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_fixed_layout_and_manifest() {
        let d = tempfile::tempdir().unwrap();
        let binary = d.path().join("backend").join("EDUSLibraryService.exe");
        assert_eq!(deployment_root(&binary).unwrap(), None);
        std::fs::write(
            d.path().join("deployment.json"),
            r#"{"schema_version":1,"product":"EDUS-Library-Portable","frontend_path":"frontend"}"#,
        )
        .unwrap();
        let expected = d.path().join("frontend");
        assert_eq!(deployment_root(&binary).unwrap(), Some(expected.clone()));
        let old = release(d.path(), FRONTEND_VERSION);
        std::fs::rename(&old, &expected).unwrap();
        std::fs::rename(
            expected.join("frontend-manifest.json"),
            expected.join("manifest.json"),
        )
        .unwrap();
        assert_eq!(
            load(&expected).unwrap().read("index.html").unwrap(),
            b"EDUS"
        );
    }
    #[test]
    fn portable_rejects_unknown_schema_and_arbitrary_paths() {
        let d = tempfile::tempdir().unwrap();
        let binary = d.path().join("backend").join("EDUSLibraryService.exe");
        for path in ["../frontend", "C:\\other", "frontend/other", "Frontend"] {
            std::fs::write(d.path().join("deployment.json"), serde_json::json!({"schema_version":1,"product":"EDUS-Library-Portable","frontend_path":path}).to_string()).unwrap();
            assert_eq!(
                deployment_root(&binary).err(),
                Some("DEPLOYMENT_CONFIG_INVALID")
            );
        }
        std::fs::write(
            d.path().join("deployment.json"),
            r#"{"schema_version":2,"product":"EDUS-Library-Portable","frontend_path":"frontend"}"#,
        )
        .unwrap();
        assert!(deployment_root(&binary).is_err());
        std::fs::write(d.path().join("deployment.json"), "invalid").unwrap();
        assert!(deployment_root(&binary).is_err());
    }
    fn release(dir: &Path, version: &str) -> PathBuf {
        let root = dir.join("versions").join("a".repeat(64)).join("wwwroot");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("index.html"), "EDUS").unwrap();
        std::fs::write(root.join("frontend-manifest.json"),serde_json::json!({"frontend_version":version,"required_local_api_version":"1","build_hash":"a".repeat(64),"build_time":"2026-09-25T00:00:00Z","files":[{"path":"index.html","sha256":format!("{:x}",Sha256::digest(b"EDUS"))}]}).to_string()).unwrap();
        std::fs::write(
            dir.join("active.json"),
            serde_json::json!({"build_hash":"a".repeat(64)}).to_string(),
        )
        .unwrap();
        root
    }
    #[test]
    fn missing_and_independent_activation() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load(d.path()).err(), Some("FRONTEND_MISSING"));
        release(d.path(), FRONTEND_VERSION);
        assert_eq!(load(d.path()).unwrap().read("index.html").unwrap(), b"EDUS");
    }
    #[test]
    fn mismatch_and_tamper_fail_closed() {
        let d = tempfile::tempdir().unwrap();
        release(d.path(), "99.0.0");
        assert_eq!(load(d.path()).err(), Some("FRONTEND_INCOMPATIBLE"));
        let root = release(d.path(), FRONTEND_VERSION);
        std::fs::write(root.join("index.html"), "changed").unwrap();
        assert_eq!(
            load(d.path()).unwrap().read("index.html").err(),
            Some("FRONTEND_HASH_MISMATCH")
        );
    }
    #[test]
    fn pointer_and_asset_traversal_rejected() {
        let d = tempfile::tempdir().unwrap();
        release(d.path(), FRONTEND_VERSION);
        assert!(load(d.path()).unwrap().read("../secrets.dpapi").is_err());
        std::fs::write(d.path().join("active.json"), r#"{"build_hash":"../data"}"#).unwrap();
        assert_eq!(load(d.path()).err(), Some("FRONTEND_UNSAFE_PATH"));
    }
    #[test]
    fn atomic_activation_preserves_previous_lazy_asset() {
        let d = tempfile::tempdir().unwrap();
        let old = release(d.path(), FRONTEND_VERSION);
        std::fs::create_dir(old.join("assets")).unwrap();
        std::fs::write(old.join("assets/old-hash.js"), b"old chunk").unwrap();
        let mut m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(old.join("frontend-manifest.json")).unwrap())
                .unwrap();
        m["files"].as_array_mut().unwrap().push(serde_json::json!({"path":"assets/old-hash.js","sha256":format!("{:x}",Sha256::digest(b"old chunk"))}));
        std::fs::write(old.join("frontend-manifest.json"), m.to_string()).unwrap();
        std::fs::rename(d.path().join("active.json"), d.path().join("previous.json")).unwrap();
        let new = d
            .path()
            .join("versions")
            .join("b".repeat(64))
            .join("wwwroot");
        std::fs::create_dir_all(&new).unwrap();
        m["build_hash"] = serde_json::json!("b".repeat(64));
        m["files"].as_array_mut().unwrap().pop();
        std::fs::write(new.join("frontend-manifest.json"), m.to_string()).unwrap();
        std::fs::write(new.join("index.html"), b"EDUS").unwrap();
        std::fs::write(
            d.path().join("active.json"),
            serde_json::json!({"build_hash":"b".repeat(64)}).to_string(),
        )
        .unwrap();
        assert_eq!(
            read_asset(d.path(), "assets/old-hash.js").unwrap(),
            b"old chunk"
        );
        std::fs::write(old.join("assets/old-hash.js"), b"tampered").unwrap();
        assert_eq!(
            read_asset(d.path(), "assets/old-hash.js").err(),
            Some("FRONTEND_HASH_MISMATCH")
        );
        assert!(read_asset(d.path(), "../secrets.dpapi").is_err());
    }
}
