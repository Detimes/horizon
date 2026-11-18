//! A player's JSON file (advancements) replaced atomically: written beside the old one, then moved over it.
use std::path::Path;

/// `json` as pretty text in `path`, through a sibling `<file>.tmp`: a failed write (a full disk) leaves the
/// old file whole and the temporary one removed.
pub fn write_json(path: &Path, json: &serde_json::Value) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(json).map_err(std::io::Error::other)?;
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = std::path::PathBuf::from(temporary);
    let written =
        std::fs::write(&temporary, text).and_then(|()| std::fs::rename(&temporary, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// [`write_json`], a failure logged: what a save that has nowhere to report to does.
pub fn save_json(path: &Path, json: &serde_json::Value) {
    if let Err(e) = write_json(path, json) {
        log::warn!("could not save {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_replaces_the_file_whole_and_a_failed_one_leaves_it() {
        let dir = std::env::temp_dir().join(format!("atomic-json-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("p.json");
        write_json(&path, &serde_json::json!({"a": 1})).unwrap();
        assert!(!dir.join("p.json.tmp").exists());
        let before = std::fs::read_to_string(&path).unwrap();
        // the temporary name is a directory, so it cannot be written: the old file stays
        std::fs::create_dir(dir.join("p.json.tmp")).unwrap();
        assert!(write_json(&path, &serde_json::json!({"a": 2})).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
