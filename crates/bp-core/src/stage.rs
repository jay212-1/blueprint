//! The stage plot: the stage from above (audience at the bottom), with items
//! from a symbol library placed in metres. Every symbol is drawn from simple
//! shapes worked out here ([`shapes`]), so the app and the print look the
//! same. Stands and DI boxes can be linked to patch inputs and show their
//! channel numbers.

use crate::model::{Id, Project};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct StagePlot {
    /// Metres, left to right as the audience sees it.
    pub width: f64,
    /// Metres, back to front.
    pub depth: f64,
    /// Grid step in metres (also the snap).
    pub grid: f64,
    pub show_grid: bool,
    /// Written below the front edge.
    pub front: String,
    pub items: Vec<StageItem>,
}

impl Default for StagePlot {
    fn default() -> Self {
        Self { width: 10.0, depth: 7.0, grid: 0.5, show_grid: true, front: "Audience".into(), items: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Symbol {
    #[default]
    MicStand,
    MicSmall,
    Wedge,
    Drums,
    Keys,
    GuitarAmp,
    BassAmp,
    DiBox,
    Power,
    Riser,
    Person,
    Iem,
    Speaker,
    Stagebox,
    Chair,
    Label,
}

impl Symbol {
    pub const ALL: [Symbol; 16] = [
        Symbol::MicStand,
        Symbol::MicSmall,
        Symbol::Wedge,
        Symbol::Drums,
        Symbol::Keys,
        Symbol::GuitarAmp,
        Symbol::BassAmp,
        Symbol::DiBox,
        Symbol::Power,
        Symbol::Riser,
        Symbol::Person,
        Symbol::Iem,
        Symbol::Speaker,
        Symbol::Stagebox,
        Symbol::Chair,
        Symbol::Label,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Symbol::MicStand => "Mic stand",
            Symbol::MicSmall => "Small mic stand",
            Symbol::Wedge => "Wedge",
            Symbol::Drums => "Drum kit",
            Symbol::Keys => "Keyboard",
            Symbol::GuitarAmp => "Guitar amp",
            Symbol::BassAmp => "Bass amp",
            Symbol::DiBox => "DI box",
            Symbol::Power => "Power",
            Symbol::Riser => "Riser",
            Symbol::Person => "Person",
            Symbol::Iem => "IEM",
            Symbol::Speaker => "Speaker",
            Symbol::Stagebox => "Stagebox",
            Symbol::Chair => "Chair",
            Symbol::Label => "Text",
        }
    }

    pub fn parse(v: &str) -> Option<Symbol> {
        let v = v.trim().to_ascii_lowercase();
        match v.as_str() {
            "mic" | "stand" | "boom" => return Some(Symbol::MicStand),
            "monitor" => return Some(Symbol::Wedge),
            "drum" | "drums" | "kit" => return Some(Symbol::Drums),
            "di" => return Some(Symbol::DiBox),
            "power" | "230v" | "socket" => return Some(Symbol::Power),
            "text" | "label" => return Some(Symbol::Label),
            _ => {}
        }
        Symbol::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase();
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }

    /// Width and depth in metres when placed.
    pub fn size(self) -> (f64, f64) {
        match self {
            Symbol::MicStand => (0.5, 0.5),
            Symbol::MicSmall => (0.35, 0.35),
            Symbol::Wedge => (0.6, 0.45),
            Symbol::Drums => (2.0, 1.8),
            Symbol::Keys => (1.4, 0.5),
            Symbol::GuitarAmp => (0.7, 0.35),
            Symbol::BassAmp => (0.7, 0.5),
            Symbol::DiBox => (0.25, 0.2),
            Symbol::Power => (0.3, 0.2),
            Symbol::Riser => (2.0, 2.0),
            Symbol::Person => (0.5, 0.5),
            Symbol::Iem => (0.3, 0.2),
            Symbol::Speaker => (0.6, 0.5),
            Symbol::Stagebox => (0.6, 0.4),
            Symbol::Chair => (0.45, 0.45),
            Symbol::Label => (1.5, 0.4),
        }
    }

    /// Symbols that can carry input channels.
    pub fn takes_inputs(self) -> bool {
        !matches!(self, Symbol::Riser | Symbol::Power | Symbol::Label | Symbol::Chair | Symbol::Person)
    }

    /// Drawn below the others.
    pub fn layer(self) -> u8 {
        match self {
            Symbol::Riser => 0,
            Symbol::Drums | Symbol::Keys => 1,
            _ => 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct StageItem {
    pub id: Id,
    pub symbol: Symbol,
    /// Centre, metres from the back left corner.
    pub x: f64,
    pub y: f64,
    /// Degrees, clockwise.
    pub rot: f64,
    pub w: f64,
    pub d: f64,
    pub label: String,
    /// Patch inputs (their channel numbers are shown).
    pub inputs: Vec<Id>,
    pub color: String,
}

impl StageItem {
    pub fn new(id: Id, symbol: Symbol, x: f64, y: f64) -> StageItem {
        let (w, d) = symbol.size();
        StageItem { id, symbol, x, y, w, d, ..StageItem::default() }
    }

    /// Is the point (metres) on the item?
    pub fn hit(&self, x: f64, y: f64) -> bool {
        let (s, c) = (-self.rot.to_radians()).sin_cos();
        let (dx, dy) = (x - self.x, y - self.y);
        let (lx, ly) = (dx * c - dy * s, dx * s + dy * c);
        let pad = 0.08;
        lx.abs() <= self.w / 2.0 + pad && ly.abs() <= self.d / 2.0 + pad
    }
}

/// How a shape is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fill {
    None,
    Light,
    Mid,
    Dark,
    Accent,
}

/// A piece of a drawn symbol, in metres on the stage.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "shape", rename_all = "lowercase")]
pub enum Shape {
    Poly { points: Vec<(f64, f64)>, fill: Fill, tint: String },
    Circle { x: f64, y: f64, r: f64, fill: Fill, tint: String },
    Line { points: Vec<(f64, f64)> },
    Text { x: f64, y: f64, text: String, size: f64, bold: bool, light: bool },
}

/// The channel numbers of an item's inputs ("3, 4").
pub fn channels(p: &Project, item: &StageItem) -> String {
    let mut nos: Vec<u32> = item
        .inputs
        .iter()
        .filter_map(|i| p.patch.inputs.iter().find(|x| x.id == *i).map(|x| x.no))
        .collect();
    nos.sort_unstable();
    nos.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

/// Parse "3, 4" (channel numbers) to input ids.
pub fn parse_channels(p: &Project, v: &str) -> Result<Vec<Id>, String> {
    let find = |n: u32| p.patch.inputs.iter().find(|i| i.no == n).map(|i| i.id).ok_or(format!("there is no input {n} in the patch"));
    let mut out = Vec::new();
    for part in v.split([',', ';', ' ', '+']).map(str::trim).filter(|s| !s.is_empty()) {
        let no = |t: &str| t.trim().parse::<u32>().map_err(|_| format!("\"{part}\" is not a channel number"));
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (no(a)?, no(b)?),
            None => (no(part)?, no(part)?),
        };
        if b < a || b - a > 128 {
            return Err(format!("\"{part}\" is not a range of channels"));
        }
        for n in a..=b {
            let id = find(n)?;
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    Ok(out)
}

/// The channel numbers for the badge on the plot: runs of three or more
/// as a range ("1-7, 9").
pub fn badge(p: &Project, item: &StageItem) -> String {
    let mut nos: Vec<u32> = item
        .inputs
        .iter()
        .filter_map(|i| p.patch.inputs.iter().find(|x| x.id == *i).map(|x| x.no))
        .collect();
    nos.sort_unstable();
    nos.dedup();
    let mut parts = Vec::new();
    let mut i = 0;
    while i < nos.len() {
        let mut j = i;
        while j + 1 < nos.len() && nos[j + 1] == nos[j] + 1 {
            j += 1;
        }
        if j >= i + 2 {
            parts.push(format!("{}-{}", nos[i], nos[j]));
        } else {
            parts.extend(nos[i..=j].iter().map(|n| n.to_string()));
        }
        i = j + 1;
    }
    parts.join(", ")
}

/// The text an item shows: its label, else the names of its inputs, else
/// the symbol.
pub fn item_text(p: &Project, item: &StageItem) -> String {
    if !item.label.trim().is_empty() {
        return item.label.clone();
    }
    let names: Vec<String> = item
        .inputs
        .iter()
        .filter_map(|i| p.patch.inputs.iter().find(|x| x.id == *i).map(|x| x.name.clone()))
        .collect();
    if !names.is_empty() {
        return names.join(" / ");
    }
    match item.symbol {
        // These show what they are already.
        Symbol::Riser | Symbol::Label | Symbol::DiBox | Symbol::Iem | Symbol::Stagebox | Symbol::Power | Symbol::Person => String::new(),
        s => s.label().to_string(),
    }
}

/// The shapes of an item, on the stage (rotated and placed), in metres.
pub fn shapes(p: &Project, item: &StageItem) -> Vec<Shape> {
    let (hw, hd) = (item.w / 2.0, item.d / 2.0);
    let tint = item.color.clone();
    let (s, c) = item.rot.to_radians().sin_cos();
    let at = |x: f64, y: f64| (item.x + x * c - y * s, item.y + x * s + y * c);
    let poly = |pts: &[(f64, f64)], fill: Fill| Shape::Poly { points: pts.iter().map(|(x, y)| at(*x, *y)).collect(), fill, tint: tint.clone() };
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64, fill: Fill| poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], fill);
    let circle = |x: f64, y: f64, r: f64, fill: Fill| {
        let (cx, cy) = at(x, y);
        Shape::Circle { x: cx, y: cy, r, fill, tint: tint.clone() }
    };
    let line = |pts: &[(f64, f64)]| Shape::Line { points: pts.iter().map(|(x, y)| at(*x, *y)).collect() };
    let mut out = Vec::new();
    match item.symbol {
        Symbol::MicStand | Symbol::MicSmall => {
            let r = hw.min(hd);
            out.push(circle(0.0, 0.0, r, Fill::None));
            out.push(line(&[(0.0, 0.0), (0.0, -r * 1.5)]));
            out.push(circle(0.0, -r * 1.5, r * 0.22, Fill::Dark));
            out.push(circle(0.0, 0.0, r * 0.18, Fill::Dark));
        }
        Symbol::Wedge => {
            out.push(poly(&[(-hw, -hd), (hw, -hd), (hw * 0.75, hd), (-hw * 0.75, hd)], Fill::Mid));
            out.push(line(&[(-hw * 0.6, -hd * 0.4), (hw * 0.6, -hd * 0.4)]));
        }
        Symbol::Drums => {
            let (sx, sy) = (item.w / 2.0, item.d / 1.8);
            let dc = |x: f64, y: f64, r: f64, f: Fill| circle(x * sx, y * sy, r * sx.min(sy), f);
            out.push(rect(-0.28 * sx, 0.05 * sy, 0.28 * sx, 0.55 * sy, Fill::Mid));
            out.push(dc(-0.42, 0.3, 0.19, Fill::Light));
            out.push(dc(-0.75, 0.05, 0.18, Fill::None));
            out.push(dc(-0.2, -0.2, 0.15, Fill::Light));
            out.push(dc(0.2, -0.2, 0.15, Fill::Light));
            out.push(dc(0.6, 0.25, 0.22, Fill::Light));
            out.push(dc(-0.62, -0.5, 0.24, Fill::None));
            out.push(dc(0.72, -0.4, 0.27, Fill::None));
            out.push(dc(0.0, -0.62, 0.17, Fill::Dark));
        }
        Symbol::Keys => {
            out.push(rect(-hw, -hd, hw, hd, Fill::Light));
            out.push(rect(-hw, hd * 0.2, hw, hd, Fill::None));
            let n = 14;
            for k in 1..n {
                let x = -hw + item.w * k as f64 / n as f64;
                out.push(line(&[(x, hd * 0.2), (x, hd)]));
            }
        }
        Symbol::GuitarAmp | Symbol::BassAmp | Symbol::Speaker => {
            out.push(rect(-hw, -hd, hw, hd, Fill::Dark));
            if item.symbol == Symbol::Speaker {
                out.push(circle(0.0, hd * 0.1, hw.min(hd) * 0.6, Fill::Light));
            } else {
                out.push(rect(-hw * 0.85, hd * 0.3, hw * 0.85, hd * 0.85, Fill::Light));
            }
        }
        Symbol::DiBox | Symbol::Iem | Symbol::Stagebox | Symbol::Power => {
            let f = if item.symbol == Symbol::Power { Fill::Accent } else { Fill::Dark };
            out.push(rect(-hw, -hd, hw, hd, f));
            let t = match item.symbol {
                Symbol::DiBox => "DI",
                Symbol::Iem => "IEM",
                Symbol::Stagebox => "SB",
                _ => "230V",
            };
            let (x, y) = at(0.0, 0.0);
            let size = hd.min(0.16).min(item.w / (t.len() as f64 * 0.7));
            out.push(Shape::Text { x, y, text: t.into(), size, bold: true, light: true });
        }
        Symbol::Riser => {
            out.push(rect(-hw, -hd, hw, hd, Fill::Light));
            // In the bottom left corner, where a kit or a player leaves room.
            let text = format!("Riser {} x {} m", trim(item.w), trim(item.d));
            let half = text.chars().count() as f64 * 0.11 * 0.28;
            let (x, y) = at((-hw + 0.1 + half).min(0.0), hd - 0.14);
            out.push(Shape::Text { x, y, text, size: 0.11, bold: false, light: false });
        }
        Symbol::Person => {
            out.push(circle(0.0, 0.0, hw.min(hd), Fill::Light));
            out.push(circle(0.0, 0.0, hw.min(hd) * 0.45, Fill::Mid));
        }
        Symbol::Chair => {
            out.push(rect(-hw, -hd, hw, hd, Fill::Light));
            out.push(rect(-hw, -hd, hw, -hd * 0.6, Fill::Mid));
        }
        Symbol::Label => {}
    }
    // The item's extent on the stage, after turning.
    let (ex, ey) = ((item.w * c.abs() + item.d * s.abs()) / 2.0, (item.w * s.abs() + item.d * c.abs()) / 2.0);
    // The text under (or on) the item, upright.
    let text = item_text(p, item);
    if !text.is_empty() {
        let (y, size) = match item.symbol {
            Symbol::Label => (item.y, 0.22),
            Symbol::Riser => (item.y - ey + 0.36, 0.13),
            Symbol::MicStand | Symbol::MicSmall => (item.y + ey + 0.13, 0.12),
            _ => (item.y + ey + 0.13, 0.13),
        };
        out.push(Shape::Text { x: item.x, y, text, size, bold: item.symbol == Symbol::Label, light: false });
    }
    // Channel numbers in a badge at the top right.
    let ch = badge(p, item);
    if !ch.is_empty() {
        let n = ch.chars().count() as f64;
        let size = if n > 4.0 { 0.11 } else { 0.13 };
        let r = (0.12_f64).max(n * size * 0.3 + 0.04);
        let (x, y) = (item.x + ex * 0.75 + r * 0.5, item.y - ey * 0.75 - r * 0.3);
        out.push(Shape::Circle { x, y, r, fill: Fill::Accent, tint: String::new() });
        out.push(Shape::Text { x, y, text: ch, size, bold: true, light: true });
    }
    out
}

fn trim(v: f64) -> String {
    crate::time::format_number(v)
}

/// All shapes of the plot, lower layers first.
pub fn plot_shapes(p: &Project, s: &StagePlot) -> Vec<Shape> {
    let mut items: Vec<&StageItem> = s.items.iter().collect();
    items.sort_by_key(|i| i.symbol.layer());
    items.into_iter().flat_map(|i| shapes(p, i)).collect()
}

/// Round to the grid.
pub fn snap(v: f64, grid: f64) -> f64 {
    if grid <= 0.0 {
        v
    } else {
        (v / grid).round() * grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_hit_rotated() {
        let mut it = StageItem::new(1, Symbol::Keys, 5.0, 3.0);
        assert!(it.hit(5.6, 3.0));
        assert!(!it.hit(5.0, 3.6));
        it.rot = 90.0;
        assert!(it.hit(5.0, 3.6));
        assert!(!it.hit(5.6, 3.0));
    }

    #[test]
    fn stands_show_their_channels() {
        let p = crate::demo::band();
        let ids: Vec<Id> = parse_channels(&p, "22, 14").unwrap();
        let mut it = StageItem::new(1, Symbol::MicStand, 5.0, 5.0);
        it.inputs = ids;
        assert_eq!(channels(&p, &it), "14, 22");
        assert_eq!(item_text(&p, &it), "Vox Ida / Ac gtr");
        let sh = shapes(&p, &it);
        assert!(sh.iter().any(|s| matches!(s, Shape::Text { text, .. } if text == "14, 22")));
        assert!(parse_channels(&p, "99").is_err());
        assert_eq!(snap(1.26, 0.5), 1.5);
    }

    #[test]
    fn channel_ranges() {
        let p = crate::demo::band();
        let mut it = StageItem::new(1, Symbol::Drums, 5.0, 5.0);
        it.inputs = parse_channels(&p, "1-7, 9 12").unwrap();
        assert_eq!(it.inputs.len(), 9);
        assert_eq!(channels(&p, &it), "1, 2, 3, 4, 5, 6, 7, 9, 12");
        assert_eq!(badge(&p, &it), "1-7, 9, 12");
        it.inputs = parse_channels(&p, "12, 13").unwrap();
        assert_eq!(badge(&p, &it), "12, 13", "two in a row stay apart");
        assert!(parse_channels(&p, "7-1").is_err());
        assert!(parse_channels(&p, "20-30").is_err(), "there is no input 27");
    }

    #[test]
    fn rotation_turns_the_shapes() {
        let p = Project::default();
        let mut it = StageItem::new(1, Symbol::GuitarAmp, 0.0, 0.0);
        let Shape::Poly { points, .. } = &shapes(&p, &it)[0] else { panic!() };
        assert!((points[1].0 - 0.35).abs() < 1e-9);
        it.rot = 90.0;
        let Shape::Poly { points, .. } = &shapes(&p, &it)[0] else { panic!() };
        assert!((points[1].1 - 0.35).abs() < 1e-9, "{points:?}");
    }
}
