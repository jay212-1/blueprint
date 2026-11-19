//! The open project with its file path, unsaved-changes flag and undo/redo.
//!
//! Undo keeps whole snapshots (projects are small; this is simple and always
//! right). A run of edits with the same key within a moment (typing in one
//! cell, dragging) becomes one undo step.

use crate::model::Project;
use crate::project::{self, FileError};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A number that is new for every state of every open project (caches of
/// worked-out views and print previews compare it).
fn next_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Undo steps kept.
pub const UNDO_LIMIT: usize = 200;
/// Edits with the same key closer together than this are one undo step.
const COALESCE: Duration = Duration::from_millis(1200);

#[derive(Default)]
pub struct Document {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// Changes with every edit, undo and redo.
    pub generation: u64,
    undo: VecDeque<Project>,
    redo: Vec<Project>,
    last: Option<(String, Instant)>,
}

impl Document {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            generation: next_generation(),
            ..Self::default()
        }
    }

    pub fn open(path: &Path) -> Result<Self, FileError> {
        Ok(Self {
            project: project::load(path)?,
            path: Some(path.to_path_buf()),
            generation: next_generation(),
            ..Self::default()
        })
    }

    /// Change the project. `key` names what is edited (e.g. "cast:12:name");
    /// repeated edits of the same thing merge into one undo step. Returns
    /// what `f` returns. Nothing is recorded when the project did not change.
    pub fn edit<R>(&mut self, key: &str, f: impl FnOnce(&mut Project) -> R) -> R {
        let before = self.project.clone();
        let r = f(&mut self.project);
        if self.project == before {
            return r;
        }
        let now = Instant::now();
        let merge = matches!(&self.last, Some((k, t)) if k == key && now.duration_since(*t) < COALESCE);
        if !merge {
            self.undo.push_back(before);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.pop_front();
            }
        }
        self.last = Some((key.to_string(), now));
        self.redo.clear();
        self.dirty = true;
        self.generation = next_generation();
        r
    }

    /// End the current run: the next edit is a new undo step even with the
    /// same key.
    pub fn seal(&mut self) {
        self.last = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop_back() else {
            return false;
        };
        self.redo
            .push(std::mem::replace(&mut self.project, prev));
        self.last = None;
        self.dirty = true;
        self.generation = next_generation();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo
            .push_back(std::mem::replace(&mut self.project, next));
        self.last = None;
        self.dirty = true;
        self.generation = next_generation();
        true
    }

    /// Save to `path` (or the current path).
    pub fn save(&mut self, path: Option<&Path>) -> Result<PathBuf, FileError> {
        let target = match path {
            Some(p) => project::with_extension(p),
            None => match &self.path {
                Some(p) => p.clone(),
                None => {
                    return Err(FileError::Parse("no file chosen yet".into()));
                }
            },
        };
        project::save(&self.project, &target)?;
        self.path = Some(target.clone());
        self.dirty = false;
        self.last = None;
        Ok(target)
    }

    /// Display name: the production, else the file name, else "Untitled".
    pub fn title(&self) -> String {
        let p = self.project.meta.production.trim();
        if !p.is_empty() {
            return p.to_string();
        }
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "Untitled".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_and_coalescing() {
        let mut d = Document::default();
        d.edit("meta:production", |p| p.meta.production = "A".into());
        d.edit("meta:production", |p| p.meta.production = "AB".into());
        d.edit("meta:venue", |p| p.meta.venue = "Hall".into());
        assert!(d.dirty);
        assert!(d.undo());
        assert_eq!(d.project.meta.venue, "");
        assert_eq!(d.project.meta.production, "AB");
        assert!(d.undo(), "typing in one field was one step");
        assert_eq!(d.project.meta.production, "");
        assert!(!d.undo());
        assert!(d.redo());
        assert_eq!(d.project.meta.production, "AB");
        // A new edit clears redo.
        d.edit("x", |p| p.meta.notes = "n".into());
        assert!(!d.can_redo());
    }

    #[test]
    fn no_change_no_step_and_seal_splits() {
        let mut d = Document::default();
        d.edit("k", |_| {});
        assert!(!d.can_undo() && !d.dirty);
        d.edit("k", |p| p.meta.notes = "1".into());
        d.seal();
        d.edit("k", |p| p.meta.notes = "2".into());
        d.undo();
        assert_eq!(d.project.meta.notes, "1");
    }

    #[test]
    fn save_needs_a_path_then_remembers_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = Document::default();
        assert!(d.save(None).is_err());
        d.edit("k", |p| p.meta.production = "Show".into());
        let path = d.save(Some(&dir.path().join("x"))).unwrap();
        assert!(path.ends_with("x.blueprint") && !d.dirty);
        assert_eq!(d.title(), "Show");
        assert_eq!(Document::open(&path).unwrap().project.meta.production, "Show");
    }
}
