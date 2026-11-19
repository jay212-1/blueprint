//! The project file: JSON with a schema version, migrated step by step on
//! load, written atomically (a crash never leaves half a file), the previous
//! file kept in `backups/`, and an autosave next to it for recovery.

use crate::model::{Project, SCHEMA_VERSION};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const EXTENSION: &str = "blueprint";
/// Backups kept per project in `backups/`.
pub const MAX_BACKUPS: usize = 20;

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("not a Blueprint project: {0}")]
    Parse(String),
    #[error("this project was made by a newer Blueprint (schema {0}); please update Blueprint")]
    TooNew(u32),
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> FileError + '_ {
    move |source| FileError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Bring an older document up to [`SCHEMA_VERSION`], one step at a time.
pub fn migrate(mut doc: Value) -> Result<Value, FileError> {
    let version = doc
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    if version > SCHEMA_VERSION {
        return Err(FileError::TooNew(version));
    }
    // Steps go here: `if version < 2 { migrate_v1_to_v2(&mut doc) }` and so on.
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("schema_version".into(), SCHEMA_VERSION.into());
    }
    Ok(doc)
}

pub fn parse(text: &str) -> Result<Project, FileError> {
    let doc: Value = serde_json::from_str(text).map_err(|e| FileError::Parse(e.to_string()))?;
    if !doc.is_object() {
        return Err(FileError::Parse("expected a JSON object".into()));
    }
    let doc = migrate(doc)?;
    let mut p: Project =
        serde_json::from_value(doc).map_err(|e| FileError::Parse(e.to_string()))?;
    repair_ids(&mut p);
    Ok(p)
}

/// `next_id` above every id in use (a hand-edited or merged file may be off).
fn repair_ids(p: &mut Project) {
    let max = p
        .cast
        .iter()
        .map(|x| x.id)
        .chain(p.roles.iter().map(|x| x.id))
        .chain(p.scenes.iter().map(|x| x.id))
        .chain(p.transmitters.iter().map(|x| x.id))
        .chain(p.capsules.iter().map(|x| x.id))
        .chain(p.locations.iter().map(|x| x.id))
        .max()
        .unwrap_or(0);
    p.next_id = p.next_id.max(max + 1);
}

pub fn to_string(p: &Project) -> Result<String, FileError> {
    serde_json::to_string_pretty(p).map_err(|e| FileError::Parse(e.to_string()))
}

pub fn load(path: &Path) -> Result<Project, FileError> {
    let text = fs::read_to_string(path).map_err(io_err(path))?;
    parse(&text)
}

/// Write `data` to `path` atomically: temp file in the same folder, fsync,
/// rename.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), FileError> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(io_err(dir))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp).map_err(io_err(&tmp))?;
        f.write_all(data).map_err(io_err(&tmp))?;
        f.sync_all().map_err(io_err(&tmp))?;
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(io_err(path)(e));
    }
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Save; the previous file goes to `backups/` first, the autosave is removed.
pub fn save(p: &Project, path: &Path) -> Result<(), FileError> {
    let text = to_string(p)?;
    if path.exists() {
        backup(path)?;
    }
    atomic_write(path, text.as_bytes())?;
    let _ = fs::remove_file(autosave_path(path));
    Ok(())
}

fn backup(path: &Path) -> Result<(), FileError> {
    let dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    fs::create_dir_all(&dir).map_err(io_err(&dir))?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let mut target = dir.join(format!("{stem}.{}.{EXTENSION}", unix_now()));
    let mut n = 1;
    while target.exists() {
        target = dir.join(format!("{stem}.{}-{n}.{EXTENSION}", unix_now()));
        n += 1;
    }
    fs::copy(path, &target).map_err(io_err(&target))?;
    let prefix = format!("{stem}.");
    let mut olds: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(io_err(&dir))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(EXTENSION))
        })
        .collect();
    olds.sort();
    while olds.len() > MAX_BACKUPS {
        let _ = fs::remove_file(olds.remove(0));
    }
    Ok(())
}

pub fn autosave_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".autosave");
    PathBuf::from(os)
}

pub fn autosave(p: &Project, path: &Path) -> Result<(), FileError> {
    atomic_write(&autosave_path(path), to_string(p)?.as_bytes())
}

/// An autosave newer than the project file (Blueprint did not close cleanly
/// with unsaved changes).
pub fn recoverable(path: &Path) -> Option<Project> {
    let auto = autosave_path(path);
    let auto_time = fs::metadata(&auto).and_then(|m| m.modified()).ok()?;
    if let Ok(saved) = fs::metadata(path).and_then(|m| m.modified()) {
        if saved > auto_time {
            return None;
        }
    }
    load(&auto).ok()
}

/// `path` with the project extension.
pub fn with_extension(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    if p.extension().and_then(|e| e.to_str()) != Some(EXTENSION) {
        let mut os = p.as_os_str().to_os_string();
        os.push(".");
        os.push(EXTENSION);
        p = PathBuf::from(os);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Person, SCHEMA_VERSION};

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("show.blueprint");
        let mut p = Project::default();
        p.meta.production = "Test".into();
        let id = p.alloc_id();
        p.cast.push(Person {
            id,
            name: "Anna".into(),
            ..Person::default()
        });
        save(&p, &path).unwrap();
        assert_eq!(load(&path).unwrap(), p);
        // A second save keeps the first in backups/.
        save(&p, &path).unwrap();
        assert_eq!(fs::read_dir(dir.path().join("backups")).unwrap().count(), 1);
    }

    #[test]
    fn autosave_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.blueprint");
        let mut p = Project::default();
        save(&p, &path).unwrap();
        assert!(recoverable(&path).is_none());
        p.meta.production = "Unsaved".into();
        std::thread::sleep(std::time::Duration::from_millis(20));
        autosave(&p, &path).unwrap();
        assert_eq!(recoverable(&path).unwrap().meta.production, "Unsaved");
        save(&p, &path).unwrap();
        assert!(recoverable(&path).is_none());
    }

    #[test]
    fn newer_schema_is_refused_and_missing_fields_default() {
        let newer = format!("{{\"schema_version\": {}}}", SCHEMA_VERSION + 1);
        assert!(matches!(parse(&newer), Err(FileError::TooNew(_))));
        let p = parse(r#"{"cast":[{"id":7,"name":"Ben"}]}"#).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        assert_eq!(p.next_id, 8, "next_id is repaired above the ids in use");
        assert!(parse("[1,2]").is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn extension_is_added_once() {
        assert_eq!(with_extension(Path::new("a")), PathBuf::from("a.blueprint"));
        assert_eq!(
            with_extension(Path::new("a.blueprint")),
            PathBuf::from("a.blueprint")
        );
    }
}
