//! The mic plan as tables for the editor: the matrix scene × transmitter,
//! the usual transmitter per role, fittings per performer, swaps, DCAs,
//! batteries, frequencies and casts of the day. Each is a [`Grid`] over a
//! [`MicView`] built for the project as it is now.

use crate::grid::{col, CellMark, ColKind, Column, Grid};
use crate::mic::{wear_color, 
    dca_channels, dca_text, parse_dca, performer, tx_label, tx_named, wears_text, MicView,
    Problem, VariantCast, POSITIONS,
};
use crate::model::{Id, Project, TxStatus};
use crate::sheet::{parse_flag, parse_roles};
use crate::time::{format_duration, format_number};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MicSheet {
    /// Scene × transmitter.
    Plot,
    /// The usual transmitter of each role.
    Tracks,
    /// Capsule and position per performer, and what they wear.
    Performers,
    Swaps,
    Dca,
    Battery,
    Frequencies,
    /// Casts of the day.
    Variants,
}

impl MicSheet {
    pub const ALL: [MicSheet; 8] = [
        MicSheet::Plot,
        MicSheet::Tracks,
        MicSheet::Performers,
        MicSheet::Swaps,
        MicSheet::Variants,
        MicSheet::Dca,
        MicSheet::Battery,
        MicSheet::Frequencies,
    ];

    pub fn title(self) -> &'static str {
        match self {
            MicSheet::Plot => "Mic plot",
            MicSheet::Tracks => "Transmitters per role",
            MicSheet::Performers => "Performers",
            MicSheet::Swaps => "Swaps",
            MicSheet::Dca => "DCAs",
            MicSheet::Battery => "Batteries",
            MicSheet::Frequencies => "Frequencies",
            MicSheet::Variants => "Cast of the day",
        }
    }

    /// One line under the heading.
    pub fn help(self) -> &'static str {
        match self {
            MicSheet::Plot => "Who wears which transmitter in each scene. Bright: on stage; faint with a line: off stage but still wearing it (channel down). Type role names into a cell to put them on that transmitter for the scene; set the usual transmitter in \"Transmitters per role\".",
            MicSheet::Tracks => "The transmitter each role normally wears (and a second one for A/B miking). Exceptions are typed into the mic plot.",
            MicSheet::Performers => "Capsule and where it sits, per performer. The transmitters follow from the plan.",
            MicSheet::Swaps => "Every time a transmitter changes hands, with the time there is. Note where and who swaps it.",
            MicSheet::Dca => "What each DCA holds per scene, typed as \"Label: Role, Role\". The desk channels follow from the plan.",
            MicSheet::Battery => "When each transmitter gets fresh batteries.",
            MicSheet::Frequencies => "All frequencies by band. Edit them in Transmitters.",
            MicSheet::Variants => "Who plays which role in another cast (empty: the first cast). Choose the cast the plan is shown for above.",
        }
    }
}

pub struct MicGrid<'v> {
    pub sheet: MicSheet,
    pub view: &'v MicView,
}

fn scene_no(p: &Project, i: usize) -> String {
    p.scenes.get(i).map(|s| s.number.clone()).unwrap_or_default()
}

fn person_label(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.person(i)).map(|x| x.name.clone()).unwrap_or_default()
}

fn tx_choices(p: &Project) -> Vec<String> {
    std::iter::once(String::new())
        .chain(p.transmitters.iter().map(|t| tx_label(p, t.id)))
        .collect()
}

fn parse_tx(p: &Project, v: &str) -> Result<Option<Id>, String> {
    if v.trim().is_empty() {
        return Ok(None);
    }
    tx_named(p, v)
        .map(Some)
        .ok_or_else(|| format!("no transmitter \"{}\"", v.trim()))
}

/// "C3 MKE2 beige" – the capsule's place in the list, model and colour.
pub fn capsule_label(p: &Project, id: Id) -> String {
    match p.capsules.iter().position(|c| c.id == id) {
        Some(i) => {
            let c = &p.capsules[i];
            let mut s = format!("C{}", i + 1);
            for part in [&c.model, &c.color] {
                if !part.trim().is_empty() {
                    s.push(' ');
                    s.push_str(part.trim());
                }
            }
            s
        }
        None => String::new(),
    }
}

fn parse_capsule(p: &Project, v: &str) -> Result<Option<Id>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if let Some(c) = p
        .capsules
        .iter()
        .find(|c| capsule_label(p, c.id).eq_ignore_ascii_case(t) || (!c.serial.is_empty() && c.serial.eq_ignore_ascii_case(t)))
    {
        return Ok(Some(c.id));
    }
    // "C3" alone.
    let first = t.split_whitespace().next().unwrap_or("");
    if let Some(n) = first
        .strip_prefix(['C', 'c'])
        .and_then(|n| n.parse::<usize>().ok())
    {
        if let Some(c) = p.capsules.get(n.wrapping_sub(1)) {
            return Ok(Some(c.id));
        }
    }
    Err(format!("no capsule \"{t}\""))
}

fn tx_title(p: &Project, tx: Id) -> String {
    let label = tx_label(p, tx);
    match p.transmitters.iter().find(|t| t.id == tx) {
        Some(t) if !t.name.trim().is_empty() && t.name.trim() != label => {
            format!("{label} {}", t.name.trim())
        }
        _ => label,
    }
}

fn freq_rows(p: &Project) -> Vec<Id> {
    let mut v: Vec<&crate::model::Transmitter> = p.transmitters.iter().collect();
    v.sort_by(|a, b| {
        a.band
            .cmp(&b.band)
            .then(a.frequency.unwrap_or(f64::MAX).total_cmp(&b.frequency.unwrap_or(f64::MAX)))
    });
    v.into_iter().map(|t| t.id).collect()
}

impl MicGrid<'_> {
    fn problems_at(&self, scene: Option<usize>, tx: Option<Id>) -> Vec<&Problem> {
        self.view
            .problems
            .iter()
            .filter(|x| {
                let (s, t) = x.place();
                match (scene, tx) {
                    (Some(sc), Some(tx)) => {
                        (s == Some(sc) && t == Some(tx))
                            || (s.is_none() && t == Some(tx) && matches!(x, Problem::NotActive { .. }))
                    }
                    (Some(sc), None) => s == Some(sc) && t.is_none(),
                    (None, Some(tx)) => match x {
                        Problem::DuplicateFrequency { a, b, .. } => *a == tx || *b == tx,
                        Problem::Intermod { a, b, victim, .. } => *a == tx || *b == tx || *victim == tx,
                        Problem::NotActive { tx: t, .. } => *t == tx,
                        _ => false,
                    },
                    (None, None) => false,
                }
            })
            .collect()
    }

    fn variant_of_column(&self, p: &Project, col: usize) -> Option<usize> {
        col.checked_sub(2).filter(|i| *i < p.mic.variants.len())
    }
}

impl Grid for MicGrid<'_> {
    fn key(&self) -> String {
        format!("mic:{:?}", self.sheet)
    }

    fn columns(&self, p: &Project) -> Vec<Column> {
        use ColKind::*;
        let fixed = |t: &str, w: f32| col(t, Fixed, w);
        match self.sheet {
            MicSheet::Plot => {
                let mut c = vec![fixed("No.", 50.0), fixed("Scene", 180.0)];
                for tx in &self.view.txs {
                    c.push(col(tx_title(p, *tx), Text, 110.0));
                }
                c
            }
            MicSheet::Tracks => vec![
                fixed("Role", 170.0),
                fixed("Played by", 160.0),
                col("TX", Choice(tx_choices(p)), 80.0),
                col("B TX", Choice(tx_choices(p)), 80.0),
                col("Notes", Text, 260.0),
            ],
            MicSheet::Performers => {
                let caps: Vec<String> = std::iter::once(String::new())
                    .chain(p.capsules.iter().map(|c| capsule_label(p, c.id)))
                    .collect();
                vec![
                    fixed("Performer", 160.0),
                    fixed("Roles", 180.0),
                    fixed("Transmitters", 140.0),
                    col("Capsule", Choice(caps.clone()), 150.0),
                    col("B capsule", Choice(caps), 130.0),
                    col("Position", Choice(POSITIONS.iter().map(|s| s.to_string()).collect()), 100.0),
                    col("Notes", Text, 220.0),
                ]
            }
            MicSheet::Swaps => vec![
                fixed("Before", 60.0),
                fixed("TX", 50.0),
                fixed("From", 180.0),
                fixed("To", 180.0),
                fixed("Time", 60.0),
                col("Where", Text, 140.0),
                col("By", Text, 110.0),
                col("Notes", Text, 200.0),
            ],
            MicSheet::Dca => {
                let mut c = vec![fixed("No.", 50.0), fixed("Scene", 160.0)];
                for i in 1..=p.mic.dca_count.max(1) {
                    c.push(col(format!("DCA {i}"), Text, 140.0));
                }
                c
            }
            MicSheet::Battery => vec![
                fixed("TX", 50.0),
                fixed("Name", 120.0),
                fixed("Battery", 90.0),
                col("Before show", Choice(vec![String::new(), "yes".into()]), 90.0),
                col("Interval", Choice(vec![String::new(), "yes".into()]), 80.0),
                col("Also after scene", Text, 120.0),
                col("Notes", Text, 240.0),
            ],
            MicSheet::Frequencies => vec![
                fixed("Band", 80.0),
                fixed("TX", 50.0),
                fixed("Name", 130.0),
                fixed("MHz", 90.0),
                fixed("Status", 70.0),
                fixed("Check", 360.0),
            ],
            MicSheet::Variants => {
                let mut c = vec![fixed("Role", 170.0), fixed("First cast", 160.0)];
                for v in &p.mic.variants {
                    let t = if v.name.trim().is_empty() { "Unnamed cast".to_string() } else { v.name.clone() };
                    c.push(col(t, Person, 160.0));
                }
                c
            }
        }
    }

    fn len(&self, p: &Project) -> usize {
        match self.sheet {
            MicSheet::Plot | MicSheet::Dca => p.scenes.len(),
            MicSheet::Tracks | MicSheet::Variants => p.roles.len(),
            MicSheet::Performers => p.cast.len(),
            MicSheet::Swaps => self.view.swaps.len(),
            MicSheet::Battery => self.view.txs.len(),
            MicSheet::Frequencies => p.transmitters.len(),
        }
    }

    fn text(&self, p: &Project, row: usize, c: usize) -> String {
        let plan = &p.mic;
        match self.sheet {
            MicSheet::Plot => match c {
                0 => scene_no(p, row),
                1 => p.scenes.get(row).map(|s| s.title.clone()).unwrap_or_default(),
                _ => self
                    .view
                    .cells
                    .get(row)
                    .and_then(|r| r.get(c - 2))
                    .map(|w| wears_text(p, w))
                    .unwrap_or_default(),
            },
            MicSheet::Tracks => {
                let Some(r) = p.roles.get(row) else { return String::new() };
                let t = plan.track(r.id);
                match c {
                    0 => r.name.clone(),
                    1 => person_label(p, performer(p, r.id)),
                    2 => t.and_then(|t| t.tx).map(|x| tx_label(p, x)).unwrap_or_default(),
                    3 => t.and_then(|t| t.tx_b).map(|x| tx_label(p, x)).unwrap_or_default(),
                    4 => t.map(|t| t.notes.clone()).unwrap_or_default(),
                    _ => String::new(),
                }
            }
            MicSheet::Performers => {
                let Some(x) = p.cast.get(row) else { return String::new() };
                let f = plan.fitting(x.id);
                match c {
                    0 => x.name.clone(),
                    1 => p
                        .roles
                        .iter()
                        .filter(|r| performer(p, r.id) == Some(x.id))
                        .map(|r| r.name.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    2 => self
                        .view
                        .person_txs(x.id)
                        .iter()
                        .map(|(tx, _, b)| {
                            let l = tx_label(p, *tx);
                            if *b {
                                format!("{l} (B)")
                            } else {
                                l
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    3 => f.and_then(|f| f.capsule).map(|c| capsule_label(p, c)).unwrap_or_default(),
                    4 => f.and_then(|f| f.capsule_b).map(|c| capsule_label(p, c)).unwrap_or_default(),
                    5 => f.map(|f| f.position.clone()).unwrap_or_default(),
                    6 => f.map(|f| f.notes.clone()).unwrap_or_default(),
                    _ => String::new(),
                }
            }
            MicSheet::Swaps => {
                let Some(s) = self.view.swaps.get(row) else { return String::new() };
                let who = |w: &crate::mic::Wear| {
                    let role = p.role(w.role).map(|r| r.name.clone()).unwrap_or_default();
                    match w.person.and_then(|i| p.person(i)) {
                        Some(x) => format!("{} ({role})", x.name),
                        None => role,
                    }
                };
                let note = p.scenes.get(s.before).and_then(|sc| plan.swap_note(s.tx, sc.id));
                match c {
                    0 => scene_no(p, s.before),
                    1 => tx_label(p, s.tx),
                    2 => who(&s.from),
                    3 => who(&s.to),
                    4 => s.window.map(format_duration).unwrap_or_default(),
                    5 => note.map(|n| n.location.clone()).unwrap_or_default(),
                    6 => note.map(|n| n.by.clone()).unwrap_or_default(),
                    7 => note.map(|n| n.notes.clone()).unwrap_or_default(),
                    _ => String::new(),
                }
            }
            MicSheet::Dca => {
                let Some(s) = p.scenes.get(row) else { return String::new() };
                match c {
                    0 => s.number.clone(),
                    1 => s.title.clone(),
                    _ => plan
                        .dca_cell(s.id, (c - 1) as u32)
                        .map(|d| dca_text(p, d))
                        .unwrap_or_default(),
                }
            }
            MicSheet::Battery => {
                let Some(tx) = self.view.txs.get(row) else { return String::new() };
                let t = p.transmitters.iter().find(|t| t.id == *tx);
                let rule = plan.battery_rule(*tx);
                let yes = |b: bool| if b { "yes".to_string() } else { String::new() };
                match c {
                    0 => tx_label(p, *tx),
                    1 => t.map(|t| t.name.clone()).unwrap_or_default(),
                    2 => t.map(|t| t.battery.clone()).unwrap_or_default(),
                    3 => yes(rule.is_none_or(|r| r.before_show)),
                    4 => yes(rule.is_some_and(|r| r.interval)),
                    5 => rule
                        .and_then(|r| r.after_scene)
                        .and_then(|s| p.scenes.iter().find(|x| x.id == s))
                        .map(|s| s.number.clone())
                        .unwrap_or_default(),
                    6 => rule.map(|r| r.notes.clone()).unwrap_or_default(),
                    _ => String::new(),
                }
            }
            MicSheet::Frequencies => {
                let rows = freq_rows(p);
                let Some(t) = rows.get(row).and_then(|id| p.transmitters.iter().find(|t| t.id == *id)) else {
                    return String::new();
                };
                match c {
                    0 => t.band.clone(),
                    1 => tx_label(p, t.id),
                    2 => t.name.clone(),
                    3 => t.frequency.map(format_number).unwrap_or_default(),
                    4 => t.status.label().to_string(),
                    5 => {
                        let msgs: Vec<String> = self
                            .problems_at(None, Some(t.id))
                            .iter()
                            .filter(|x| !matches!(x, Problem::NotActive { .. }))
                            .map(|x| x.message(p))
                            .collect();
                        if msgs.is_empty() {
                            if t.frequency.is_some() { "ok".into() } else { String::new() }
                        } else {
                            msgs.join("; ")
                        }
                    }
                    _ => String::new(),
                }
            }
            MicSheet::Variants => {
                let Some(r) = p.roles.get(row) else { return String::new() };
                match c {
                    0 => r.name.clone(),
                    1 => person_label(p, r.lead()),
                    _ => self
                        .variant_of_column(p, c)
                        .and_then(|i| plan.variants[i].cast.iter().find(|x| x.role == r.id))
                        .map(|x| person_label(p, Some(x.person)))
                        .unwrap_or_default(),
                }
            }
        }
    }

    fn set(&self, p: &mut Project, row: usize, c: usize, value: &str) -> Result<(), String> {
        let v = value.trim_end_matches(['\r', '\n']);
        match self.sheet {
            MicSheet::Plot => {
                let (Some(scene), Some(tx)) = (p.scenes.get(row).map(|s| s.id), c.checked_sub(2).and_then(|i| self.view.txs.get(i)).copied()) else {
                    return Ok(());
                };
                // Role names, or people on stage in this scene.
                let mut roles = Vec::new();
                for part in v.split([',', ';']) {
                    let part = part.trim();
                    let part = part.strip_suffix("(B)").map(str::trim).unwrap_or(part);
                    if part.is_empty() {
                        continue;
                    }
                    let role = match p.role_named(part) {
                        Some(r) => r.id,
                        None => {
                            let person = p
                                .person_named(part)
                                .ok_or_else(|| format!("no role or performer named \"{part}\""))?
                                .id;
                            let on: Vec<Id> = p.scenes[row].roles.clone();
                            on.into_iter()
                                .chain(p.roles.iter().map(|r| r.id))
                                .find(|r| performer(p, *r) == Some(person))
                                .ok_or_else(|| format!("{part} plays no role"))?
                        }
                    };
                    if !roles.contains(&role) {
                        roles.push(role);
                    }
                }
                let wearing: Vec<Id> = self
                    .view
                    .cells
                    .get(row)
                    .and_then(|r| r.get(c - 2))
                    .map(|w| w.iter().filter(|x| !x.b).map(|x| x.role).collect())
                    .unwrap_or_default();
                for r in wearing {
                    if !roles.contains(&r) {
                        p.mic.set_tx_in(scene, r, None);
                    }
                }
                for r in roles {
                    if !p.scenes[row].roles.contains(&r) {
                        p.scenes[row].roles.push(r);
                    }
                    p.mic.set_tx_in(scene, r, Some(tx));
                }
            }
            MicSheet::Tracks => {
                let Some(role) = p.roles.get(row).map(|r| r.id) else { return Ok(()) };
                match c {
                    2 => {
                        let tx = parse_tx(p, v)?;
                        p.mic.track_mut(role).tx = tx;
                    }
                    3 => {
                        let tx = parse_tx(p, v)?;
                        p.mic.track_mut(role).tx_b = tx;
                    }
                    4 => p.mic.track_mut(role).notes = v.to_string(),
                    _ => {}
                }
            }
            MicSheet::Performers => {
                let Some(person) = p.cast.get(row).map(|x| x.id) else { return Ok(()) };
                match c {
                    3 => {
                        let cap = parse_capsule(p, v)?;
                        p.mic.fitting_mut(person).capsule = cap;
                    }
                    4 => {
                        let cap = parse_capsule(p, v)?;
                        p.mic.fitting_mut(person).capsule_b = cap;
                    }
                    5 => p.mic.fitting_mut(person).position = v.trim().to_string(),
                    6 => p.mic.fitting_mut(person).notes = v.to_string(),
                    _ => {}
                }
            }
            MicSheet::Swaps => {
                let Some(s) = self.view.swaps.get(row) else { return Ok(()) };
                let Some(scene) = p.scenes.get(s.before).map(|x| x.id) else { return Ok(()) };
                let n = p.mic.swap_note_mut(s.tx, scene);
                match c {
                    5 => n.location = v.trim().to_string(),
                    6 => n.by = v.trim().to_string(),
                    7 => n.notes = v.to_string(),
                    _ => {}
                }
            }
            MicSheet::Dca => {
                let Some(scene) = p.scenes.get(row).map(|s| s.id) else { return Ok(()) };
                if c < 2 {
                    return Ok(());
                }
                let dca = (c - 1) as u32;
                let (label, roles) = parse_dca(p, v)?;
                p.mic.dca.retain(|d| !(d.scene == scene && d.dca == dca));
                if !label.is_empty() || !roles.is_empty() {
                    p.mic.dca.push(crate::mic::DcaAssign { scene, dca, label, roles });
                }
            }
            MicSheet::Battery => {
                let Some(tx) = self.view.txs.get(row).copied() else { return Ok(()) };
                match c {
                    3 => {
                        let f = parse_flag(v)?;
                        p.mic.battery_mut(tx).before_show = f;
                    }
                    4 => {
                        let f = parse_flag(v)?;
                        p.mic.battery_mut(tx).interval = f;
                    }
                    5 => {
                        let t = v.trim();
                        let scene = if t.is_empty() {
                            None
                        } else {
                            Some(
                                p.scenes
                                    .iter()
                                    .find(|s| s.number.trim().eq_ignore_ascii_case(t) || s.title.trim().eq_ignore_ascii_case(t))
                                    .ok_or_else(|| format!("no scene \"{t}\""))?
                                    .id,
                            )
                        };
                        p.mic.battery_mut(tx).after_scene = scene;
                    }
                    6 => p.mic.battery_mut(tx).notes = v.to_string(),
                    _ => {}
                }
            }
            MicSheet::Frequencies => {}
            MicSheet::Variants => {
                let Some(role) = p.roles.get(row).map(|r| r.id) else { return Ok(()) };
                let Some(i) = self.variant_of_column(p, c) else { return Ok(()) };
                let person = if v.trim().is_empty() {
                    None
                } else {
                    Some(
                        p.person_named(v)
                            .ok_or_else(|| format!("no cast member named \"{}\"", v.trim()))?
                            .id,
                    )
                };
                let cast = &mut p.mic.variants[i].cast;
                cast.retain(|x| x.role != role);
                if let Some(person) = person {
                    cast.push(VariantCast { role, person });
                }
            }
        }
        Ok(())
    }

    fn color(&self, p: &Project, row: usize) -> String {
        match self.sheet {
            MicSheet::Performers => p.cast.get(row).map(|x| x.color.clone()).unwrap_or_default(),
            MicSheet::Tracks | MicSheet::Variants => {
                p.roles.get(row).map(|x| x.color.clone()).unwrap_or_default()
            }
            MicSheet::Frequencies => freq_rows(p)
                .get(row)
                .and_then(|id| p.transmitters.iter().find(|t| t.id == *id))
                .map(|t| t.color.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn mark(&self, p: &Project, row: usize, c: usize) -> Option<CellMark> {
        match self.sheet {
            MicSheet::Plot => {
                if c == 1 {
                    let probs = self.problems_at(Some(row), None);
                    return (!probs.is_empty()).then(|| CellMark {
                        warn: true,
                        hint: probs.iter().map(|x| x.message(p)).collect::<Vec<_>>().join("\n"),
                        ..CellMark::default()
                    });
                }
                let ci = c.checked_sub(2)?;
                let tx = *self.view.txs.get(ci)?;
                let wears = self.view.cells.get(row)?.get(ci)?;
                let mut hint = Vec::new();
                let mut color = String::new();
                for w in wears {
                    let person = w.person.and_then(|i| p.person(i));
                    if color.is_empty() {
                        color = person
                            .map(|x| x.color.clone())
                            .filter(|c| !c.is_empty())
                            .or_else(|| p.role(w.role).map(|r| r.color.clone()))
                            .unwrap_or_default();
                    }
                    hint.push(format!(
                        "{}{}: {}",
                        p.role(w.role).map(|r| r.name.as_str()).unwrap_or("?"),
                        if w.b { " (B mic)" } else { "" },
                        person.map(|x| x.name.as_str()).unwrap_or("nobody cast")
                    ));
                }
                if let Some(s) = self.view.swap_at(row, tx) {
                    let from = s.from.person.and_then(|i| p.person(i)).map(|x| x.name.clone()).unwrap_or_default();
                    hint.push(format!(
                        "Swap from {from} after scene {}{}",
                        scene_no(p, s.after),
                        s.window.map(|w| format!(", {} to do it", format_duration(w))).unwrap_or_default()
                    ));
                }
                let probs = self.problems_at(Some(row), Some(tx));
                let warn = !wears.is_empty() && !probs.is_empty();
                if warn {
                    hint.extend(probs.iter().map(|x| x.message(p)));
                }
                if wears.is_empty() && hint.is_empty() {
                    let w = self.view.off.get(row)?.get(ci)?.as_ref()?;
                    let person = w.person.and_then(|i| p.person(i));
                    return Some(CellMark {
                        color: wear_color(p, w),
                        ghost: wears_text(p, std::slice::from_ref(w)),
                        hint: format!(
                            "Off stage: {} keeps the transmitter on (channel down)",
                            person.map(|x| x.name.clone()).or_else(|| p.role(w.role).map(|r| r.name.clone())).unwrap_or_default()
                        ),
                        ..CellMark::default()
                    });
                }
                Some(CellMark { color, warn, hint: hint.join("\n"), ..CellMark::default() })
            }
            MicSheet::Dca => {
                let scene = p.scenes.get(row)?;
                let d = p.mic.dca_cell(scene.id, c.checked_sub(1)? as u32)?;
                let ch = dca_channels(p, scene.id, d);
                Some(CellMark {
                    hint: if ch.is_empty() { "No channels".into() } else { format!("Channels {}", ch.join(", ")) },
                    ..CellMark::default()
                })
            }
            MicSheet::Frequencies => {
                let rows = freq_rows(p);
                let tx = *rows.get(row)?;
                let probs = self.problems_at(None, Some(tx));
                let warn = probs.iter().any(|x| !matches!(x, Problem::NotActive { .. }));
                warn.then(|| CellMark { warn, ..CellMark::default() })
            }
            MicSheet::Swaps => {
                let s = self.view.swaps.get(row)?;
                let short = s.window.is_some_and(|w| w < p.mic.min_swap);
                (short && c == 4).then(|| CellMark {
                    warn: true,
                    hint: format!("Less than {}", format_duration(p.mic.min_swap)),
                    ..CellMark::default()
                })
            }
            MicSheet::Tracks => {
                let role = p.roles.get(row)?;
                let t = p.mic.track(role.id)?;
                let bad = [t.tx, t.tx_b].into_iter().flatten().find(|tx| {
                    p.transmitters.iter().any(|x| x.id == *tx && x.status != TxStatus::Active)
                })?;
                (c == 2 || c == 3).then(|| CellMark {
                    warn: true,
                    hint: format!("Transmitter {} is not active", tx_label(p, bad)),
                    ..CellMark::default()
                })
            }
            _ => None,
        }
    }
}

/// Parse a roles list for the scenes table (re-exported for the app).
pub fn roles_from_text(p: &Project, v: &str) -> Result<Vec<Id>, String> {
    parse_roles(p, v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::paste;
    use crate::model::{CastKind, Casting, Person, Role, Scene, Transmitter};

    fn show() -> Project {
        let mut p = Project::default();
        for n in ["Anna", "Ben"] {
            let id = p.alloc_id();
            p.cast.push(Person { id, name: n.into(), ..Person::default() });
        }
        for (n, who) in [("Mara", 0), ("Tobias", 1)] {
            let id = p.alloc_id();
            let person = p.cast[who].id;
            p.roles.push(Role { id, name: n.into(), casting: vec![Casting { person, kind: CastKind::Lead }], ..Role::default() });
        }
        for ch in ["1", "2"] {
            let id = p.alloc_id();
            p.transmitters.push(Transmitter { id, channel: ch.into(), ..Transmitter::default() });
        }
        for n in ["1", "2"] {
            let id = p.alloc_id();
            p.scenes.push(Scene { id, number: n.into(), ..Scene::default() });
        }
        p
    }

    #[test]
    fn off_stage_between_scenes_keeps_the_mic() {
        let p = crate::demo::musical();
        let v = MicView::build(&p);
        let g = MicGrid { sheet: MicSheet::Plot, view: &v };
        // Mara (TX 1) is on in 1.1, off in 1.2, on again in 1.3.
        let mara = p.roles[0].id;
        assert_eq!(g.text(&p, 1, 2), "");
        assert_eq!(v.off[1][0].map(|w| w.role), Some(mara));
        let m = g.mark(&p, 1, 2).unwrap();
        assert_eq!(m.ghost, "Mara");
        assert!(m.hint.contains("Off stage"));
        // On stage: no ghost; before the first scene of a transmitter: nothing.
        assert!(g.mark(&p, 0, 2).unwrap().ghost.is_empty());
        assert!(v.off[0].iter().all(Option::is_none));
        // Between two different wearers (a swap) nothing is marked.
        for s in &v.swaps {
            let ci = v.column_of(s.tx).unwrap();
            for row in s.after + 1..s.before {
                assert!(v.off[row][ci].is_none());
            }
        }
    }

    #[test]
    fn typing_into_the_plot_sets_exceptions_and_who_is_on() {
        let mut p = show();
        let (el, fi) = (p.roles[0].id, p.roles[1].id);
        Grid::set(&MicGrid { sheet: MicSheet::Tracks, view: &MicView::build(&p) }, &mut p, 0, 2, "1").unwrap();
        Grid::set(&MicGrid { sheet: MicSheet::Tracks, view: &MicView::build(&p) }, &mut p, 1, 2, "2").unwrap();
        let v = MicView::build(&p);
        let g = MicGrid { sheet: MicSheet::Plot, view: &v };
        // Mara in scene 1 on her usual TX 1: no exception, she is now on stage.
        g.set(&mut p, 0, 2, "Mara").unwrap();
        assert_eq!(p.scenes[0].roles, vec![el]);
        assert!(p.mic.overrides.is_empty());
        // By performer name, on TX 2 (an exception).
        let v = MicView::build(&p);
        let g = MicGrid { sheet: MicSheet::Plot, view: &v };
        g.set(&mut p, 1, 3, "anna").unwrap();
        assert_eq!(p.mic.tx_in(p.scenes[1].id, el), Some(p.transmitters[1].id));
        assert!(g.set(&mut p, 1, 3, "Nobody").is_err());
        let v = MicView::build(&p);
        assert_eq!(MicGrid { sheet: MicSheet::Plot, view: &v }.text(&p, 1, 3), "Mara");
        // Clearing takes the mic away (she stays on stage, which is a problem).
        MicGrid { sheet: MicSheet::Plot, view: &v }.set(&mut p, 1, 3, "").unwrap();
        let v = MicView::build(&p);
        assert!(v.problems.iter().any(|x| matches!(x, Problem::NoMic { role, .. } if *role == el)));
        let _ = fi;
    }

    #[test]
    fn pasting_a_dca_plan() {
        let mut p = show();
        let v = MicView::build(&p);
        let g = MicGrid { sheet: MicSheet::Dca, view: &v };
        let (rows, errors) = paste(&g, &mut p, 0, 2, "Leads: Mara, Tobias\tTobias\nMara\n\nextra");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(rows, vec![0, 1], "no rows beyond the scenes");
        assert_eq!(p.mic.dca.len(), 3);
        assert_eq!(g.text(&p, 0, 2), "Leads: Mara, Tobias");
    }

    #[test]
    fn capsules_by_label_or_number() {
        let mut p = show();
        let id = p.alloc_id();
        p.capsules.push(crate::model::Capsule { id, model: "MKE2".into(), color: "beige".into(), ..Default::default() });
        assert_eq!(capsule_label(&p, id), "C1 MKE2 beige");
        assert_eq!(parse_capsule(&p, "c1"), Ok(Some(id)));
        assert_eq!(parse_capsule(&p, "C1 MKE2 beige"), Ok(Some(id)));
        assert!(parse_capsule(&p, "C9").is_err());
    }

    #[test]
    fn battery_defaults_to_before_the_show() {
        let mut p = show();
        let v = MicView::build(&p);
        let g = MicGrid { sheet: MicSheet::Battery, view: &v };
        assert_eq!(g.text(&p, 0, 3), "yes");
        g.set(&mut p, 0, 4, "x").unwrap();
        g.set(&mut p, 0, 5, "2").unwrap();
        assert_eq!(g.text(&p, 0, 4), "yes");
        assert_eq!(g.text(&p, 0, 5), "2");
        assert!(g.set(&mut p, 0, 5, "99").is_err());
    }
}
