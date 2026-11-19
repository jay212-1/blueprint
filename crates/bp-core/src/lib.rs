//! Blueprint core: the project model, the project file (schema version,
//! migrations, atomic save, autosave), undo/redo and the table view of the
//! master data. No user interface, no printing.

pub mod demo;
pub mod document;
pub mod grid;
pub mod mic;
pub mod mic_grid;
pub mod model;
pub mod patch;
pub mod patch_grid;
pub mod plan_grid;
pub mod plans;
pub mod project;
pub mod setlist;
pub mod sheet;
pub mod stage;
pub mod time;
