//! Setlists: a song library (each song once, with key, tempo, length, line-up
//! and cues) and setlists that refer to its songs by id, mixed with special
//! entries (break, announcement, changeover, encore). Times are worked out from
//! the start time, the gap between songs and the transitions.

use crate::grid::{col, ColKind, Column, Grid};
use crate::model::{Id, Project};
use crate::sheet::{move_rows, parse_color, remove_rows};
use crate::time::{format_clock, format_duration, format_number, parse_clock, parse_duration, parse_number};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Song {
    pub id: Id,
    pub title: String,
    /// "E", "F#m", "Bb" …
    pub key: String,
    pub bpm: Option<f64>,
    /// Seconds.
    pub duration: Option<f64>,
    /// Who plays what ("Gtr: Max, acoustic").
    pub lineup: String,
    /// Lead vocal.
    pub lead: String,
    pub light_cue: String,
    pub sound_cue: String,
    pub color: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum EntryKind {
    #[default]
    Song,
    Break,
    Announcement,
    Changeover,
    /// Starts the encore (a marker; it can have a length, e.g. the walk off).
    Encore,
}

impl EntryKind {
    pub const ALL: [EntryKind; 5] = [
        EntryKind::Song,
        EntryKind::Break,
        EntryKind::Announcement,
        EntryKind::Changeover,
        EntryKind::Encore,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EntryKind::Song => "Song",
            EntryKind::Break => "Break",
            EntryKind::Announcement => "Announcement",
            EntryKind::Changeover => "Changeover",
            EntryKind::Encore => "Encore",
        }
    }

    pub fn parse(v: &str) -> Option<EntryKind> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(EntryKind::Song);
        }
        EntryKind::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase();
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }
}

/// How an entry goes over into the next one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Transition {
    /// The usual gap of the setlist.
    #[default]
    Gap,
    /// Straight into the next song, no gap.
    Attacca,
    /// Crossfade or segue, no gap.
    Segue,
}

impl Transition {
    pub const ALL: [Transition; 3] = [Transition::Gap, Transition::Attacca, Transition::Segue];

    pub fn label(self) -> &'static str {
        match self {
            Transition::Gap => "",
            Transition::Attacca => "Attacca",
            Transition::Segue => "Segue",
        }
    }

    /// The sign printed between two entries.
    pub fn sign(self) -> &'static str {
        match self {
            Transition::Gap => "",
            Transition::Attacca => "→",
            Transition::Segue => "~",
        }
    }

    pub fn parse(v: &str) -> Option<Transition> {
        let v = v.trim().to_ascii_lowercase();
        match v.as_str() {
            "" | "gap" | "-" | "none" => Some(Transition::Gap),
            "attacca" | "att" | "→" | "->" | ">" => Some(Transition::Attacca),
            "segue" | "seg" | "~" | "crossfade" => Some(Transition::Segue),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Entry {
    pub id: Id,
    pub kind: EntryKind,
    /// The library song (for `Song` entries).
    pub song: Option<Id>,
    /// Text of a special entry ("Thank you, introduce the band").
    pub title: String,
    /// Length of a special entry, seconds.
    pub duration: Option<f64>,
    /// Into the next entry.
    pub transition: Transition,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Setlist {
    pub id: Id,
    pub name: String,
    pub band: String,
    pub venue: String,
    pub date: String,
    /// Time of day the first entry starts, seconds after midnight.
    pub start: Option<f64>,
    /// Gap between two songs, seconds.
    pub gap: f64,
    pub notes: String,
    /// Logo image (PNG, JPEG or SVG), base64.
    pub logo: String,
    pub entries: Vec<Entry>,
}

impl Default for Setlist {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            band: String::new(),
            venue: String::new(),
            date: String::new(),
            start: None,
            gap: 20.0,
            notes: String::new(),
            logo: String::new(),
            entries: Vec::new(),
        }
    }
}

impl Setlist {
    pub fn logo_bytes(&self) -> Option<Vec<u8>> {
        if self.logo.is_empty() {
            return None;
        }
        base64::engine::general_purpose::STANDARD.decode(&self.logo).ok()
    }

    pub fn set_logo(&mut self, bytes: &[u8]) {
        self.logo = base64::engine::general_purpose::STANDARD.encode(bytes);
    }
}

/// Times of one entry.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct EntryTime {
    /// Seconds after the set start.
    pub at: f64,
    pub length: f64,
    /// Time of day, when the setlist has a start time.
    pub clock: Option<f64>,
    /// Song number (songs only, counted through the whole list).
    pub number: Option<usize>,
    /// A song without a length (the totals are too short).
    pub unknown: bool,
}

/// A part of the show between breaks (the encore is a part of its own).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Part {
    pub name: String,
    /// Entry indices.
    pub from: usize,
    pub to: usize,
    pub length: f64,
    pub songs: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Timing {
    pub entries: Vec<EntryTime>,
    pub parts: Vec<Part>,
    /// From the first entry to the end of the last.
    pub total: f64,
    /// Songs only.
    pub music: f64,
    pub songs: usize,
    /// Songs without a length.
    pub unknown: usize,
    pub end: Option<f64>,
}

impl Timing {
    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} song{} · music {} · total {}",
            self.songs,
            if self.songs == 1 { "" } else { "s" },
            format_duration(self.music),
            format_duration(self.total)
        );
        if let Some(e) = self.end {
            s.push_str(&format!(" · ends {}", format_clock(e)));
        }
        if self.unknown > 0 {
            s.push_str(&format!(" · {} without a length", self.unknown));
        }
        s
    }
}

impl Project {
    pub fn song(&self, id: Id) -> Option<&Song> {
        self.songs.iter().find(|s| s.id == id)
    }

    pub fn song_mut(&mut self, id: Id) -> Option<&mut Song> {
        self.songs.iter_mut().find(|s| s.id == id)
    }

    pub fn song_named(&self, title: &str) -> Option<&Song> {
        let t = title.trim();
        self.songs
            .iter()
            .find(|s| !t.is_empty() && s.title.trim().eq_ignore_ascii_case(t))
    }

    pub fn setlist(&self, id: Id) -> Option<&Setlist> {
        self.setlists.iter().find(|s| s.id == id)
    }

    pub fn setlist_mut(&mut self, id: Id) -> Option<&mut Setlist> {
        self.setlists.iter_mut().find(|s| s.id == id)
    }

    /// A new, empty setlist at the end; returns its id.
    pub fn add_setlist(&mut self, name: &str) -> Id {
        let id = self.alloc_id();
        let band = self.setlists.last().map(|s| s.band.clone()).unwrap_or_default();
        self.setlists.push(Setlist { id, name: name.into(), band, ..Setlist::default() });
        id
    }

    /// A copy of a setlist (new ids, same songs); returns its id.
    pub fn duplicate_setlist(&mut self, id: Id) -> Option<Id> {
        let mut s = self.setlist(id)?.clone();
        s.id = self.alloc_id();
        s.name = format!("{} (copy)", s.name);
        for e in &mut s.entries {
            e.id = self.alloc_id();
        }
        let at = self.setlists.iter().position(|x| x.id == id).map_or(self.setlists.len(), |i| i + 1);
        let new = s.id;
        self.setlists.insert(at, s);
        Some(new)
    }

    /// How many setlist entries use the song.
    pub fn song_uses(&self, id: Id) -> usize {
        self.setlists
            .iter()
            .flat_map(|s| &s.entries)
            .filter(|e| e.song == Some(id))
            .count()
    }

    /// The library song with this title, made if there is none.
    pub fn song_for_title(&mut self, title: &str) -> Id {
        if let Some(s) = self.song_named(title) {
            return s.id;
        }
        let id = self.alloc_id();
        self.songs.push(Song { id, title: title.trim().into(), ..Song::default() });
        id
    }
}

/// The text shown for an entry (song title or the special text).
pub fn entry_title(p: &Project, e: &Entry) -> String {
    match e.kind {
        EntryKind::Song => e.song.and_then(|id| p.song(id)).map(|s| s.title.clone()).unwrap_or_default(),
        _ => e.title.clone(),
    }
}

/// The length of an entry, if known.
pub fn entry_length(p: &Project, e: &Entry) -> Option<f64> {
    match e.kind {
        EntryKind::Song => e.duration.or_else(|| e.song.and_then(|id| p.song(id)).and_then(|s| s.duration)),
        _ => e.duration,
    }
}

/// Work out the times of a setlist.
pub fn timing(p: &Project, s: &Setlist) -> Timing {
    let mut t = Timing::default();
    let mut at = 0.0;
    let mut number = 0;
    let mut part = Part { name: "Set 1".into(), ..Part::default() };
    let mut sets = 1;
    let n = s.entries.len();
    for (i, e) in s.entries.iter().enumerate() {
        let len = entry_length(p, e);
        let length = len.unwrap_or(0.0);
        let is_song = e.kind == EntryKind::Song;
        let unknown = is_song && len.is_none();
        let num = if is_song {
            number += 1;
            Some(number)
        } else {
            None
        };
        match e.kind {
            EntryKind::Break => {
                if part.to > part.from || part.songs > 0 {
                    t.parts.push(std::mem::take(&mut part));
                }
                sets += 1;
                part = Part { name: format!("Set {sets}"), from: i + 1, to: i + 1, ..Part::default() };
            }
            EntryKind::Encore => {
                if part.to > part.from || part.songs > 0 {
                    t.parts.push(std::mem::take(&mut part));
                }
                part = Part { name: "Encore".into(), from: i, to: i + 1, length, ..Part::default() };
            }
            _ => {
                part.to = i + 1;
                part.length += length;
                if is_song {
                    part.songs += 1;
                }
            }
        }
        t.entries.push(EntryTime { at, length, clock: s.start.map(|st| st + at), number: num, unknown });
        at += length;
        if is_song {
            t.music += length;
            t.songs += 1;
        }
        if unknown {
            t.unknown += 1;
        }
        // The gap goes between two songs only.
        let next_song = s.entries.get(i + 1).is_some_and(|x| x.kind == EntryKind::Song);
        if is_song && next_song && i + 1 < n && e.transition == Transition::Gap {
            at += s.gap.max(0.0);
            part.length += s.gap.max(0.0);
        }
    }
    if part.to > part.from || part.songs > 0 {
        t.parts.push(part);
    }
    t.total = at;
    t.end = s.start.map(|st| st + at);
    t
}

/// The song library as a table.
pub struct SongGrid;

/// One setlist as a table.
pub struct SetlistGrid(pub Id);

impl SongGrid {
    pub const COLS: usize = 11;
}

impl Grid for SongGrid {
    fn key(&self) -> String {
        "songs".into()
    }
    fn columns(&self, _p: &Project) -> Vec<Column> {
        use ColKind::*;
        vec![
            col("Title", Text, 220.0),
            col("Key", Text, 60.0),
            col("BPM", Number, 60.0),
            col("Length", Duration, 70.0),
            col("Lead vocal", Text, 120.0),
            col("Line-up", Text, 200.0),
            col("Light cue", Text, 110.0),
            col("Sound cue", Text, 110.0),
            col("Notes", Text, 220.0),
            col("Colour", Color, 90.0),
            col("Used", Fixed, 60.0),
        ]
    }
    fn len(&self, p: &Project) -> usize {
        p.songs.len()
    }
    fn text(&self, p: &Project, row: usize, c: usize) -> String {
        let Some(s) = p.songs.get(row) else { return String::new() };
        match c {
            10 => {
                let n = p.song_uses(s.id);
                if n == 0 { String::new() } else { n.to_string() }
            }
            _ => song_text(s, c),
        }
    }
    fn set(&self, p: &mut Project, row: usize, c: usize, v: &str) -> Result<(), String> {
        let s = p.songs.get_mut(row).ok_or("no such song")?;
        set_song(s, c, v)
    }
    fn color(&self, p: &Project, row: usize) -> String {
        p.songs.get(row).map(|s| s.color.clone()).unwrap_or_default()
    }
    fn editable_rows(&self) -> bool {
        true
    }
    fn insert(&self, p: &mut Project, at: usize) -> usize {
        let id = p.alloc_id();
        let at = at.min(p.songs.len());
        p.songs.insert(at, Song { id, ..Song::default() });
        at
    }
    fn remove(&self, p: &mut Project, rows: &[usize]) {
        remove_rows(&mut p.songs, rows);
        p.prune();
    }
    fn move_rows(&self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        move_rows(&mut p.songs, rows, to)
    }
}

/// Song fields by library column (0 title … 9 colour).
fn song_text(s: &Song, c: usize) -> String {
    match c {
        0 => s.title.clone(),
        1 => s.key.clone(),
        2 => s.bpm.map(format_number).unwrap_or_default(),
        3 => s.duration.map(format_duration).unwrap_or_default(),
        4 => s.lead.clone(),
        5 => s.lineup.clone(),
        6 => s.light_cue.clone(),
        7 => s.sound_cue.clone(),
        8 => s.notes.clone(),
        9 => s.color.clone(),
        _ => String::new(),
    }
}

fn set_song(s: &mut Song, c: usize, v: &str) -> Result<(), String> {
    let t = v.trim();
    match c {
        0 => s.title = t.into(),
        1 => s.key = t.into(),
        2 => {
            s.bpm = if t.is_empty() {
                None
            } else {
                Some(parse_number(t).filter(|b| *b > 0.0).ok_or(format!("\"{t}\" is not a tempo"))?)
            }
        }
        3 => s.duration = parse_opt_duration(t)?,
        4 => s.lead = t.into(),
        5 => s.lineup = t.into(),
        6 => s.light_cue = t.into(),
        7 => s.sound_cue = t.into(),
        8 => s.notes = v.into(),
        9 => s.color = parse_color(t)?,
        _ => return Err("this column is worked out".into()),
    }
    Ok(())
}

fn parse_opt_duration(t: &str) -> Result<Option<f64>, String> {
    if t.is_empty() {
        Ok(None)
    } else {
        parse_duration(t).map(Some).ok_or(format!("\"{t}\" is not a length (use m:ss)"))
    }
}

/// Setlist columns.
pub mod setcol {
    pub const NO: usize = 0;
    pub const KIND: usize = 1;
    pub const TITLE: usize = 2;
    pub const KEY: usize = 3;
    pub const BPM: usize = 4;
    pub const LENGTH: usize = 5;
    pub const NEXT: usize = 6;
    pub const START: usize = 7;
    pub const CLOCK: usize = 8;
    pub const LEAD: usize = 9;
    pub const LINEUP: usize = 10;
    pub const LIGHT: usize = 11;
    pub const SOUND: usize = 12;
    pub const NOTES: usize = 13;
}

/// Setlist column → library column for the song fields.
fn library_col(c: usize) -> Option<usize> {
    use setcol::*;
    Some(match c {
        KEY => 1,
        BPM => 2,
        LENGTH => 3,
        LEAD => 4,
        LINEUP => 5,
        LIGHT => 6,
        SOUND => 7,
        _ => return None,
    })
}

impl SetlistGrid {
    fn list<'a>(&self, p: &'a Project) -> Option<&'a Setlist> {
        p.setlist(self.0)
    }
}

impl Grid for SetlistGrid {
    fn key(&self) -> String {
        format!("setlist:{}", self.0)
    }
    fn columns(&self, _p: &Project) -> Vec<Column> {
        use ColKind::*;
        let kinds = EntryKind::ALL.iter().map(|k| k.label().to_string()).collect();
        let next = vec!["Gap".to_string(), "Attacca".into(), "Segue".into()];
        vec![
            col("No.", Fixed, 40.0),
            col("Type", Choice(kinds), 110.0),
            col("Title", Text, 230.0),
            col("Key", Text, 55.0),
            col("BPM", Number, 55.0),
            col("Length", Duration, 70.0),
            col("Into next", Choice(next), 85.0),
            col("Starts", Fixed, 65.0),
            col("Clock", Fixed, 60.0),
            col("Lead vocal", Text, 110.0),
            col("Line-up", Text, 180.0),
            col("Light cue", Text, 100.0),
            col("Sound cue", Text, 100.0),
            col("Notes", Text, 200.0),
        ]
    }
    fn len(&self, p: &Project) -> usize {
        self.list(p).map_or(0, |s| s.entries.len())
    }
    fn text(&self, p: &Project, row: usize, c: usize) -> String {
        use setcol::*;
        let Some(s) = self.list(p) else { return String::new() };
        let Some(e) = s.entries.get(row) else { return String::new() };
        let song = e.song.and_then(|id| p.song(id)).filter(|_| e.kind == EntryKind::Song);
        match c {
            NO | START | CLOCK => {
                let t = timing(p, s);
                let Some(et) = t.entries.get(row) else { return String::new() };
                match c {
                    NO => et.number.map(|n| n.to_string()).unwrap_or_default(),
                    START => format_duration(et.at),
                    _ => et.clock.map(format_clock).unwrap_or_default(),
                }
            }
            KIND => e.kind.label().into(),
            TITLE => entry_title(p, e),
            LENGTH => entry_length(p, e).map(format_duration).unwrap_or_default(),
            NEXT => match e.transition {
                Transition::Gap => String::new(),
                t => t.label().into(),
            },
            NOTES => e.notes.clone(),
            _ => match (song, library_col(c)) {
                (Some(song), Some(lc)) => song_text(song, lc),
                _ => String::new(),
            },
        }
    }
    fn set(&self, p: &mut Project, row: usize, c: usize, v: &str) -> Result<(), String> {
        use setcol::*;
        let entry = self
            .list(p)
            .and_then(|s| s.entries.get(row))
            .cloned()
            .ok_or("no such entry")?;
        let t = v.trim();
        let mut e = entry.clone();
        match c {
            KIND => {
                let k = EntryKind::parse(t).ok_or(format!("\"{t}\" is not a type (use song, break, announcement, changeover or encore)"))?;
                if k != e.kind {
                    let title = entry_title(p, &e);
                    if k == EntryKind::Song {
                        e.song = (!title.trim().is_empty()).then(|| p.song_for_title(&title));
                        e.title.clear();
                    } else {
                        e.title = if e.kind == EntryKind::Song || title.is_empty() { k.label().into() } else { title };
                        e.song = None;
                    }
                    e.kind = k;
                }
            }
            TITLE => {
                if e.kind == EntryKind::Song {
                    if t.is_empty() {
                        e.song = None;
                    } else if let Some(other) = p.song_named(t).map(|s| s.id) {
                        e.song = Some(other);
                    } else if let Some(id) = e.song.filter(|id| p.song_uses(*id) == 1) {
                        // Only here: rename the library song.
                        if let Some(s) = p.song_mut(id) {
                            s.title = t.into();
                        }
                    } else {
                        e.song = Some(p.song_for_title(t));
                    }
                } else {
                    e.title = t.into();
                }
            }
            LENGTH if e.kind != EntryKind::Song => e.duration = parse_opt_duration(t)?,
            NEXT => e.transition = Transition::parse(t).ok_or(format!("\"{t}\" is not a transition (use gap, attacca or segue)"))?,
            NOTES => e.notes = v.into(),
            NO | START | CLOCK => return Err("this column is worked out".into()),
            _ => {
                let lc = library_col(c).ok_or("no such column")?;
                if e.kind != EntryKind::Song {
                    return if t.is_empty() { Ok(()) } else { Err("only songs have this".into()) };
                }
                // A song field typed before the title makes an untitled song.
                let id = match e.song {
                    Some(id) => id,
                    None => {
                        let id = p.alloc_id();
                        p.songs.push(Song { id, ..Song::default() });
                        e.song = Some(id);
                        id
                    }
                };
                let s = p.song_mut(id).ok_or("the song is gone")?;
                set_song(s, lc, v)?;
            }
        }
        if e != entry {
            if let Some(slot) = p.setlist_mut(self.0).and_then(|s| s.entries.get_mut(row)) {
                *slot = e;
            }
        }
        Ok(())
    }
    fn color(&self, p: &Project, row: usize) -> String {
        let Some(e) = self.list(p).and_then(|s| s.entries.get(row)) else { return String::new() };
        match e.kind {
            EntryKind::Song => e.song.and_then(|id| p.song(id)).map(|s| s.color.clone()).unwrap_or_default(),
            _ => String::new(),
        }
    }
    fn mark(&self, p: &Project, row: usize, c: usize) -> Option<crate::grid::CellMark> {
        let s = self.list(p)?;
        let e = s.entries.get(row)?;
        if e.kind != EntryKind::Song && c == setcol::KIND {
            return Some(crate::grid::CellMark { color: "blue".into(), ..Default::default() });
        }
        if c == setcol::LENGTH && e.kind == EntryKind::Song && entry_length(p, e).is_none() && e.song.is_some() {
            return Some(crate::grid::CellMark {
                warn: true,
                hint: "No length: the times after this song are too early".into(),
                ..Default::default()
            });
        }
        None
    }
    fn editable_rows(&self) -> bool {
        true
    }
    fn insert(&self, p: &mut Project, at: usize) -> usize {
        let id = p.alloc_id();
        let Some(s) = p.setlist_mut(self.0) else { return at };
        let at = at.min(s.entries.len());
        s.entries.insert(at, Entry { id, ..Entry::default() });
        at
    }
    fn remove(&self, p: &mut Project, rows: &[usize]) {
        if let Some(s) = p.setlist_mut(self.0) {
            remove_rows(&mut s.entries, rows);
        }
    }
    fn move_rows(&self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        match p.setlist_mut(self.0) {
            Some(s) => move_rows(&mut s.entries, rows, to),
            None => rows.to_vec(),
        }
    }
}

/// Parse a time of day for the setlist start ("" clears it).
pub fn parse_start(v: &str) -> Result<Option<f64>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    parse_clock(t).map(Some).ok_or(format!("\"{t}\" is not a time of day (use 20:00)"))
}

/// Remove entries whose song is gone.
pub(crate) fn prune(setlists: &mut [Setlist], songs: &[Id]) {
    for s in setlists {
        s.entries
            .retain(|e| e.kind != EntryKind::Song || e.song.is_none_or(|id| songs.contains(&id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::paste;

    fn project() -> (Project, Id) {
        let mut p = Project::default();
        let id = p.add_setlist("Main set");
        let g = SetlistGrid(id);
        let rows = "Song\tOpener\tE\t120\t3:30\nSong\tSecond\tA\t\t4:00\tAttacca\nSong\tThird\t\t\t3:00\nBreak\tInterval\t\t\t15:00\nSong\tFourth\t\t\t5:00\nEncore\tEncore\nSong\tLast one\t\t\t4:00";
        let (_, errs) = paste(&g, &mut p, 0, 1, rows);
        assert!(errs.is_empty(), "{errs:?}");
        (p, id)
    }

    #[test]
    fn typing_titles_fills_the_library() {
        let (p, id) = project();
        assert_eq!(p.songs.len(), 5);
        assert_eq!(p.song_named("opener").and_then(|s| s.duration), Some(210.0));
        assert_eq!(p.song_named("Opener").map(|s| s.key.as_str()), Some("E"));
        let s = p.setlist(id).unwrap();
        assert_eq!(s.entries.len(), 7);
        assert_eq!(s.entries[3].kind, EntryKind::Break);
        assert_eq!(s.entries[3].title, "Interval");
        assert_eq!(s.entries[1].transition, Transition::Attacca);
    }

    #[test]
    fn times_gaps_and_parts() {
        let (mut p, id) = project();
        p.setlist_mut(id).unwrap().start = parse_start("20:00").unwrap();
        let s = p.setlist(id).unwrap();
        let t = timing(&p, s);
        // Opener 3:30, gap 20, Second 4:00 attacca, Third 3:00.
        assert_eq!(t.entries[1].at, 230.0);
        assert_eq!(t.entries[2].at, 470.0);
        assert_eq!(t.entries[3].at, 650.0);
        assert_eq!(t.entries[4].at, 650.0 + 900.0);
        assert_eq!(t.entries[4].clock.map(format_clock).as_deref(), Some("20:26"));
        assert_eq!(t.parts.len(), 3);
        assert_eq!(t.parts[0].name, "Set 1");
        assert_eq!(t.parts[0].songs, 3);
        assert_eq!(t.parts[0].length, 650.0);
        assert_eq!(t.parts[1].name, "Set 2");
        assert_eq!(t.parts[2].name, "Encore");
        assert_eq!(t.parts[2].songs, 1);
        assert_eq!(t.songs, 5);
        assert_eq!(t.music, 210.0 + 240.0 + 180.0 + 300.0 + 240.0);
        assert_eq!(t.entries.iter().filter_map(|e| e.number).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        assert!(t.summary().contains("ends 20:"), "{}", t.summary());
    }

    #[test]
    fn one_song_in_two_setlists() {
        let (mut p, id) = project();
        let second = p.duplicate_setlist(id).unwrap();
        // A change from one setlist shows in the other.
        SetlistGrid(second).set(&mut p, 0, setcol::KEY, "F").unwrap();
        assert_eq!(SetlistGrid(id).text(&p, 0, setcol::KEY), "F");
        // A title used twice is not renamed: the entry gets a new song.
        SetlistGrid(second).set(&mut p, 0, setcol::TITLE, "Brand new").unwrap();
        assert_eq!(SetlistGrid(id).text(&p, 0, setcol::TITLE), "Opener");
        assert_eq!(p.songs.len(), 6);
        // A title used once is renamed in the library.
        SetlistGrid(second).set(&mut p, 0, setcol::TITLE, "Brand new (live)").unwrap();
        assert_eq!(p.songs.len(), 6);
        // A known title links.
        SetlistGrid(second).set(&mut p, 0, setcol::TITLE, "third").unwrap();
        assert_eq!(SetlistGrid(second).text(&p, 0, setcol::TITLE), "Third");
    }

    #[test]
    fn deleting_a_song_removes_its_entries() {
        let (mut p, id) = project();
        let row = p.songs.iter().position(|s| s.title == "Second").unwrap();
        SongGrid.remove(&mut p, &[row]);
        assert_eq!(p.setlist(id).unwrap().entries.len(), 6);
        assert_eq!(SongGrid.text(&p, 0, 10), "1");
    }

    #[test]
    fn types_and_errors() {
        let (mut p, id) = project();
        let g = SetlistGrid(id);
        assert!(g.set(&mut p, 0, setcol::KIND, "dance").is_err());
        assert!(g.set(&mut p, 0, setcol::START, "1").is_err());
        assert!(g.set(&mut p, 3, setcol::KEY, "E").is_err());
        g.set(&mut p, 0, setcol::KIND, "ann").unwrap();
        assert_eq!(p.setlist(id).unwrap().entries[0].title, "Announcement");
        g.set(&mut p, 0, setcol::TITLE, "Opener").unwrap();
        g.set(&mut p, 0, setcol::KIND, "song").unwrap();
        assert_eq!(g.text(&p, 0, setcol::KEY), "E");
        assert!(parse_start("25:00").is_err());
    }

    #[test]
    fn logo_round_trip() {
        let mut s = Setlist::default();
        s.set_logo(&[1, 2, 3, 250]);
        assert_eq!(s.logo_bytes(), Some(vec![1, 2, 3, 250]));
    }
}
