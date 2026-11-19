//! The mic plan: which transmitter is on which role in which scene, swaps,
//! spares, DCA programming, battery changes, frequencies and the checks.
//!
//! The plan is kept small and by reference: every role has its usual
//! transmitter (its "track", optionally with a B mic), exceptions are set per
//! scene. Who wears it follows from the casting (or the cast of the day), who
//! is on stage from the scenes. Swaps, the matrix and every check are worked
//! out from that ([`MicView`]), so a change shows up everywhere at once.

use crate::model::{Id, Project, TxStatus};
use crate::time::format_duration;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where the capsule sits (suggestions; any text is kept).
pub const POSITIONS: [&str; 7] = ["", "forehead", "hairline", "cheek", "ear", "wig", "costume"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MicPlan {
    pub revision: Revision,
    /// The usual transmitter of each role.
    pub tracks: Vec<Track>,
    /// Capsule and position per performer.
    pub fittings: Vec<Fitting>,
    /// Per scene exceptions from the usual transmitter.
    pub overrides: Vec<MicOverride>,
    /// What is noted about a swap (where, who does it).
    pub swap_notes: Vec<SwapNote>,
    /// Casts of the day ("Cast B", "Tuesday matinee").
    pub variants: Vec<CastVariant>,
    /// The cast the plan is shown and printed for (`None` = first cast).
    pub variant: Option<Id>,
    /// A swap needs at least this long (seconds).
    pub min_swap: f64,
    /// Number of DCAs on the desk.
    pub dca_count: u32,
    pub dca: Vec<DcaAssign>,
    pub battery: Vec<BatteryRule>,
    /// Check third order intermodulation (2·f1 − f2).
    pub im3: bool,
    /// An intermodulation product this close to a carrier is a problem (kHz).
    pub im3_guard_khz: f64,
}

impl Default for MicPlan {
    fn default() -> Self {
        Self {
            revision: Revision::default(),
            tracks: Vec::new(),
            fittings: Vec::new(),
            overrides: Vec::new(),
            swap_notes: Vec::new(),
            variants: Vec::new(),
            variant: None,
            min_swap: 60.0,
            dca_count: 8,
            dca: Vec::new(),
            battery: Vec::new(),
            im3: false,
            im3_guard_khz: 100.0,
        }
    }
}

/// Revision of a plan, printed in the foot. `base` is what the plan looked
/// like at the revision before, so the print can mark what changed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Revision {
    pub number: u32,
    /// "2026-11-19".
    pub date: String,
    /// The plan when this revision was issued.
    pub issued: BTreeMap<String, String>,
    /// The plan at the revision before (changes are marked against it).
    pub base: BTreeMap<String, String>,
}

impl Revision {
    /// Issue the next revision with today's state of the plan.
    pub fn issue(&mut self, now: BTreeMap<String, String>, date: String) {
        self.number += 1;
        self.date = date;
        self.base = std::mem::replace(&mut self.issued, now);
    }

    /// Keys whose value differs from the revision before (empty before the
    /// second revision).
    pub fn changed(&self, now: &BTreeMap<String, String>) -> Vec<String> {
        if self.base.is_empty() {
            return Vec::new();
        }
        let mut keys: Vec<String> = now
            .iter()
            .filter(|(k, v)| self.base.get(*k) != Some(*v))
            .map(|(k, _)| k.clone())
            .collect();
        keys.extend(
            self.base
                .iter()
                .filter(|(k, v)| !v.is_empty() && !now.contains_key(*k))
                .map(|(k, _)| k.clone()),
        );
        keys
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Track {
    pub role: Id,
    pub tx: Option<Id>,
    /// Second transmitter (A/B miking).
    pub tx_b: Option<Id>,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Fitting {
    pub person: Id,
    pub capsule: Option<Id>,
    pub capsule_b: Option<Id>,
    /// "forehead", "cheek", "ear", "wig", "costume" or free text.
    pub position: String,
    pub notes: String,
}

/// In `scene`, `role` wears `tx` instead of its usual one (`None`: no mic).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(default)]
pub struct MicOverride {
    pub scene: Id,
    pub role: Id,
    pub tx: Option<Id>,
}

/// Notes on the swap of `tx` before `scene`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct SwapNote {
    pub tx: Id,
    pub scene: Id,
    /// "Quick change SR".
    pub location: String,
    /// Who swaps it: "A2", "Mic wrangler SL".
    pub by: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct CastVariant {
    pub id: Id,
    pub name: String,
    pub notes: String,
    /// Who plays a role in this cast (roles not listed: first cast).
    pub cast: Vec<VariantCast>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(default)]
pub struct VariantCast {
    pub role: Id,
    pub person: Id,
}

/// What a DCA holds in a scene (line by line mixing).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct DcaAssign {
    pub scene: Id,
    /// 1-based.
    pub dca: u32,
    pub label: String,
    pub roles: Vec<Id>,
}

/// When a transmitter gets new batteries.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct BatteryRule {
    pub tx: Id,
    pub before_show: bool,
    pub interval: bool,
    /// Also after this scene.
    pub after_scene: Option<Id>,
    pub notes: String,
}

impl MicPlan {
    pub fn track(&self, role: Id) -> Option<&Track> {
        self.tracks.iter().find(|t| t.role == role)
    }

    pub fn track_mut(&mut self, role: Id) -> &mut Track {
        if let Some(i) = self.tracks.iter().position(|t| t.role == role) {
            return &mut self.tracks[i];
        }
        self.tracks.push(Track {
            role,
            ..Track::default()
        });
        let last = self.tracks.len() - 1;
        &mut self.tracks[last]
    }

    pub fn fitting(&self, person: Id) -> Option<&Fitting> {
        self.fittings.iter().find(|f| f.person == person)
    }

    pub fn fitting_mut(&mut self, person: Id) -> &mut Fitting {
        if let Some(i) = self.fittings.iter().position(|f| f.person == person) {
            return &mut self.fittings[i];
        }
        self.fittings.push(Fitting {
            person,
            ..Fitting::default()
        });
        let last = self.fittings.len() - 1;
        &mut self.fittings[last]
    }

    pub fn swap_note(&self, tx: Id, scene: Id) -> Option<&SwapNote> {
        self.swap_notes.iter().find(|n| n.tx == tx && n.scene == scene)
    }

    pub fn swap_note_mut(&mut self, tx: Id, scene: Id) -> &mut SwapNote {
        if let Some(i) = self
            .swap_notes
            .iter()
            .position(|n| n.tx == tx && n.scene == scene)
        {
            return &mut self.swap_notes[i];
        }
        self.swap_notes.push(SwapNote {
            tx,
            scene,
            ..SwapNote::default()
        });
        let last = self.swap_notes.len() - 1;
        &mut self.swap_notes[last]
    }

    pub fn battery_rule(&self, tx: Id) -> Option<&BatteryRule> {
        self.battery.iter().find(|b| b.tx == tx)
    }

    pub fn battery_mut(&mut self, tx: Id) -> &mut BatteryRule {
        if let Some(i) = self.battery.iter().position(|b| b.tx == tx) {
            return &mut self.battery[i];
        }
        self.battery.push(BatteryRule {
            tx,
            before_show: true,
            ..BatteryRule::default()
        });
        let last = self.battery.len() - 1;
        &mut self.battery[last]
    }

    pub fn dca_cell(&self, scene: Id, dca: u32) -> Option<&DcaAssign> {
        self.dca.iter().find(|d| d.scene == scene && d.dca == dca)
    }

    /// The transmitter of `role` in `scene` (the exception, else the usual one).
    pub fn tx_in(&self, scene: Id, role: Id) -> Option<Id> {
        match self
            .overrides
            .iter()
            .find(|o| o.scene == scene && o.role == role)
        {
            Some(o) => o.tx,
            None => self.track(role).and_then(|t| t.tx),
        }
    }

    /// Put `role` on `tx` in `scene` (an exception only where it differs).
    pub fn set_tx_in(&mut self, scene: Id, role: Id, tx: Option<Id>) {
        self.overrides.retain(|o| !(o.scene == scene && o.role == role));
        let usual = self.track(role).and_then(|t| t.tx);
        if usual != tx {
            self.overrides.push(MicOverride { scene, role, tx });
        }
    }

    pub fn variant(&self) -> Option<&CastVariant> {
        let id = self.variant?;
        self.variants.iter().find(|v| v.id == id)
    }

    /// Remove references to things that no longer exist.
    pub fn prune(&mut self, people: &[Id], roles: &[Id], scenes: &[Id], txs: &[Id], capsules: &[Id]) {
        let tx_ok = |t: &Option<Id>| t.is_none_or(|t| txs.contains(&t));
        self.tracks.retain(|t| roles.contains(&t.role));
        for t in &mut self.tracks {
            if !tx_ok(&t.tx) {
                t.tx = None;
            }
            if !tx_ok(&t.tx_b) {
                t.tx_b = None;
            }
        }
        self.fittings.retain(|f| people.contains(&f.person));
        for f in &mut self.fittings {
            if f.capsule.is_some_and(|c| !capsules.contains(&c)) {
                f.capsule = None;
            }
            if f.capsule_b.is_some_and(|c| !capsules.contains(&c)) {
                f.capsule_b = None;
            }
        }
        self.overrides.retain(|o| {
            scenes.contains(&o.scene) && roles.contains(&o.role) && tx_ok(&o.tx)
        });
        self.swap_notes
            .retain(|n| txs.contains(&n.tx) && scenes.contains(&n.scene));
        for v in &mut self.variants {
            v.cast
                .retain(|c| roles.contains(&c.role) && people.contains(&c.person));
        }
        self.dca.retain(|d| scenes.contains(&d.scene));
        for d in &mut self.dca {
            d.roles.retain(|r| roles.contains(r));
        }
        self.battery.retain(|b| txs.contains(&b.tx));
        for b in &mut self.battery {
            if b.after_scene.is_some_and(|s| !scenes.contains(&s)) {
                b.after_scene = None;
            }
        }
    }
}

/// Who plays `role` in the cast the plan is set to.
pub fn performer(p: &Project, role: Id) -> Option<Id> {
    if let Some(v) = p.mic.variant() {
        if let Some(c) = v.cast.iter().find(|c| c.role == role) {
            return Some(c.person);
        }
    }
    p.role(role).and_then(|r| r.lead())
}

/// Start and end of every scene in seconds from curtain up, where known: a
/// scene without a start begins when the one before ends.
pub fn timeline(p: &Project) -> Vec<(Option<f64>, Option<f64>)> {
    let mut out = Vec::with_capacity(p.scenes.len());
    let mut prev_end: Option<f64> = None;
    for s in &p.scenes {
        let start = s.start.or(prev_end);
        let end = match (start, s.duration) {
            (Some(a), Some(d)) => Some(a + d),
            _ => None,
        };
        out.push((start, end));
        prev_end = end;
    }
    out
}

/// Somebody wearing a transmitter in a scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wear {
    pub role: Id,
    pub person: Option<Id>,
    /// The B mic of the role.
    pub b: bool,
}

/// A transmitter changes hands between two scenes.
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    pub tx: Id,
    /// Index of the last scene of the one before.
    pub after: usize,
    /// Index of the first scene of the next one.
    pub before: usize,
    pub from: Wear,
    pub to: Wear,
    /// Time between the two (seconds), where the scene times are known.
    pub window: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// Two people on one transmitter in the same scene.
    DoubleBooked { scene: usize, tx: Id, roles: Vec<Id> },
    /// A role is on stage without a mic.
    NoMic { scene: usize, role: Id },
    /// A role is on stage but nobody plays it (in this cast).
    NoPerformer { role: Id },
    /// One person in two roles on stage at once.
    PersonTwice { scene: usize, person: Id, roles: Vec<Id> },
    SwapTooShort { tx: Id, scene: usize, window: f64, min: f64 },
    /// A spare or defect transmitter is in the plan.
    NotActive { tx: Id, status: TxStatus },
    DuplicateFrequency { a: Id, b: Id, mhz: f64 },
    /// 2·f(a) − f(b) lands on `victim`.
    Intermod { a: Id, b: Id, victim: Id, mhz: f64 },
}

/// Short label of a transmitter: its channel, else its name.
pub fn tx_label(p: &Project, tx: Id) -> String {
    match p.transmitters.iter().find(|t| t.id == tx) {
        Some(t) if !t.channel.trim().is_empty() => t.channel.trim().to_string(),
        Some(t) if !t.name.trim().is_empty() => t.name.trim().to_string(),
        Some(t) => format!("#{}", t.id),
        None => "?".into(),
    }
}

/// Transmitter by its channel or name (case-insensitive).
pub fn tx_named(p: &Project, text: &str) -> Option<Id> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    p.transmitters
        .iter()
        .find(|x| x.channel.trim().eq_ignore_ascii_case(t))
        .or_else(|| p.transmitters.iter().find(|x| x.name.trim().eq_ignore_ascii_case(t)))
        .or_else(|| {
            let n = t.strip_prefix('#')?.parse::<Id>().ok()?;
            p.transmitters.iter().find(|x| x.id == n)
        })
        .map(|x| x.id)
}

fn scene_label(p: &Project, i: usize) -> String {
    match p.scenes.get(i) {
        Some(s) if !s.number.trim().is_empty() => s.number.trim().to_string(),
        Some(s) if !s.title.trim().is_empty() => s.title.trim().to_string(),
        _ => format!("scene {}", i + 1),
    }
}

fn role_name(p: &Project, r: Id) -> String {
    p.role(r).map(|x| x.name.clone()).unwrap_or_else(|| "?".into())
}

fn person_name(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.person(i))
        .map(|x| x.name.clone())
        .unwrap_or_else(|| "nobody".into())
}

impl Problem {
    /// One line for the list of checks.
    pub fn message(&self, p: &Project) -> String {
        match self {
            Problem::DoubleBooked { scene, tx, roles } => format!(
                "Scene {}: transmitter {} is on {} at once",
                scene_label(p, *scene),
                tx_label(p, *tx),
                roles.iter().map(|r| role_name(p, *r)).collect::<Vec<_>>().join(" and ")
            ),
            Problem::NoMic { scene, role } => format!(
                "Scene {}: {} ({}) is on stage without a mic",
                scene_label(p, *scene),
                role_name(p, *role),
                person_name(p, performer(p, *role))
            ),
            Problem::NoPerformer { role } => {
                format!("{} is on stage but nobody plays the role", role_name(p, *role))
            }
            Problem::PersonTwice { scene, person, roles } => format!(
                "Scene {}: {} plays {} at the same time",
                scene_label(p, *scene),
                person_name(p, Some(*person)),
                roles.iter().map(|r| role_name(p, *r)).collect::<Vec<_>>().join(" and ")
            ),
            Problem::SwapTooShort { tx, scene, window, min } => format!(
                "Swap of transmitter {} before scene {}: only {} (at least {})",
                tx_label(p, *tx),
                scene_label(p, *scene),
                format_duration(*window),
                format_duration(*min)
            ),
            Problem::NotActive { tx, status } => format!(
                "Transmitter {} is marked {} but in the plan",
                tx_label(p, *tx),
                status.label()
            ),
            Problem::DuplicateFrequency { a, b, mhz } => format!(
                "Transmitters {} and {} are both on {} MHz",
                tx_label(p, *a),
                tx_label(p, *b),
                crate::time::format_number(*mhz)
            ),
            Problem::Intermod { a, b, victim, mhz } => format!(
                "Intermodulation of {} and {} ({} MHz) hits {}",
                tx_label(p, *a),
                tx_label(p, *b),
                crate::time::format_number(*mhz),
                tx_label(p, *victim)
            ),
        }
    }

    /// The scene and transmitter it is about, to jump there.
    pub fn place(&self) -> (Option<usize>, Option<Id>) {
        match self {
            Problem::DoubleBooked { scene, tx, .. } => (Some(*scene), Some(*tx)),
            Problem::NoMic { scene, .. } | Problem::PersonTwice { scene, .. } => (Some(*scene), None),
            Problem::SwapTooShort { tx, scene, .. } => (Some(*scene), Some(*tx)),
            Problem::NotActive { tx, .. } => (None, Some(*tx)),
            Problem::DuplicateFrequency { a, .. } | Problem::Intermod { a, .. } => (None, Some(*a)),
            Problem::NoPerformer { .. } => (None, None),
        }
    }
}

/// The plan worked out for the current project: the matrix, swaps and
/// problems. Cheap to build (scenes × roles), built again after each change.
#[derive(Clone, Debug, Default)]
pub struct MicView {
    /// Transmitters shown as columns: all but defect ones, plus defect ones
    /// still in the plan.
    pub txs: Vec<Id>,
    /// `cells[scene][column]`: who wears that transmitter in the scene.
    pub cells: Vec<Vec<Vec<Wear>>>,
    pub swaps: Vec<Swap>,
    pub problems: Vec<Problem>,
    pub times: Vec<(Option<f64>, Option<f64>)>,
}

impl MicView {
    pub fn build(p: &Project) -> MicView {
        let plan = &p.mic;
        let times = timeline(p);
        let mut used: Vec<Id> = Vec::new();
        // Who wears what, per scene and transmitter id.
        let mut wear: Vec<BTreeMap<Id, Vec<Wear>>> = Vec::with_capacity(p.scenes.len());
        let mut problems = Vec::new();
        let mut no_performer: Vec<Id> = Vec::new();
        for (si, s) in p.scenes.iter().enumerate() {
            let mut m: BTreeMap<Id, Vec<Wear>> = BTreeMap::new();
            let mut people: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
            for &role in &s.roles {
                let person = performer(p, role);
                match person {
                    Some(x) => people.entry(x).or_default().push(role),
                    None => {
                        if !no_performer.contains(&role) {
                            no_performer.push(role);
                        }
                    }
                }
                let a = plan.tx_in(s.id, role);
                let b = plan.track(role).and_then(|t| t.tx_b);
                if a.is_none() && b.is_none() {
                    problems.push(Problem::NoMic { scene: si, role });
                }
                for (tx, is_b) in [(a, false), (b, true)] {
                    if let Some(tx) = tx {
                        m.entry(tx).or_default().push(Wear { role, person, b: is_b });
                        if !used.contains(&tx) {
                            used.push(tx);
                        }
                    }
                }
            }
            for (tx, w) in &m {
                // People (or, where nobody is cast, roles) on this transmitter.
                let mut who: Vec<(bool, Id)> = w
                    .iter()
                    .map(|x| x.person.map_or((false, x.role), |id| (true, id)))
                    .collect();
                who.sort_unstable();
                who.dedup();
                if who.len() > 1 {
                    problems.push(Problem::DoubleBooked {
                        scene: si,
                        tx: *tx,
                        roles: w.iter().map(|x| x.role).collect(),
                    });
                }
            }
            for (person, roles) in people {
                if roles.len() > 1 {
                    problems.push(Problem::PersonTwice { scene: si, person, roles });
                }
            }
            wear.push(m);
        }
        for role in no_performer {
            problems.push(Problem::NoPerformer { role });
        }

        let txs: Vec<Id> = p
            .transmitters
            .iter()
            .filter(|t| t.status != TxStatus::Defect || used.contains(&t.id))
            .map(|t| t.id)
            .collect();
        for t in &p.transmitters {
            if t.status != TxStatus::Active && used.contains(&t.id) {
                problems.push(Problem::NotActive { tx: t.id, status: t.status });
            }
        }

        // Swaps: the wearer of a transmitter changes between two uses.
        let mut swaps = Vec::new();
        for &tx in &txs {
            let mut last: Option<(usize, Wear)> = None;
            for (si, m) in wear.iter().enumerate() {
                let Some(w) = m.get(&tx).and_then(|w| w.first()).copied() else {
                    continue;
                };
                if let Some((li, lw)) = last {
                    let same = match (lw.person, w.person) {
                        (Some(a), Some(b)) => a == b,
                        _ => lw.role == w.role,
                    };
                    if !same {
                        let window = match (times[li].1, times[si].0) {
                            (Some(end), Some(start)) => Some((start - end).max(0.0)),
                            _ => None,
                        };
                        if let Some(win) = window {
                            if win < plan.min_swap {
                                problems.push(Problem::SwapTooShort {
                                    tx,
                                    scene: si,
                                    window: win,
                                    min: plan.min_swap,
                                });
                            }
                        }
                        swaps.push(Swap { tx, after: li, before: si, from: lw, to: w, window });
                    }
                }
                last = Some((si, w));
            }
        }
        swaps.sort_by_key(|s| (s.before, txs.iter().position(|t| *t == s.tx)));

        problems.extend(frequency_problems(p));

        let cells = wear
            .iter()
            .map(|m| txs.iter().map(|t| m.get(t).cloned().unwrap_or_default()).collect())
            .collect();
        MicView { txs, cells, swaps, problems, times }
    }

    pub fn column_of(&self, tx: Id) -> Option<usize> {
        self.txs.iter().position(|t| *t == tx)
    }

    /// The swap that hands `tx` over before scene `scene`, if any.
    pub fn swap_at(&self, scene: usize, tx: Id) -> Option<&Swap> {
        self.swaps.iter().find(|s| s.before == scene && s.tx == tx)
    }

    /// Transmitters worn by `person` over the show, with the scenes.
    pub fn person_txs(&self, person: Id) -> Vec<(Id, Vec<usize>, bool)> {
        let mut out: Vec<(Id, Vec<usize>, bool)> = Vec::new();
        for (si, row) in self.cells.iter().enumerate() {
            for (ci, wears) in row.iter().enumerate() {
                for w in wears {
                    if w.person == Some(person) {
                        let tx = self.txs[ci];
                        match out.iter_mut().find(|x| x.0 == tx) {
                            Some(x) => {
                                if !x.1.contains(&si) {
                                    x.1.push(si);
                                }
                            }
                            None => out.push((tx, vec![si], w.b)),
                        }
                    }
                }
            }
        }
        out
    }

    /// The plan as key → text, to mark what changed between revisions.
    pub fn snapshot(&self, p: &Project) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for (si, row) in self.cells.iter().enumerate() {
            let Some(scene) = p.scenes.get(si) else { continue };
            for (ci, w) in row.iter().enumerate() {
                if !w.is_empty() {
                    m.insert(cell_key(scene.id, self.txs[ci]), wears_text(p, w));
                }
            }
        }
        m
    }
}

/// Key of a matrix cell in revision snapshots.
pub fn cell_key(scene: Id, tx: Id) -> String {
    format!("{scene}:{tx}")
}

/// "Mara, Lena (B)".
pub fn wears_text(p: &Project, w: &[Wear]) -> String {
    w.iter()
        .map(|x| {
            let n = role_name(p, x.role);
            if x.b {
                format!("{n} (B)")
            } else {
                n
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Duplicate frequencies and (if switched on) third order intermodulation,
/// among transmitters that are not defect.
pub fn frequency_problems(p: &Project) -> Vec<Problem> {
    let live: Vec<(Id, f64)> = p
        .transmitters
        .iter()
        .filter(|t| t.status != TxStatus::Defect)
        .filter_map(|t| t.frequency.map(|f| (t.id, f)))
        .collect();
    let mut out = Vec::new();
    for (i, (a, fa)) in live.iter().enumerate() {
        for (b, fb) in &live[i + 1..] {
            if (fa - fb).abs() < 0.000_5 {
                out.push(Problem::DuplicateFrequency { a: *a, b: *b, mhz: *fa });
            }
        }
    }
    if p.mic.im3 {
        let guard = p.mic.im3_guard_khz.max(0.0) / 1000.0;
        for (a, fa) in &live {
            for (b, fb) in &live {
                if a == b || (fa - fb).abs() < 0.000_5 {
                    continue;
                }
                let product = 2.0 * fa - fb;
                for (v, fv) in &live {
                    if v != a && v != b && (product - fv).abs() <= guard {
                        out.push(Problem::Intermod { a: *a, b: *b, victim: *v, mhz: product });
                    }
                }
            }
        }
    }
    out
}

/// Transmitter `broken` is out: `spare` takes over everywhere (usual
/// transmitters, B mics, exceptions, swap notes, battery plan). `broken` is
/// marked defect and `spare` active.
pub fn replace_tx(p: &mut Project, broken: Id, spare: Id) -> Result<(), String> {
    if broken == spare {
        return Err("choose a different transmitter to take over".into());
    }
    if !p.transmitters.iter().any(|t| t.id == spare) || !p.transmitters.iter().any(|t| t.id == broken) {
        return Err("no such transmitter".into());
    }
    let swap = |t: &mut Option<Id>| {
        if *t == Some(broken) {
            *t = Some(spare);
        }
    };
    for t in &mut p.mic.tracks {
        swap(&mut t.tx);
        swap(&mut t.tx_b);
    }
    for o in &mut p.mic.overrides {
        swap(&mut o.tx);
    }
    for n in &mut p.mic.swap_notes {
        if n.tx == broken {
            n.tx = spare;
        }
    }
    if !p.mic.battery.iter().any(|b| b.tx == spare) {
        for b in &mut p.mic.battery {
            if b.tx == broken {
                b.tx = spare;
            }
        }
    }
    for t in &mut p.transmitters {
        if t.id == broken {
            t.status = TxStatus::Defect;
        } else if t.id == spare {
            t.status = TxStatus::Active;
        }
    }
    Ok(())
}

/// "Lead vocals: Mara, Lena" ⇄ label and roles. Without a colon, the
/// text is the role list and the label stays empty.
pub fn parse_dca(p: &Project, text: &str) -> Result<(String, Vec<Id>), String> {
    let t = text.trim();
    let (label, list) = match t.split_once(':') {
        Some((l, r)) => (l.trim().to_string(), r),
        None => (String::new(), t),
    };
    let roles = crate::sheet::parse_roles(p, list)?;
    Ok((label, roles))
}

pub fn dca_text(p: &Project, d: &DcaAssign) -> String {
    let roles = crate::sheet::roles_text(p, &d.roles);
    if d.label.is_empty() {
        roles
    } else {
        format!("{}: {roles}", d.label)
    }
}

/// Desk channels on a DCA in a scene (the transmitters of its roles there).
pub fn dca_channels(p: &Project, scene: Id, d: &DcaAssign) -> Vec<String> {
    let mut out = Vec::new();
    for r in &d.roles {
        let a = p.mic.tx_in(scene, *r);
        let b = p.mic.track(*r).and_then(|t| t.tx_b);
        for tx in [a, b].into_iter().flatten() {
            let l = tx_label(p, tx);
            if !out.contains(&l) {
                out.push(l);
            }
        }
    }
    // Numbers in order, then the rest ("S1").
    out.sort_by(|a, b| match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        _ => a.cmp(b),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CastKind, Casting, Person, Role, Scene, Transmitter};

    /// Anna plays Mara, Ben plays Tobias, Cleo covers Mara. Two TX.
    fn show() -> Project {
        let mut p = Project::default();
        for (name, short) in [("Anna", "AN"), ("Ben", "BE"), ("Cleo", "CL")] {
            let id = p.alloc_id();
            p.cast.push(Person { id, name: name.into(), short: short.into(), ..Person::default() });
        }
        let (anna, ben, cleo) = (p.cast[0].id, p.cast[1].id, p.cast[2].id);
        for (name, lead) in [("Mara", anna), ("Tobias", ben)] {
            let id = p.alloc_id();
            p.roles.push(Role {
                id,
                name: name.into(),
                casting: vec![Casting { person: lead, kind: CastKind::Lead }],
                ..Role::default()
            });
        }
        p.roles[0].casting.push(Casting { person: cleo, kind: CastKind::Cover });
        for (ch, f) in [("1", 606.1), ("2", 607.3)] {
            let id = p.alloc_id();
            p.transmitters.push(Transmitter {
                id,
                channel: ch.into(),
                frequency: Some(f),
                ..Transmitter::default()
            });
        }
        let (el, fi) = (p.roles[0].id, p.roles[1].id);
        for (n, roles, start, dur) in [
            ("1", vec![el], 0.0, 300.0),
            ("2", vec![fi], 330.0, 200.0),
            ("3", vec![el, fi], 600.0, 100.0),
        ] {
            let id = p.alloc_id();
            p.scenes.push(Scene {
                id,
                number: n.into(),
                roles,
                start: Some(start),
                duration: Some(dur),
                ..Scene::default()
            });
        }
        p
    }

    #[test]
    fn roles_without_a_mic_and_double_booking() {
        let mut p = show();
        let (el, fi) = (p.roles[0].id, p.roles[1].id);
        let (t1, t2) = (p.transmitters[0].id, p.transmitters[1].id);
        let v = MicView::build(&p);
        assert_eq!(v.problems.iter().filter(|x| matches!(x, Problem::NoMic { .. })).count(), 4);
        p.mic.track_mut(el).tx = Some(t1);
        p.mic.track_mut(fi).tx = Some(t1);
        let v = MicView::build(&p);
        assert!(v.problems.iter().all(|x| !matches!(x, Problem::NoMic { .. })));
        // Scene 3: both on transmitter 1.
        assert!(v
            .problems
            .iter()
            .any(|x| matches!(x, Problem::DoubleBooked { scene: 2, tx, .. } if *tx == t1)));
        // One exception fixes it.
        p.mic.set_tx_in(p.scenes[2].id, fi, Some(t2));
        let v = MicView::build(&p);
        assert!(!v.problems.iter().any(|x| matches!(x, Problem::DoubleBooked { .. })));
        assert_eq!(v.cells[2][1][0].role, fi);
        // Setting it back to the usual one removes the exception.
        p.mic.set_tx_in(p.scenes[2].id, fi, Some(t1));
        assert!(p.mic.overrides.is_empty());
    }

    #[test]
    fn swaps_and_their_window() {
        let mut p = show();
        let (el, fi) = (p.roles[0].id, p.roles[1].id);
        let t1 = p.transmitters[0].id;
        let t2 = p.transmitters[1].id;
        p.mic.track_mut(el).tx = Some(t1);
        p.mic.track_mut(fi).tx = Some(t1);
        p.mic.set_tx_in(p.scenes[2].id, fi, Some(t2));
        let v = MicView::build(&p);
        // Transmitter 1: Anna (1) → Ben (2) → Anna (3).
        let s: Vec<_> = v.swaps.iter().filter(|s| s.tx == t1).collect();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].after, s[0].before), (0, 1));
        assert_eq!(s[0].window, Some(30.0));
        assert_eq!(s[1].window, Some(70.0));
        // 30 s is below the minute.
        let short: Vec<_> = v
            .problems
            .iter()
            .filter(|x| matches!(x, Problem::SwapTooShort { .. }))
            .collect();
        assert_eq!(short.len(), 1);
        p.mic.min_swap = 20.0;
        assert!(!MicView::build(&p)
            .problems
            .iter()
            .any(|x| matches!(x, Problem::SwapTooShort { .. })));
    }

    #[test]
    fn cast_of_the_day_changes_who_wears_it() {
        let mut p = show();
        let el = p.roles[0].id;
        let t1 = p.transmitters[0].id;
        let cleo = p.cast[2].id;
        p.mic.track_mut(el).tx = Some(t1);
        let id = p.alloc_id();
        p.mic.variants.push(CastVariant {
            id,
            name: "Cover show".into(),
            cast: vec![VariantCast { role: el, person: cleo }],
            ..CastVariant::default()
        });
        assert_eq!(MicView::build(&p).cells[0][0][0].person, Some(p.cast[0].id));
        p.mic.variant = Some(id);
        let v = MicView::build(&p);
        assert_eq!(v.cells[0][0][0].person, Some(cleo));
        assert_eq!(v.person_txs(cleo), vec![(t1, vec![0, 2], false)]);
    }

    #[test]
    fn spare_takes_over_everywhere() {
        let mut p = show();
        let el = p.roles[0].id;
        let (t1, t2) = (p.transmitters[0].id, p.transmitters[1].id);
        p.transmitters[1].status = TxStatus::Spare;
        p.mic.track_mut(el).tx = Some(t1);
        p.mic.set_tx_in(p.scenes[2].id, el, Some(t1));
        p.mic.swap_note_mut(t1, p.scenes[1].id).by = "A2".into();
        assert!(!MicView::build(&p).problems.iter().any(|x| matches!(x, Problem::NotActive { .. })));
        replace_tx(&mut p, t1, t2).unwrap();
        assert_eq!(p.mic.track(el).and_then(|t| t.tx), Some(t2));
        assert_eq!(p.mic.swap_notes[0].tx, t2);
        assert_eq!(p.transmitters[0].status, TxStatus::Defect);
        assert_eq!(p.transmitters[1].status, TxStatus::Active);
        let v = MicView::build(&p);
        assert!(!v.txs.contains(&t1), "a defect transmitter no longer in use is hidden");
        assert!(replace_tx(&mut p, t2, t2).is_err());
        // A spare (or defect) transmitter in the plan is a problem.
        p.transmitters[1].status = TxStatus::Spare;
        assert!(MicView::build(&p)
            .problems
            .iter()
            .any(|x| matches!(x, Problem::NotActive { .. })));
    }

    #[test]
    fn frequencies_and_intermodulation() {
        let mut p = show();
        p.transmitters[1].frequency = Some(606.1);
        let f = frequency_problems(&p);
        assert!(matches!(f[0], Problem::DuplicateFrequency { .. }));
        // 2·606.1 − 606.3 = 605.9.
        p.transmitters[1].frequency = Some(606.3);
        let id = p.alloc_id();
        p.transmitters.push(Transmitter { id, channel: "3".into(), frequency: Some(605.95), ..Transmitter::default() });
        assert!(frequency_problems(&p).is_empty(), "off unless switched on");
        p.mic.im3 = true;
        let f = frequency_problems(&p);
        assert!(f.iter().any(|x| matches!(x, Problem::Intermod { victim, .. } if *victim == id)), "{f:?}");
        p.mic.im3_guard_khz = 10.0;
        assert!(!frequency_problems(&p).iter().any(|x| matches!(x, Problem::Intermod { victim, .. } if *victim == id)));
    }

    #[test]
    fn person_in_two_roles_and_nobody_cast() {
        let mut p = show();
        let fi = p.roles[1].id;
        p.roles[1].casting.clear();
        let v = MicView::build(&p);
        assert!(v.problems.contains(&Problem::NoPerformer { role: fi }));
        p.roles[1].casting.push(Casting { person: p.cast[0].id, kind: CastKind::Lead });
        let v = MicView::build(&p);
        assert!(v.problems.iter().any(|x| matches!(x, Problem::PersonTwice { scene: 2, .. })));
    }

    #[test]
    fn revisions_mark_changes_against_the_one_before() {
        let mut r = Revision::default();
        let snap = |v: &[(&str, &str)]| v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<BTreeMap<_, _>>();
        r.issue(snap(&[("1:1", "Mara")]), "2026-11-01".into());
        assert_eq!(r.number, 1);
        assert!(r.changed(&snap(&[("1:1", "Lena")])).is_empty(), "nothing to compare with yet");
        r.issue(snap(&[("1:1", "Lena"), ("2:1", "Tobias")]), "2026-11-02".into());
        let now = snap(&[("1:1", "Lena"), ("2:1", "Tobias")]);
        let mut c = r.changed(&now);
        c.sort();
        assert_eq!(c, vec!["1:1".to_string(), "2:1".to_string()]);
    }

    #[test]
    fn dca_text_round_trip() {
        let p = show();
        let (label, roles) = parse_dca(&p, "Leads: Mara, tobias").unwrap();
        assert_eq!(label, "Leads");
        assert_eq!(roles.len(), 2);
        let d = DcaAssign { scene: p.scenes[0].id, dca: 1, label, roles };
        assert_eq!(dca_text(&p, &d), "Leads: Mara, Tobias");
        assert!(parse_dca(&p, "Nobody").is_err());
    }
}
