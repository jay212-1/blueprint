//! More plans: a project holds any number of them, each of one kind (stage
//! plot, run of show, contacts, packing list, intercom, free table). A new
//! kind is a new [`PlanKind`] variant with its data, its tables in
//! `plan_grid.rs`, a page in the app and a print layout.

use crate::model::{Id, Project, TxStatus};
use crate::stage::StagePlot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Plan {
    pub id: Id,
    pub name: String,
    pub notes: String,
    pub kind: PlanKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum PlanKind {
    StagePlot(StagePlot),
    RunOfShow(RunOfShow),
    Contacts(Contacts),
    Packlist(Packlist),
    Intercom(Intercom),
    Table(FreeTable),
}

impl Default for PlanKind {
    fn default() -> Self {
        PlanKind::Table(FreeTable::default())
    }
}

/// The kinds of plan, without their data (menus, new plans).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanType {
    StagePlot,
    RunOfShow,
    Contacts,
    Packlist,
    Intercom,
    Table,
}

impl PlanType {
    pub const ALL: [PlanType; 6] = [
        PlanType::StagePlot,
        PlanType::RunOfShow,
        PlanType::Contacts,
        PlanType::Packlist,
        PlanType::Intercom,
        PlanType::Table,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PlanType::StagePlot => "Stage plot",
            PlanType::RunOfShow => "Run of show",
            PlanType::Contacts => "Contacts",
            PlanType::Packlist => "Packing list",
            PlanType::Intercom => "Intercom / radio",
            PlanType::Table => "Free table",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            PlanType::StagePlot => "The stage from above: risers, backline, monitors, stands (with their input channels)",
            PlanType::RunOfShow => "Times, lengths, who, where and cues: get-in, rehearsal, show day",
            PlanType::Contacts => "Crew, band, venue and suppliers with phone and email",
            PlanType::Packlist => "Gear to pack: counted from the patch and the mic plan, plus your own items",
            PlanType::Intercom => "Channels and who talks or listens on which",
            PlanType::Table => "Your own columns (text, number, time, length, check box, choice, colour)",
        }
    }

    pub fn new_kind(self) -> PlanKind {
        match self {
            PlanType::StagePlot => PlanKind::StagePlot(StagePlot::default()),
            PlanType::RunOfShow => PlanKind::RunOfShow(RunOfShow::default()),
            PlanType::Contacts => PlanKind::Contacts(Contacts::default()),
            PlanType::Packlist => PlanKind::Packlist(Packlist::default()),
            PlanType::Intercom => PlanKind::Intercom(Intercom::default()),
            PlanType::Table => PlanKind::Table(FreeTable::starter()),
        }
    }
}

impl PlanKind {
    pub fn plan_type(&self) -> PlanType {
        match self {
            PlanKind::StagePlot(_) => PlanType::StagePlot,
            PlanKind::RunOfShow(_) => PlanType::RunOfShow,
            PlanKind::Contacts(_) => PlanType::Contacts,
            PlanKind::Packlist(_) => PlanType::Packlist,
            PlanKind::Intercom(_) => PlanType::Intercom,
            PlanKind::Table(_) => PlanType::Table,
        }
    }

    /// Every id inside the plan (for `repair_ids`).
    pub fn ids(&self) -> Vec<Id> {
        match self {
            PlanKind::StagePlot(s) => s.items.iter().map(|x| x.id).collect(),
            PlanKind::RunOfShow(r) => r.rows.iter().map(|x| x.id).collect(),
            PlanKind::Contacts(c) => c.rows.iter().map(|x| x.id).collect(),
            PlanKind::Packlist(k) => k.rows.iter().map(|x| x.id).collect(),
            PlanKind::Intercom(i) => i.channels.iter().map(|x| x.id).chain(i.people.iter().map(|x| x.id)).collect(),
            PlanKind::Table(t) => t.columns.iter().map(|x| x.id).chain(t.rows.iter().map(|x| x.id)).collect(),
        }
    }
}

impl Project {
    pub fn plan(&self, id: Id) -> Option<&Plan> {
        self.plans.iter().find(|p| p.id == id)
    }

    pub fn plan_mut(&mut self, id: Id) -> Option<&mut Plan> {
        self.plans.iter_mut().find(|p| p.id == id)
    }

    /// A new plan of this kind at the end; returns its id.
    pub fn rename_plan(&mut self, id: Id, name: &str) {
        if let Some(p) = self.plan_mut(id) {
            p.name = name.into();
        }
    }

    pub fn add_plan(&mut self, t: PlanType) -> Id {
        let id = self.alloc_id();
        let n = self.plans.iter().filter(|p| p.kind.plan_type() == t).count();
        let name = if n == 0 { t.label().to_string() } else { format!("{} {}", t.label(), n + 1) };
        let mut kind = t.new_kind();
        // The starter columns of a free table need ids.
        if let PlanKind::Table(ft) = &mut kind {
            for c in &mut ft.columns {
                c.id = self.alloc_id();
            }
        }
        self.plans.push(Plan { id, name, notes: String::new(), kind });
        id
    }

    /// A copy with new ids (references to other plans' rows are kept).
    pub fn duplicate_plan(&mut self, id: Id) -> Option<Id> {
        let mut plan = self.plan(id)?.clone();
        plan.id = self.alloc_id();
        plan.name = format!("{} (copy)", plan.name);
        match &mut plan.kind {
            PlanKind::StagePlot(s) => s.items.iter_mut().for_each(|x| x.id = self.alloc_id()),
            PlanKind::RunOfShow(r) => r.rows.iter_mut().for_each(|x| x.id = self.alloc_id()),
            PlanKind::Contacts(c) => c.rows.iter_mut().for_each(|x| x.id = self.alloc_id()),
            PlanKind::Packlist(k) => k.rows.iter_mut().for_each(|x| x.id = self.alloc_id()),
            PlanKind::Intercom(i) => {
                let mut map = BTreeMap::new();
                for c in &mut i.channels {
                    let new = self.alloc_id();
                    map.insert(c.id, new);
                    c.id = new;
                }
                for u in &mut i.people {
                    u.id = self.alloc_id();
                    for a in &mut u.access {
                        a.channel = map.get(&a.channel).copied().unwrap_or(a.channel);
                    }
                }
            }
            PlanKind::Table(t) => {
                let mut map = BTreeMap::new();
                for c in &mut t.columns {
                    let new = self.alloc_id();
                    map.insert(c.id, new);
                    c.id = new;
                }
                for r in &mut t.rows {
                    r.id = self.alloc_id();
                    for c in &mut r.cells {
                        c.col = map.get(&c.col).copied().unwrap_or(c.col);
                    }
                }
            }
        }
        let at = self.plans.iter().position(|x| x.id == id).map_or(self.plans.len(), |i| i + 1);
        let new = plan.id;
        self.plans.insert(at, plan);
        Some(new)
    }
}

// ---------------------------------------------------------------------------
// Run of show

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct RunOfShow {
    /// "Fri 16 April 2027".
    pub date: String,
    pub rows: Vec<Cue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Cue {
    pub id: Id,
    /// Time of day (seconds after midnight); empty: after the row above.
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub what: String,
    pub who: String,
    pub location: String,
    pub cue: String,
    pub notes: String,
    pub color: String,
}

/// Start and end of every row: a row without a start follows the one above.
pub fn run_times(r: &RunOfShow) -> Vec<(Option<f64>, Option<f64>)> {
    let mut out = Vec::new();
    let mut prev_end: Option<f64> = None;
    for c in &r.rows {
        let start = c.start.or(prev_end);
        let end = match (start, c.duration) {
            (Some(s), Some(d)) => Some(s + d),
            (Some(s), None) => Some(s),
            _ => None,
        };
        out.push((start, end));
        prev_end = end;
    }
    out
}

// ---------------------------------------------------------------------------
// Contacts

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Contacts {
    pub rows: Vec<Contact>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Contact {
    pub id: Id,
    pub name: String,
    /// "FOH engineer", "Stage manager".
    pub function: String,
    /// "Crew", "Band", "Venue", "Supplier".
    pub group: String,
    pub company: String,
    pub phone: String,
    pub email: String,
    pub notes: String,
    pub color: String,
}

/// Add the cast to a contact list (those not on it yet); returns how many.
pub fn add_cast(p: &mut Project, plan: Id) -> usize {
    let cast: Vec<(String, String, String)> = p
        .cast
        .iter()
        .map(|x| (x.name.clone(), x.contact.clone(), x.notes.clone()))
        .collect();
    let mut ids = Vec::new();
    for _ in 0..cast.len() {
        ids.push(p.alloc_id());
    }
    let Some(Plan { kind: PlanKind::Contacts(c), .. }) = p.plan_mut(plan) else { return 0 };
    let mut n = 0;
    for ((name, contact, notes), id) in cast.into_iter().zip(ids) {
        if name.trim().is_empty() || c.rows.iter().any(|r| r.name.trim().eq_ignore_ascii_case(name.trim())) {
            continue;
        }
        let (email, phone) = if contact.contains('@') { (contact, String::new()) } else { (String::new(), contact) };
        c.rows.push(Contact { id, name, function: notes, group: "Cast".into(), phone, email, ..Contact::default() });
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Packing list

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Packlist {
    pub rows: Vec<PackRow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct PackRow {
    pub id: Id,
    pub group: String,
    pub what: String,
    pub count: u32,
    /// Case or box it travels in.
    pub case: String,
    pub packed: bool,
    pub notes: String,
    /// "" for your own rows, "Patch" or "Mic plan" for counted ones.
    pub source: String,
}

/// Items counted from the patch and the mic plan: (source, group, what,
/// count, notes).
pub fn counted_items(p: &Project) -> Vec<(&'static str, String, String, u32, String)> {
    let mut out = Vec::new();
    for x in crate::patch::pack_list(p) {
        out.push(("Patch", x.group.to_string(), x.what, x.count, x.notes));
    }
    let mut tx: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let mut batteries: BTreeMap<String, u32> = BTreeMap::new();
    for t in &p.transmitters {
        if t.status == TxStatus::Defect {
            continue;
        }
        let model = if t.model.trim().is_empty() { "Transmitter".to_string() } else { t.model.trim().to_string() };
        let e = tx.entry(model).or_default();
        if t.status == TxStatus::Spare {
            e.1 += 1;
        } else {
            e.0 += 1;
        }
        if !t.battery.trim().is_empty() {
            *batteries.entry(t.battery.trim().to_string()).or_default() += 1;
        }
    }
    for (model, (active, spare)) in tx {
        let notes = if spare > 0 { format!("{active} in use, {spare} spare") } else { String::new() };
        out.push(("Mic plan", "Wireless".into(), model, active + spare, notes));
    }
    let mut caps: BTreeMap<String, u32> = BTreeMap::new();
    for c in &p.capsules {
        let what = if c.model.trim().is_empty() { "Capsule".to_string() } else { c.model.trim().to_string() };
        *caps.entry(what).or_default() += 1;
    }
    for (what, n) in caps {
        out.push(("Mic plan", "Capsules".into(), what, n, String::new()));
    }
    for (what, n) in batteries {
        out.push(("Mic plan", "Batteries".into(), what, n, "One per transmitter and show: add the interval changes".into()));
    }
    out
}

/// Bring the counted rows up to date: new ones are added, counts follow the
/// plans, counted rows that are gone are removed (packed marks and cases are
/// kept). Your own rows stay as they are. Returns (added, changed, removed).
pub fn refresh_packlist(p: &mut Project, plan: Id) -> (usize, usize, usize) {
    let items = counted_items(p);
    let ids: Vec<Id> = (0..items.len()).map(|_| p.alloc_id()).collect();
    let Some(Plan { kind: PlanKind::Packlist(k), .. }) = p.plan_mut(plan) else { return (0, 0, 0) };
    let (mut added, mut changed) = (0, 0);
    let key = |s: &str, g: &str, w: &str| (s.to_string(), g.to_lowercase(), w.to_lowercase());
    let wanted: Vec<_> = items.iter().map(|(s, g, w, _, _)| key(s, g, w)).collect();
    let before = k.rows.len();
    k.rows.retain(|r| r.source.is_empty() || wanted.contains(&key(&r.source, &r.group, &r.what)));
    let removed = before - k.rows.len();
    for ((source, group, what, count, notes), id) in items.into_iter().zip(ids) {
        match k.rows.iter_mut().find(|r| key(&r.source, &r.group, &r.what) == key(source, &group, &what)) {
            Some(r) => {
                if r.count != count || r.notes != notes {
                    r.count = count;
                    r.notes = notes;
                    changed += 1;
                }
            }
            None => {
                k.rows.push(PackRow { id, group, what, count, notes, source: source.into(), ..PackRow::default() });
                added += 1;
            }
        }
    }
    (added, changed, removed)
}

// ---------------------------------------------------------------------------
// Intercom

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum IcKind {
    #[default]
    Partyline,
    Wireless,
    Radio,
    Program,
}

impl IcKind {
    pub const ALL: [IcKind; 4] = [IcKind::Partyline, IcKind::Wireless, IcKind::Radio, IcKind::Program];

    pub fn label(self) -> &'static str {
        match self {
            IcKind::Partyline => "Partyline",
            IcKind::Wireless => "Wireless",
            IcKind::Radio => "Radio",
            IcKind::Program => "Program",
        }
    }

    pub fn parse(v: &str) -> Option<IcKind> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(IcKind::Partyline);
        }
        IcKind::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase();
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct IcChannel {
    pub id: Id,
    /// "A Stage", "B Light", "Radio 1".
    pub name: String,
    pub kind: IcKind,
    /// Port, channel number or frequency.
    pub number: String,
    pub notes: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Access {
    #[default]
    Talk,
    Listen,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(default)]
pub struct IcAccess {
    pub channel: Id,
    pub access: Access,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct IcUser {
    pub id: Id,
    pub name: String,
    /// "FOH", "Stage manager desk".
    pub position: String,
    /// Beltpack, radio or panel.
    pub device: String,
    pub group: String,
    pub access: Vec<IcAccess>,
    pub notes: String,
}

impl IcUser {
    pub fn on(&self, ch: Id) -> Option<Access> {
        self.access.iter().find(|a| a.channel == ch).map(|a| a.access)
    }

    pub fn set(&mut self, ch: Id, a: Option<Access>) {
        self.access.retain(|x| x.channel != ch);
        if let Some(access) = a {
            self.access.push(IcAccess { channel: ch, access });
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Intercom {
    pub channels: Vec<IcChannel>,
    pub people: Vec<IcUser>,
}

impl Intercom {
    pub(crate) fn prune(&mut self) {
        let ids: Vec<Id> = self.channels.iter().map(|c| c.id).collect();
        for u in &mut self.people {
            u.access.retain(|a| ids.contains(&a.channel));
        }
    }
}

// ---------------------------------------------------------------------------
// Free tables

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum FreeKind {
    #[default]
    Text,
    Number,
    Time,
    Duration,
    Check,
    Choice,
    Color,
}

impl FreeKind {
    pub const ALL: [FreeKind; 7] = [
        FreeKind::Text,
        FreeKind::Number,
        FreeKind::Time,
        FreeKind::Duration,
        FreeKind::Check,
        FreeKind::Choice,
        FreeKind::Color,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FreeKind::Text => "Text",
            FreeKind::Number => "Number",
            FreeKind::Time => "Time",
            FreeKind::Duration => "Length",
            FreeKind::Check => "Check box",
            FreeKind::Choice => "Choice",
            FreeKind::Color => "Colour",
        }
    }

    pub fn parse(v: &str) -> Option<FreeKind> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(FreeKind::Text);
        }
        match v.as_str() {
            "duration" => return Some(FreeKind::Duration),
            "checkbox" | "check" | "yes/no" => return Some(FreeKind::Check),
            "color" => return Some(FreeKind::Color),
            "clock" | "time of day" => return Some(FreeKind::Time),
            _ => {}
        }
        FreeKind::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase();
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct FreeCol {
    pub id: Id,
    pub name: String,
    pub kind: FreeKind,
    /// Choices, separated by commas.
    pub options: String,
    pub width: f32,
}

impl FreeCol {
    pub fn choices(&self) -> Vec<String> {
        self.options.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct FreeCell {
    pub col: Id,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct FreeRow {
    pub id: Id,
    pub cells: Vec<FreeCell>,
}

impl FreeRow {
    pub fn get(&self, col: Id) -> &str {
        self.cells.iter().find(|c| c.col == col).map(|c| c.value.as_str()).unwrap_or("")
    }

    pub fn put(&mut self, col: Id, value: String) {
        self.cells.retain(|c| c.col != col);
        if !value.is_empty() {
            self.cells.push(FreeCell { col, value });
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct FreeTable {
    pub columns: Vec<FreeCol>,
    pub rows: Vec<FreeRow>,
}

impl FreeTable {
    /// Two columns to start from (ids are given by `add_plan`).
    fn starter() -> FreeTable {
        FreeTable {
            columns: vec![
                FreeCol { name: "Item".into(), width: 200.0, ..FreeCol::default() },
                FreeCol { name: "Notes".into(), width: 260.0, ..FreeCol::default() },
            ],
            rows: Vec::new(),
        }
    }

    pub(crate) fn prune(&mut self) {
        let ids: Vec<Id> = self.columns.iter().map(|c| c.id).collect();
        for r in &mut self.rows {
            r.cells.retain(|c| ids.contains(&c.col));
        }
    }
}

/// A value typed into a free table cell, checked and written the usual way.
pub fn free_value(col: &FreeCol, v: &str) -> Result<String, String> {
    use crate::time::{format_clock, format_duration, format_number, parse_clock, parse_duration, parse_number};
    let t = v.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    Ok(match col.kind {
        FreeKind::Text => v.to_string(),
        FreeKind::Number => format_number(parse_number(t).ok_or(format!("\"{t}\" is not a number"))?),
        FreeKind::Time => format_clock(parse_clock(t).ok_or(format!("\"{t}\" is not a time of day (use 20:00)"))?),
        FreeKind::Duration => format_duration(parse_duration(t).ok_or(format!("\"{t}\" is not a length (use m:ss)"))?),
        FreeKind::Check => {
            if crate::sheet::parse_flag(t)? {
                "yes".into()
            } else {
                String::new()
            }
        }
        FreeKind::Choice => {
            let ch = col.choices();
            ch.iter()
                .find(|c| c.eq_ignore_ascii_case(t))
                .cloned()
                .ok_or(format!("\"{t}\" is not one of: {}", ch.join(", ")))?
        }
        FreeKind::Color => crate::sheet::parse_color(t)?,
    })
}

/// Drop references to things that are gone.
pub(crate) fn prune(plans: &mut [Plan], inputs: &[Id]) {
    for plan in plans {
        match &mut plan.kind {
            PlanKind::StagePlot(s) => {
                for it in &mut s.items {
                    it.inputs.retain(|i| inputs.contains(i));
                }
            }
            PlanKind::Intercom(i) => i.prune(),
            PlanKind::Table(t) => t.prune(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::parse_clock;

    #[test]
    fn run_of_show_times_follow_on() {
        let r = RunOfShow {
            rows: vec![
                Cue { start: parse_clock("14:00"), duration: Some(3600.0), ..Cue::default() },
                Cue { duration: Some(1800.0), ..Cue::default() },
                Cue { start: parse_clock("16:00"), ..Cue::default() },
                Cue { duration: Some(600.0), ..Cue::default() },
            ],
            ..RunOfShow::default()
        };
        let t = run_times(&r);
        assert_eq!(t[1], (parse_clock("15:00"), parse_clock("15:30")));
        assert_eq!(t[2], (parse_clock("16:00"), parse_clock("16:00")));
        assert_eq!(t[3].1, parse_clock("16:10"));
        assert_eq!(run_times(&RunOfShow { rows: vec![Cue::default()], ..RunOfShow::default() })[0], (None, None));
    }

    #[test]
    fn packing_list_counts_and_keeps_your_rows() {
        let mut p = crate::demo::band();
        let id = p.add_plan(PlanType::Packlist);
        let (added, _, _) = refresh_packlist(&mut p, id);
        assert!(added > 5);
        let Some(Plan { kind: PlanKind::Packlist(k), .. }) = p.plan_mut(id) else { panic!() };
        k.rows.push(PackRow { id: 999_999, what: "Gaffa tape".into(), count: 4, ..PackRow::default() });
        let xlr = k.rows.iter_mut().find(|r| r.what == "XLR").unwrap();
        xlr.packed = true;
        let before = xlr.count;
        // One more input: the XLR count follows, the mark stays.
        let mut extra = p.patch.inputs[0].clone();
        extra.id = 888_888;
        p.patch.inputs.push(extra);
        let (_, changed, _) = refresh_packlist(&mut p, id);
        assert!(changed >= 1);
        let Some(Plan { kind: PlanKind::Packlist(k), .. }) = p.plan(id) else { panic!() };
        let xlr = k.rows.iter().find(|r| r.what == "XLR").unwrap();
        assert_eq!(xlr.count, before + 1);
        assert!(xlr.packed);
        assert!(k.rows.iter().any(|r| r.what == "Gaffa tape"));
    }

    #[test]
    fn mic_plan_gear_is_counted() {
        let p = crate::demo::musical();
        let items = counted_items(&p);
        let tx = items.iter().find(|x| x.1 == "Wireless").unwrap();
        assert_eq!(tx.3, 16);
        assert!(tx.4.contains("2 spare"));
        assert!(items.iter().any(|x| x.1 == "Batteries"));
    }

    #[test]
    fn free_values_are_checked() {
        let mut col = FreeCol { kind: FreeKind::Choice, options: "Red, Green".into(), ..FreeCol::default() };
        assert_eq!(free_value(&col, "green").unwrap(), "Green");
        assert!(free_value(&col, "blue").is_err());
        col.kind = FreeKind::Time;
        assert_eq!(free_value(&col, "2015").unwrap(), "20:15");
        col.kind = FreeKind::Check;
        assert_eq!(free_value(&col, "x").unwrap(), "yes");
        col.kind = FreeKind::Number;
        assert_eq!(free_value(&col, "2,5").unwrap(), "2.5");
    }

    #[test]
    fn duplicating_an_intercom_plan_keeps_its_matrix() {
        let mut p = Project::default();
        let id = p.add_plan(PlanType::Intercom);
        let ch = p.alloc_id();
        let u = p.alloc_id();
        if let Some(Plan { kind: PlanKind::Intercom(i), .. }) = p.plan_mut(id) {
            i.channels.push(IcChannel { id: ch, name: "A".into(), ..IcChannel::default() });
            let mut user = IcUser { id: u, name: "SM".into(), ..IcUser::default() };
            user.set(ch, Some(Access::Talk));
            i.people.push(user);
        }
        let copy = p.duplicate_plan(id).unwrap();
        let Some(Plan { kind: PlanKind::Intercom(i), .. }) = p.plan(copy) else { panic!() };
        assert_ne!(i.channels[0].id, ch);
        assert_eq!(i.people[0].on(i.channels[0].id), Some(Access::Talk));
        let text = crate::project::to_string(&p).unwrap();
        assert_eq!(crate::project::parse(&text).unwrap(), p);
    }
}
