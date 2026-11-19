//! The plans as data for the templates (one JSON object per print).

use crate::{Doc, Options};
use bp_core::mic::{
    cell_key, dca_channels, performer, tx_label, wears_text, MicView, Problem,
};
use bp_core::mic_grid::capsule_label;
use bp_core::model::{Id, Project, TxStatus};
use bp_core::time::{format_duration, format_number, today};
use serde_json::{json, Value};

/// What a template gets (`sys.inputs.data`).
pub struct Data(pub Value);

impl Data {
    pub fn get(&self) -> &Value {
        &self.0
    }
}

fn scene_no(p: &Project, i: usize) -> String {
    p.scenes.get(i).map(|s| s.number.clone()).unwrap_or_default()
}

fn person(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.person(i)).map(|x| x.name.clone()).unwrap_or_default()
}

fn role(p: &Project, id: Id) -> String {
    p.role(id).map(|r| r.name.clone()).unwrap_or_default()
}

/// Transmitters per block of the mic plot (scene columns repeat per block).
fn plot_columns(opts: &Options) -> usize {
    let (w, h) = opts.paper.size_cm();
    let width = if opts.landscape { h } else { w };
    let per_cm = match opts.theme {
        crate::Theme::Stage => 0.42,
        _ => 0.62,
    };
    (((width - 2.6) * per_cm) as usize).saturating_sub(3).max(4)
}

pub fn build(p: &Project, doc: Doc, opts: &Options) -> Data {
    if let Doc::Setlist(id) = doc {
        return Data(setlist(p, id, opts));
    }
    let view = MicView::build(p);
    let plan = &p.mic;
    let rev = if plan.revision.number > 0 {
        format!("Rev {} · {}", plan.revision.number, plan.revision.date)
    } else {
        format!("Draft · printed {}", today())
    };
    let cast = match plan.variant() {
        Some(v) => format!("Cast: {}", v.name),
        None => String::new(),
    };
    let mut d = json!({
        "title": doc.title(),
        "subtitle": cast,
        "revision": rev,
        "paper": opts.paper_name(),
        "landscape": opts.landscape,
        "theme": opts.theme_name(),
        "legend": "",
        "meta": {
            "production": p.meta.production,
            "company": p.meta.company,
            "venue": p.meta.venue,
            "dates": p.meta.dates,
        },
    });
    let body = match doc {
        Doc::MicPlot => mic_plot(p, &view, opts, &mut d),
        Doc::MicCards => mic_cards(p, &view, opts),
        Doc::SwapList => swap_list(p, &view),
        Doc::Frequencies => frequencies(p, &view),
        Doc::DcaSheet => dca_sheet(p),
        Doc::BatteryPlan => battery(p, &view),
        Doc::Setlist(_) => Value::Null,
    };
    if let (Some(o), Value::Object(b)) = (d.as_object_mut(), body) {
        o.extend(b);
    }
    Data(d)
}

fn mic_plot(p: &Project, view: &MicView, opts: &Options, d: &mut Value) -> Value {
    let changed = if opts.mark_changes {
        p.mic.revision.changed(&view.snapshot(p))
    } else {
        Vec::new()
    };
    if !changed.is_empty() {
        d["legend"] = json!(format!(
            "Bold frame: changed since Rev {}. → a transmitter changes hands before this scene.",
            p.mic.revision.number.saturating_sub(1)
        ));
    } else {
        d["legend"] = json!("→ a transmitter changes hands before this scene.");
    }
    // As many blocks as needed, the transmitters spread evenly over them.
    let max = plot_columns(opts);
    let n = view.txs.len();
    let per = if n == 0 { 1 } else { n.div_ceil(n.div_ceil(max)) };
    let chunks: Vec<&[Id]> = if view.txs.is_empty() { vec![&[][..]] } else { view.txs.chunks(per).collect() };
    let mut start = 0;
    let blocks: Vec<Value> = chunks
        .iter()
        .map(|txs| {
            let first = start;
            start += txs.len();
            let columns: Vec<String> = txs.iter().map(|t| tx_label(p, *t)).collect();
            let rows: Vec<Value> = p
                .scenes
                .iter()
                .enumerate()
                .map(|(si, s)| {
                    let warn: Vec<String> = view
                        .problems
                        .iter()
                        .filter(|x| matches!(x, Problem::NoMic { scene, .. } if *scene == si))
                        .map(|x| match x {
                            Problem::NoMic { role: r, .. } => format!("no mic: {}", role(p, *r)),
                            _ => String::new(),
                        })
                        .collect();
                    let cells: Vec<Value> = txs
                        .iter()
                        .enumerate()
                        .map(|(k, tx)| {
                            let w = &view.cells[si][first + k];
                            let who: Vec<String> = w
                                .iter()
                                .filter_map(|x| x.person.and_then(|i| p.person(i)))
                                .map(|x| if x.short.is_empty() { x.name.clone() } else { x.short.clone() })
                                .collect();
                            let color = w
                                .first()
                                .and_then(|x| x.person.and_then(|i| p.person(i)).map(|x| x.color.clone()))
                                .filter(|c| !c.is_empty())
                                .or_else(|| w.first().and_then(|x| p.role(x.role)).map(|r| r.color.clone()))
                                .unwrap_or_default();
                            json!({
                                "text": wears_text(p, w),
                                "who": who.join(", "),
                                "color": color,
                                "swap": view.swap_at(si, *tx).is_some(),
                                "changed": changed.contains(&cell_key(s.id, *tx)),
                            })
                        })
                        .collect();
                    json!({ "no": s.number, "title": s.title, "warn": warn.join(", "), "cells": cells })
                })
                .collect();
            json!({ "columns": columns, "rows": rows })
        })
        .collect();
    json!({ "blocks": blocks })
}

fn mic_cards(p: &Project, view: &MicView, opts: &Options) -> Value {
    let cards: Vec<Value> = p
        .cast
        .iter()
        .filter(|x| p.roles.iter().any(|r| performer(p, r.id) == Some(x.id)) || !view.person_txs(x.id).is_empty())
        .map(|x| {
            let f = p.mic.fitting(x.id);
            let roles: Vec<String> = p
                .roles
                .iter()
                .filter(|r| performer(p, r.id) == Some(x.id))
                .map(|r| r.name.clone())
                .collect();
            let txs: Vec<Value> = view
                .person_txs(x.id)
                .iter()
                .map(|(tx, scenes, b)| {
                    let cap = if *b { f.and_then(|f| f.capsule_b) } else { f.and_then(|f| f.capsule) };
                    json!({
                        "tx": format!("{}{}", tx_label(p, *tx), if *b { " B" } else { "" }),
                        "capsule": cap.map(|c| capsule_label(p, c)).unwrap_or_default(),
                        "scenes": scene_ranges(p, scenes),
                    })
                })
                .collect();
            let swaps: Vec<String> = view
                .swaps
                .iter()
                .filter_map(|s| {
                    if s.to.person == Some(x.id) {
                        Some(format!(
                            "Before {}: gets TX {} from {}",
                            scene_no(p, s.before),
                            tx_label(p, s.tx),
                            person(p, s.from.person)
                        ))
                    } else if s.from.person == Some(x.id) {
                        Some(format!(
                            "After {}: hands TX {} to {}",
                            scene_no(p, s.after),
                            tx_label(p, s.tx),
                            person(p, s.to.person)
                        ))
                    } else {
                        None
                    }
                })
                .collect();
            let notes: Vec<&str> = [x.notes.as_str(), f.map(|f| f.notes.as_str()).unwrap_or("")]
                .into_iter()
                .filter(|n| !n.trim().is_empty())
                .collect();
            json!({
                "name": x.name,
                "short": x.short,
                "roles": roles.join(", "),
                "txs": txs,
                "position": f.map(|f| f.position.clone()).unwrap_or_default(),
                "swaps": swaps,
                "notes": notes.join(" · "),
            })
        })
        .collect();
    let (w, h) = opts.paper.size_cm();
    let (w, h) = if opts.landscape { (h, w) } else { (w, h) };
    let columns = if w > 35.0 { 3 } else { 2 };
    let rows = ((h - 3.9) / 6.2).floor().max(1.0);
    json!({
        "cards": cards,
        "card_columns": columns,
        "card_height": ((h - 3.9) / rows * 100.0).floor() / 100.0 - 0.05,
    })
}

/// "1.1–1.4, 2.2" from scene indices.
fn scene_ranges(p: &Project, scenes: &[usize]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < scenes.len() {
        let mut j = i;
        while j + 1 < scenes.len() && scenes[j + 1] == scenes[j] + 1 {
            j += 1;
        }
        if j > i {
            parts.push(format!("{}–{}", scene_no(p, scenes[i]), scene_no(p, scenes[j])));
        } else {
            parts.push(scene_no(p, scenes[i]));
        }
        i = j + 1;
    }
    parts.join(", ")
}

fn section(title: &str, rows: Vec<Value>) -> Value {
    json!({ "title": title, "rows": rows })
}

fn row(cells: Vec<String>, warn: bool) -> Value {
    json!({ "cells": cells, "warn": warn })
}

fn swap_list(p: &Project, view: &MicView) -> Value {
    let rows: Vec<Value> = view
        .swaps
        .iter()
        .map(|s| {
            let who = |w: &bp_core::mic::Wear| {
                let n = person(p, w.person);
                if n.is_empty() { role(p, w.role) } else { format!("{n} ({})", role(p, w.role)) }
            };
            let note = p.scenes.get(s.before).and_then(|sc| p.mic.swap_note(s.tx, sc.id));
            let short = s.window.is_some_and(|w| w < p.mic.min_swap);
            row(
                vec![
                    format!("{} → {}", scene_no(p, s.after), scene_no(p, s.before)),
                    tx_label(p, s.tx),
                    who(&s.from),
                    who(&s.to),
                    s.window.map(format_duration).unwrap_or_default(),
                    note.map(|n| n.location.clone()).unwrap_or_default(),
                    note.map(|n| n.by.clone()).unwrap_or_default(),
                    note.map(|n| n.notes.clone()).unwrap_or_default(),
                ],
                short,
            )
        })
        .collect();
    json!({
        "head": ["Scenes", "TX", "From", "To", "Time", "Where", "By", "Notes"],
        "widths": [0, 0, 2, 2, 0, 1.5, 1, 2],
        "sections": [section("", rows)],
        "legend": format!("Red: less than {} for the swap.", format_duration(p.mic.min_swap)),
    })
}

fn frequencies(p: &Project, view: &MicView) -> Value {
    let mut bands: Vec<String> = p.transmitters.iter().map(|t| t.band.clone()).collect();
    bands.sort();
    bands.dedup();
    let sections: Vec<Value> = bands
        .iter()
        .map(|band| {
            let mut txs: Vec<&bp_core::model::Transmitter> = p.transmitters.iter().filter(|t| &t.band == band).collect();
            txs.sort_by(|a, b| a.frequency.unwrap_or(f64::MAX).total_cmp(&b.frequency.unwrap_or(f64::MAX)));
            let rows = txs
                .iter()
                .map(|t| {
                    let probs: Vec<String> = view
                        .problems
                        .iter()
                        .filter(|x| match x {
                            Problem::DuplicateFrequency { a, b, .. } => *a == t.id || *b == t.id,
                            Problem::Intermod { a, b, victim, .. } => *a == t.id || *b == t.id || *victim == t.id,
                            _ => false,
                        })
                        .map(|x| x.message(p))
                        .collect();
                    row(
                        vec![
                            tx_label(p, t.id),
                            t.name.clone(),
                            t.model.clone(),
                            t.frequency.map(format_number).unwrap_or_default(),
                            t.status.label().to_string(),
                            probs.join("; "),
                        ],
                        !probs.is_empty() || t.status == TxStatus::Defect,
                    )
                })
                .collect();
            section(if band.is_empty() { "No band" } else { band }, rows)
        })
        .collect();
    json!({
        "head": ["TX", "Name", "Model", "MHz", "Status", "Check"],
        "widths": [0, 1.5, 1.5, 0, 0, 3],
        "sections": sections,
    })
}

fn dca_sheet(p: &Project) -> Value {
    let n = p.mic.dca_count.max(1);
    let mut head = vec!["No.".to_string(), "Scene".to_string()];
    head.extend((1..=n).map(|i| format!("DCA {i}")));
    let rows: Vec<Value> = p
        .scenes
        .iter()
        .map(|s| {
            let mut cells = vec![s.number.clone(), s.title.clone()];
            for i in 1..=n {
                cells.push(match p.mic.dca_cell(s.id, i) {
                    Some(d) => {
                        let ch = dca_channels(p, s.id, d);
                        let mut t = if d.label.is_empty() {
                            bp_core::sheet::roles_text(p, &d.roles)
                        } else {
                            d.label.clone()
                        };
                        if !ch.is_empty() {
                            t.push_str(&format!(" ({})", ch.join(", ")));
                        }
                        t
                    }
                    None => String::new(),
                });
            }
            row(cells, false)
        })
        .collect();
    let mut widths = vec![json!(0), json!(1.6)];
    widths.extend((1..=n).map(|_| json!(1)));
    json!({ "head": head, "widths": widths, "sections": [section("", rows)], "legend": "Label (desk channels)." })
}

fn battery(p: &Project, view: &MicView) -> Value {
    let rows: Vec<Value> = view
        .txs
        .iter()
        .filter_map(|tx| p.transmitters.iter().find(|t| t.id == *tx))
        .map(|t| {
            let r = p.mic.battery_rule(t.id);
            let yes = |b: bool| if b { "yes".to_string() } else { String::new() };
            let after = r
                .and_then(|r| r.after_scene)
                .and_then(|s| p.scenes.iter().find(|x| x.id == s))
                .map(|s| s.number.clone())
                .unwrap_or_default();
            row(
                vec![
                    tx_label(p, t.id),
                    t.name.clone(),
                    t.battery.clone(),
                    yes(r.is_none_or(|r| r.before_show)),
                    yes(r.is_some_and(|r| r.interval)),
                    after,
                    r.map(|r| r.notes.clone()).unwrap_or_default(),
                ],
                false,
            )
        })
        .collect();
    json!({
        "head": ["TX", "Name", "Battery", "Before show", "Interval", "After scene", "Notes"],
        "widths": [0, 1.5, 1, 0, 0, 0, 3],
        "sections": [section("", rows)],
    })
}

fn setlist(p: &Project, id: Id, opts: &Options) -> Value {
    use bp_core::setlist::{entry_length, entry_title, timing, EntryKind, Setlist};
    use bp_core::time::format_clock;
    let empty = Setlist::default();
    let s = p.setlist(id).unwrap_or(&empty);
    let t = timing(p, s);
    let pick = |a: &str, b: &str| if a.trim().is_empty() { b.to_string() } else { a.to_string() };
    let entries: Vec<Value> = s
        .entries
        .iter()
        .zip(&t.entries)
        .enumerate()
        .map(|(i, (e, et))| {
            let song = e.song.and_then(|id| p.song(id)).filter(|_| e.kind == EntryKind::Song);
            let f = |g: fn(&bp_core::setlist::Song) -> String| song.map(g).unwrap_or_default();
            // The sign goes on an entry only when a song follows.
            let next_song = s.entries.get(i + 1).is_some_and(|x| x.kind == EntryKind::Song);
            json!({
                "kind": format!("{:?}", e.kind).to_ascii_lowercase(),
                "kind_label": e.kind.label(),
                "no": et.number.map(|n| n.to_string()).unwrap_or_default(),
                "title": entry_title(p, e),
                "key": f(|x| x.key.clone()),
                "bpm": song.and_then(|x| x.bpm).map(format_number).unwrap_or_default(),
                "length": entry_length(p, e).map(format_duration).unwrap_or_default(),
                "at": format_duration(et.at),
                "clock": et.clock.map(format_clock).unwrap_or_default(),
                "lead": f(|x| x.lead.clone()),
                "lineup": f(|x| x.lineup.clone()),
                "light": f(|x| x.light_cue.clone()),
                "sound": f(|x| x.sound_cue.clone()),
                "notes": if e.notes.trim().is_empty() { f(|x| x.notes.clone()) } else { e.notes.clone() },
                "color": f(|x| x.color.clone()),
                "next": if next_song { e.transition.sign() } else { "" },
                "next_label": if next_song { e.transition.label() } else { "" },
                "minutes": entry_length(p, e).filter(|l| *l > 0.0).map(minutes).unwrap_or_default(),
            })
        })
        .collect();
    // FOH columns: the fixed ones and those with something in them.
    let has = |k: &str| entries.iter().any(|e| e[k].as_str().is_some_and(|v| !v.is_empty()) && e["kind"] == "song");
    // `wide`: share of the free width (0 = as wide as the text).
    let mut columns = vec![json!({"key": "no", "title": "#", "wide": 0, "mono": true})];
    if s.start.is_some() {
        columns.push(json!({"key": "clock", "title": "Clock", "wide": 0, "mono": true}));
    }
    columns.push(json!({"key": "at", "title": "Starts", "wide": 0, "mono": true}));
    columns.push(json!({"key": "title", "title": "Title", "wide": 2.2, "mono": false}));
    for (k, title, wide, mono) in [
        ("key", "Key", 0.0, false),
        ("bpm", "BPM", 0.0, true),
        ("length", "Length", 0.0, true),
        ("lead", "Lead", 0.0, false),
        ("lineup", "Line-up", 1.3, false),
        ("light", "Light", 0.0, false),
        ("sound", "Sound", 0.0, false),
        ("notes", "Notes", 1.6, false),
    ] {
        if k == "length" || has(k) {
            columns.push(json!({"key": k, "title": title, "wide": wide, "mono": mono}));
        }
    }
    let parts: Vec<Value> = t
        .parts
        .iter()
        .map(|x| json!({"name": x.name, "length": format_duration(x.length), "songs": x.songs}))
        .collect();
    json!({
        "title": pick(&s.name, "Setlist"),
        "band": pick(&s.band, &p.meta.production),
        "venue": pick(&s.venue, &p.meta.venue),
        "date": s.date,
        "start": s.start.map(format_clock).unwrap_or_default(),
        "paper": opts.paper_name(),
        "landscape": opts.landscape,
        "theme": opts.theme_name(),
        "summary": t.summary(),
        "printed": format!("Printed {}", today()),
        "entries": entries,
        "columns": columns,
        "parts": parts,
    })
}

/// "20 min", "1 h 05 min", "45 s".
fn minutes(secs: f64) -> String {
    let m = (secs / 60.0).round() as u64;
    if secs < 60.0 {
        format!("{} s", secs.round() as u64)
    } else if m >= 60 {
        format!("{} h {:02} min", m / 60, m % 60)
    } else {
        format!("{m} min")
    }
}
