//! The patch as tables: devices, lines, inputs (one column per desk for the
//! splits) and outputs, plus the cable and stand list as a view.

use crate::grid::{col, CellMark, ColKind, Column, Grid};
use crate::model::{Id, Location, Project};
use crate::patch::{
    device_label, device_use, input_path, line_use, link_label, location_name, output_path, pack_list,
    Connector, Destination, Device, DeviceKind, Input, Link, LinkKind, Output, PatchProblem, Stand,
};
use crate::sheet::{move_rows, parse_color, parse_flag, remove_rows};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatchSheet {
    Inputs,
    Outputs,
    Devices,
    Lines,
    Pack,
}

impl PatchSheet {
    pub const ALL: [PatchSheet; 5] = [
        PatchSheet::Inputs,
        PatchSheet::Outputs,
        PatchSheet::Devices,
        PatchSheet::Lines,
        PatchSheet::Pack,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PatchSheet::Inputs => "Input patch",
            PatchSheet::Outputs => "Output patch",
            PatchSheet::Devices => "Devices",
            PatchSheet::Lines => "Lines",
            PatchSheet::Pack => "Cables and stands",
        }
    }

    pub fn noun(self) -> Option<&'static str> {
        match self {
            PatchSheet::Inputs => Some("input"),
            PatchSheet::Outputs => Some("output"),
            PatchSheet::Devices => Some("device"),
            PatchSheet::Lines => Some("line"),
            PatchSheet::Pack => None,
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            PatchSheet::Inputs => "One row per input. Stagebox and port say where it is plugged; the line follows from the stagebox (an analog multicore uses the port as its channel). Each desk and recorder has a column: a channel in several of them is a split.",
            PatchSheet::Outputs => "Desk bus or matrix, the line it travels on, the device and port it comes out of, and where it goes (PA zone, wedge, IEM, recording, broadcast).",
            PatchSheet::Devices => "Stageboxes, racks, desks, splitters, amps, recorders and IEM transmitters with their port counts, at the locations. A new location name is added to the locations.",
            PatchSheet::Lines => "Multicores and network lines (Dante, AES50, MADI, AVB …) between two devices, with their channel count and cable label.",
            PatchSheet::Pack => "Counted from the patch: lines, cables, DI boxes, mics and stands. A packing list for the load-in.",
        }
    }
}

/// A patch table; `problems` are the checks of the current patch.
pub struct PatchGrid<'a> {
    pub sheet: PatchSheet,
    pub problems: &'a [PatchProblem],
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InCol {
    No,
    Name,
    Source,
    Mic,
    Stand,
    Phantom,
    Performer,
    Stagebox,
    Port,
    Line,
    LineCh,
    Desk(Id),
    Cable,
    Notes,
    Color,
    Path,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OutCol {
    Name,
    Desk,
    Bus,
    Line,
    LineCh,
    Device,
    Port,
    Dest,
    Target,
    Performer,
    Mix,
    Cable,
    Notes,
    Color,
    Path,
}

const OUT_COLS: [OutCol; 15] = [
    OutCol::Name,
    OutCol::Desk,
    OutCol::Bus,
    OutCol::Line,
    OutCol::LineCh,
    OutCol::Device,
    OutCol::Port,
    OutCol::Dest,
    OutCol::Target,
    OutCol::Performer,
    OutCol::Mix,
    OutCol::Cable,
    OutCol::Notes,
    OutCol::Color,
    OutCol::Path,
];

fn in_cols(p: &Project) -> Vec<InCol> {
    let mut v = vec![
        InCol::No,
        InCol::Name,
        InCol::Source,
        InCol::Mic,
        InCol::Stand,
        InCol::Phantom,
        InCol::Performer,
        InCol::Stagebox,
        InCol::Port,
        InCol::Line,
        InCol::LineCh,
    ];
    v.extend(p.patch.desks().into_iter().map(InCol::Desk));
    v.extend([InCol::Cable, InCol::Notes, InCol::Color, InCol::Path]);
    v
}

fn names_of(p: &Project, f: impl Fn(&Device) -> bool) -> Vec<String> {
    std::iter::once(String::new())
        .chain(p.patch.devices.iter().filter(|d| f(d)).map(|d| d.name.clone()))
        .collect()
}

fn line_names(p: &Project) -> Vec<String> {
    std::iter::once(String::new()).chain(p.patch.links.iter().map(|l| l.name.clone())).collect()
}

fn opt_num(v: Option<u32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn parse_num(v: &str, what: &str) -> Result<Option<u32>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<u32>().map(Some).map_err(|_| format!("\"{t}\" is not a {what} number"))
}

fn parse_count(v: &str) -> Result<u32, String> {
    Ok(parse_num(v, "")?.unwrap_or(0))
}

fn parse_device(p: &Project, v: &str) -> Result<Option<Id>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    p.patch
        .device_named(t)
        .map(|d| Some(d.id))
        .ok_or(format!("no device \"{t}\" (add it under Devices first)"))
}

fn parse_line(p: &Project, v: &str) -> Result<Option<Id>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    p.patch
        .link_named(t)
        .map(|d| Some(d.id))
        .ok_or(format!("no line \"{t}\" (add it under Lines first)"))
}

fn parse_person(p: &Project, v: &str) -> Result<Option<Id>, String> {
    let t = v.trim();
    if t.is_empty() {
        return Ok(None);
    }
    p.person_named(t).map(|x| Some(x.id)).ok_or(format!("no one called \"{t}\" in the cast"))
}

fn person(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.person(i)).map(|x| x.name.clone()).unwrap_or_default()
}

/// The location with this name, added when new.
fn location_for(p: &mut Project, v: &str) -> Option<Id> {
    let t = v.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(l) = p.locations.iter().find(|l| l.name.trim().eq_ignore_ascii_case(t)) {
        return Some(l.id);
    }
    let id = p.alloc_id();
    p.locations.push(Location { id, name: t.into(), ..Location::default() });
    Some(id)
}

fn choices<T: Copy>(all: &[T], label: fn(T) -> &'static str) -> ColKind {
    ColKind::Choice(all.iter().map(|k| label(*k).to_string()).collect())
}

impl PatchGrid<'_> {
    fn warn(&self, output: bool, row: usize, want: impl Fn(&PatchProblem) -> bool, p: &Project) -> Option<CellMark> {
        let hits: Vec<String> = self
            .problems
            .iter()
            .filter(|x| x.rows().contains(&(output, row)) && want(x))
            .map(|x| x.message(p))
            .collect();
        (!hits.is_empty()).then(|| CellMark { warn: true, hint: hits.join("\n"), ..CellMark::default() })
    }
}

impl Grid for PatchGrid<'_> {
    fn key(&self) -> String {
        format!("patch:{:?}", self.sheet)
    }

    fn columns(&self, p: &Project) -> Vec<Column> {
        use ColKind::*;
        match self.sheet {
            PatchSheet::Inputs => in_cols(p)
                .into_iter()
                .map(|c| match c {
                    InCol::No => col("Ch", Number, 44.0),
                    InCol::Name => col("Name", Text, 130.0),
                    InCol::Source => col("Source", Text, 130.0),
                    InCol::Mic => col("Mic / DI", Text, 100.0),
                    InCol::Stand => col("Stand", choices(&Stand::ALL, Stand::label), 70.0),
                    InCol::Phantom => col("48V", Choice(vec!["".into(), "yes".into()]), 44.0),
                    InCol::Performer => col("Player", Person, 110.0),
                    InCol::Stagebox => col("Stagebox", Choice(names_of(p, |d| !d.kind.is_desk())), 100.0),
                    InCol::Port => col("Port", Number, 48.0),
                    InCol::Line => col("Line", Choice(line_names(p)), 90.0),
                    InCol::LineCh => col("Line ch", Number, 58.0),
                    InCol::Desk(d) => col(device_label(p, Some(d)), Number, 64.0),
                    InCol::Cable => col("Cable", Text, 90.0),
                    InCol::Notes => col("Insert / notes", Text, 170.0),
                    InCol::Color => col("Colour", Color, 80.0),
                    InCol::Path => col("Signal path", Fixed, 380.0),
                })
                .collect(),
            PatchSheet::Outputs => OUT_COLS
                .iter()
                .map(|c| match c {
                    OutCol::Name => col("Name", Text, 130.0),
                    OutCol::Desk => col("Desk", Choice(names_of(p, |d| d.kind.is_desk())), 90.0),
                    OutCol::Bus => col("Bus / matrix", Text, 100.0),
                    OutCol::Line => col("Line", Choice(line_names(p)), 90.0),
                    OutCol::LineCh => col("Line ch", Number, 58.0),
                    OutCol::Device => col("Device", Choice(names_of(p, |d| !d.kind.is_desk())), 100.0),
                    OutCol::Port => col("Port", Number, 48.0),
                    OutCol::Dest => col("Goes to", choices(&Destination::ALL, Destination::label), 90.0),
                    OutCol::Target => col("Speaker / feed", Text, 140.0),
                    OutCol::Performer => col("For", Person, 110.0),
                    OutCol::Mix => col("Mix notes", Text, 170.0),
                    OutCol::Cable => col("Cable", Text, 90.0),
                    OutCol::Notes => col("Notes", Text, 150.0),
                    OutCol::Color => col("Colour", Color, 80.0),
                    OutCol::Path => col("Signal path", Fixed, 360.0),
                })
                .collect(),
            PatchSheet::Devices => vec![
                col("Name", Text, 130.0),
                col("Kind", choices(&DeviceKind::ALL, DeviceKind::label), 110.0),
                col("Location", Text, 130.0),
                col("Model", Text, 140.0),
                col("Inputs", Number, 60.0),
                col("Outputs", Number, 64.0),
                col("Connector", choices(&Connector::ALL, Connector::label), 90.0),
                col("In use", Fixed, 90.0),
                col("Notes", Text, 200.0),
                col("Colour", Color, 80.0),
            ],
            PatchSheet::Lines => vec![
                col("Name", Text, 130.0),
                col("Kind", choices(&LinkKind::ALL, LinkKind::label), 100.0),
                col("From", Choice(names_of(p, |_| true)), 110.0),
                col("To", Choice(names_of(p, |_| true)), 110.0),
                col("Channels", Number, 70.0),
                col("Length", Text, 80.0),
                col("In use", Fixed, 110.0),
                col("Notes", Text, 220.0),
            ],
            PatchSheet::Pack => vec![
                col("Group", Fixed, 100.0),
                col("What", Fixed, 220.0),
                col("Count", Fixed, 60.0),
                col("Notes", Fixed, 300.0),
            ],
        }
    }

    fn len(&self, p: &Project) -> usize {
        match self.sheet {
            PatchSheet::Inputs => p.patch.inputs.len(),
            PatchSheet::Outputs => p.patch.outputs.len(),
            PatchSheet::Devices => p.patch.devices.len(),
            PatchSheet::Lines => p.patch.links.len(),
            PatchSheet::Pack => pack_list(p).len(),
        }
    }

    fn text(&self, p: &Project, row: usize, c: usize) -> String {
        let pa = &p.patch;
        match self.sheet {
            PatchSheet::Inputs => {
                let (Some(i), Some(c)) = (pa.inputs.get(row), in_cols(p).get(c).copied()) else { return String::new() };
                match c {
                    InCol::No => i.no.to_string(),
                    InCol::Name => i.name.clone(),
                    InCol::Source => i.source.clone(),
                    InCol::Mic => i.mic.clone(),
                    InCol::Stand => i.stand.label().into(),
                    InCol::Phantom => if i.phantom { "yes".into() } else { String::new() },
                    InCol::Performer => person(p, i.performer),
                    InCol::Stagebox => device_label(p, i.stagebox),
                    InCol::Port => opt_num(i.port),
                    InCol::Line => link_label(p, pa.input_line(i).map(|x| x.0)),
                    InCol::LineCh => opt_num(pa.input_line(i).and_then(|x| x.1)),
                    InCol::Desk(d) => opt_num(i.feed(d)),
                    InCol::Cable => i.cable.clone(),
                    InCol::Notes => i.notes.clone(),
                    InCol::Color => i.color.clone(),
                    InCol::Path => input_path(p, i).join("  >  "),
                }
            }
            PatchSheet::Outputs => {
                let (Some(o), Some(c)) = (pa.outputs.get(row), OUT_COLS.get(c).copied()) else { return String::new() };
                match c {
                    OutCol::Name => o.name.clone(),
                    OutCol::Desk => device_label(p, o.desk),
                    OutCol::Bus => o.bus.clone(),
                    OutCol::Line => link_label(p, pa.output_line(o).map(|x| x.0)),
                    OutCol::LineCh => opt_num(pa.output_line(o).and_then(|x| x.1)),
                    OutCol::Device => device_label(p, o.device),
                    OutCol::Port => opt_num(o.port),
                    OutCol::Dest => o.dest.label().into(),
                    OutCol::Target => o.target.clone(),
                    OutCol::Performer => person(p, o.performer),
                    OutCol::Mix => o.mix_notes.clone(),
                    OutCol::Cable => o.cable.clone(),
                    OutCol::Notes => o.notes.clone(),
                    OutCol::Color => o.color.clone(),
                    OutCol::Path => output_path(p, o).join("  >  "),
                }
            }
            PatchSheet::Devices => {
                let Some(d) = pa.devices.get(row) else { return String::new() };
                match c {
                    0 => d.name.clone(),
                    1 => d.kind.label().into(),
                    2 => location_name(p, d.location),
                    3 => d.model.clone(),
                    4 => d.inputs.to_string(),
                    5 => d.outputs.to_string(),
                    6 => d.connector.label().into(),
                    7 => {
                        let (i, o) = device_use(p, d.id);
                        format!("{i}/{} in, {o}/{} out", d.inputs, d.outputs)
                    }
                    8 => d.notes.clone(),
                    9 => d.color.clone(),
                    _ => String::new(),
                }
            }
            PatchSheet::Lines => {
                let Some(l) = pa.links.get(row) else { return String::new() };
                match c {
                    0 => l.name.clone(),
                    1 => l.kind.label().into(),
                    2 => device_label(p, l.from),
                    3 => device_label(p, l.to),
                    4 => l.channels.to_string(),
                    5 => l.length.clone(),
                    6 => {
                        let (i, o) = line_use(p, l.id);
                        format!("{i} sends, {o} returns")
                    }
                    7 => l.notes.clone(),
                    _ => String::new(),
                }
            }
            PatchSheet::Pack => {
                let list = pack_list(p);
                let Some(x) = list.get(row) else { return String::new() };
                match c {
                    0 => x.group.into(),
                    1 => x.what.clone(),
                    2 => x.count.to_string(),
                    3 => x.notes.clone(),
                    _ => String::new(),
                }
            }
        }
    }

    fn set(&self, p: &mut Project, row: usize, c: usize, v: &str) -> Result<(), String> {
        let t = v.trim();
        match self.sheet {
            PatchSheet::Inputs => {
                let c = in_cols(p).get(c).copied().ok_or("no such column")?;
                let mut i = p.patch.inputs.get(row).cloned().ok_or("no such input")?;
                match c {
                    InCol::No => i.no = parse_num(t, "channel")?.ok_or("the channel needs a number")?,
                    InCol::Name => i.name = t.into(),
                    InCol::Source => i.source = t.into(),
                    InCol::Mic => {
                        let was = crate::patch::needs_phantom(&i.mic);
                        i.mic = t.into();
                        // A known condenser switches 48V on.
                        if !was && crate::patch::needs_phantom(t) {
                            i.phantom = true;
                        }
                    }
                    InCol::Stand => i.stand = Stand::parse(t).ok_or(format!("\"{t}\" is not a stand (small, tall, clamp, floor)"))?,
                    InCol::Phantom => i.phantom = parse_flag(t)?,
                    InCol::Performer => i.performer = parse_person(p, t)?,
                    InCol::Stagebox => i.stagebox = parse_device(p, t)?,
                    InCol::Port => i.port = parse_num(t, "port")?,
                    InCol::Line => i.link = parse_line(p, t)?,
                    InCol::LineCh => i.link_ch = parse_num(t, "channel")?,
                    InCol::Desk(d) => i.set_feed(d, parse_num(t, "channel")?),
                    InCol::Cable => i.cable = t.into(),
                    InCol::Notes => i.notes = v.into(),
                    InCol::Color => i.color = parse_color(t)?,
                    InCol::Path => return Err("the signal path is worked out".into()),
                }
                if let Some(slot) = p.patch.inputs.get_mut(row) {
                    *slot = i;
                }
            }
            PatchSheet::Outputs => {
                let c = OUT_COLS.get(c).copied().ok_or("no such column")?;
                let mut o = p.patch.outputs.get(row).cloned().ok_or("no such output")?;
                match c {
                    OutCol::Name => o.name = t.into(),
                    OutCol::Desk => o.desk = parse_device(p, t)?,
                    OutCol::Bus => o.bus = t.into(),
                    OutCol::Line => o.link = parse_line(p, t)?,
                    OutCol::LineCh => o.link_ch = parse_num(t, "channel")?,
                    OutCol::Device => o.device = parse_device(p, t)?,
                    OutCol::Port => o.port = parse_num(t, "port")?,
                    OutCol::Dest => {
                        o.dest = Destination::parse(t).ok_or(format!("\"{t}\" is not a destination (PA, monitor, IEM, recording, broadcast)"))?
                    }
                    OutCol::Target => o.target = t.into(),
                    OutCol::Performer => o.performer = parse_person(p, t)?,
                    OutCol::Mix => o.mix_notes = v.into(),
                    OutCol::Cable => o.cable = t.into(),
                    OutCol::Notes => o.notes = v.into(),
                    OutCol::Color => o.color = parse_color(t)?,
                    OutCol::Path => return Err("the signal path is worked out".into()),
                }
                if let Some(slot) = p.patch.outputs.get_mut(row) {
                    *slot = o;
                }
            }
            PatchSheet::Devices => {
                let mut d = p.patch.devices.get(row).cloned().ok_or("no such device")?;
                match c {
                    0 => {
                        if p.patch.devices.iter().enumerate().any(|(k, x)| k != row && !t.is_empty() && x.name.trim().eq_ignore_ascii_case(t)) {
                            return Err(format!("there is already a device called \"{t}\""));
                        }
                        d.name = t.into()
                    }
                    1 => d.kind = DeviceKind::parse(t).ok_or(format!("\"{t}\" is not a kind of device"))?,
                    2 => d.location = location_for(p, t),
                    3 => d.model = t.into(),
                    4 => d.inputs = parse_count(t)?,
                    5 => d.outputs = parse_count(t)?,
                    6 => d.connector = Connector::parse(t).ok_or(format!("\"{t}\" is not a connector (XLR, jack, AES, network, Speakon)"))?,
                    8 => d.notes = v.into(),
                    9 => d.color = parse_color(t)?,
                    _ => return Err("this column is worked out".into()),
                }
                if let Some(slot) = p.patch.devices.get_mut(row) {
                    *slot = d;
                }
            }
            PatchSheet::Lines => {
                let mut l = p.patch.links.get(row).cloned().ok_or("no such line")?;
                match c {
                    0 => {
                        if p.patch.links.iter().enumerate().any(|(k, x)| k != row && !t.is_empty() && x.name.trim().eq_ignore_ascii_case(t)) {
                            return Err(format!("there is already a line called \"{t}\""));
                        }
                        l.name = t.into()
                    }
                    1 => l.kind = LinkKind::parse(t).ok_or(format!("\"{t}\" is not a kind of line (multicore, Dante, AES50, MADI, AVB)"))?,
                    2 => l.from = parse_device(p, t)?,
                    3 => l.to = parse_device(p, t)?,
                    4 => l.channels = parse_count(t)?,
                    5 => l.length = t.into(),
                    7 => l.notes = v.into(),
                    _ => return Err("this column is worked out".into()),
                }
                if let Some(slot) = p.patch.links.get_mut(row) {
                    *slot = l;
                }
            }
            PatchSheet::Pack => return Err("the list is counted from the patch".into()),
        }
        Ok(())
    }

    fn color(&self, p: &Project, row: usize) -> String {
        let pa = &p.patch;
        match self.sheet {
            PatchSheet::Inputs => pa.inputs.get(row).map(|x| x.color.clone()),
            PatchSheet::Outputs => pa.outputs.get(row).map(|x| x.color.clone()),
            PatchSheet::Devices => pa.devices.get(row).map(|x| x.color.clone()),
            _ => None,
        }
        .unwrap_or_default()
    }

    fn mark(&self, p: &Project, row: usize, c: usize) -> Option<CellMark> {
        match self.sheet {
            PatchSheet::Inputs => {
                let c = in_cols(p).get(c).copied()?;
                let i = p.patch.inputs.get(row)?;
                let w = |f: &dyn Fn(&PatchProblem) -> bool| self.warn(false, row, f, p);
                let auto = |hint: &str| Some(CellMark { hint: hint.into(), ..CellMark::default() });
                match c {
                    InCol::Port => w(&|x| matches!(x, PatchProblem::PortTwice { .. } | PatchProblem::PortOutOfRange { .. })),
                    InCol::LineCh => w(&|x| matches!(x, PatchProblem::LineChannelTwice { .. } | PatchProblem::LineChannelOutOfRange { .. }))
                        .or_else(|| (i.link_ch.is_none() && p.patch.input_line(i).and_then(|x| x.1).is_some()).then(|| auto("From the port (analog multicore)")).flatten()),
                    InCol::Line => (i.link.is_none() && p.patch.input_line(i).is_some()).then(|| auto("The line of the stagebox")).flatten(),
                    InCol::Desk(d) => w(&|x| match x {
                        PatchProblem::DeskChannelTwice { device, .. } | PatchProblem::DeskChannelOutOfRange { device, .. } => *device == d,
                        _ => false,
                    }),
                    InCol::Phantom => w(&|x| matches!(x, PatchProblem::NoPhantom { .. })),
                    InCol::Name => w(&|x| matches!(x, PatchProblem::NoTarget { .. })),
                    _ => None,
                }
            }
            PatchSheet::Outputs => {
                let c = OUT_COLS.get(c).copied()?;
                let w = |f: &dyn Fn(&PatchProblem) -> bool| self.warn(true, row, f, p);
                match c {
                    OutCol::Port => w(&|x| matches!(x, PatchProblem::PortTwice { .. } | PatchProblem::PortOutOfRange { .. })),
                    OutCol::LineCh => w(&|x| matches!(x, PatchProblem::LineChannelTwice { .. } | PatchProblem::LineChannelOutOfRange { .. })),
                    OutCol::Name => w(&|x| matches!(x, PatchProblem::NoTarget { .. })),
                    _ => None,
                }
            }
            PatchSheet::Lines if c == 6 => {
                let l = p.patch.links.get(row)?;
                let hits: Vec<String> = self
                    .problems
                    .iter()
                    .filter(|x| matches!(x, PatchProblem::LineFull { link, .. } if *link == l.id))
                    .map(|x| x.message(p))
                    .collect();
                (!hits.is_empty()).then(|| CellMark { warn: true, hint: hits.join("\n"), ..CellMark::default() })
            }
            _ => None,
        }
    }

    fn editable_rows(&self) -> bool {
        self.sheet != PatchSheet::Pack
    }

    fn insert(&self, p: &mut Project, at: usize) -> usize {
        let id = p.alloc_id();
        let pa = &mut p.patch;
        match self.sheet {
            PatchSheet::Inputs => {
                // The next number, the stagebox and port after the row above.
                let at = at.min(pa.inputs.len());
                let no = pa.next_input_no();
                let above = at.checked_sub(1).and_then(|k| pa.inputs.get(k)).cloned();
                let mut i = Input { id, no, ..Input::default() };
                if let Some(a) = above {
                    i.stagebox = a.stagebox;
                    i.port = a.port.map(|x| x + 1);
                    i.link = a.link;
                    i.feeds = a.feeds.iter().map(|f| crate::patch::Feed { device: f.device, channel: f.channel + 1 }).collect();
                } else {
                    i.feeds = pa.desks().first().map(|d| vec![crate::patch::Feed { device: *d, channel: no }]).unwrap_or_default();
                }
                pa.inputs.insert(at, i);
                at
            }
            PatchSheet::Outputs => {
                let at = at.min(pa.outputs.len());
                pa.outputs.insert(at, Output { id, ..Output::default() });
                at
            }
            PatchSheet::Devices => {
                let at = at.min(pa.devices.len());
                pa.devices.insert(at, Device { id, inputs: 16, outputs: 8, ..Device::default() });
                at
            }
            PatchSheet::Lines => {
                let at = at.min(pa.links.len());
                pa.links.insert(at, Link { id, channels: 16, ..Link::default() });
                at
            }
            PatchSheet::Pack => at,
        }
    }

    fn remove(&self, p: &mut Project, rows: &[usize]) {
        let pa = &mut p.patch;
        match self.sheet {
            PatchSheet::Inputs => remove_rows(&mut pa.inputs, rows),
            PatchSheet::Outputs => remove_rows(&mut pa.outputs, rows),
            PatchSheet::Devices => remove_rows(&mut pa.devices, rows),
            PatchSheet::Lines => remove_rows(&mut pa.links, rows),
            PatchSheet::Pack => {}
        }
        p.prune();
    }

    fn move_rows(&self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        let pa = &mut p.patch;
        match self.sheet {
            PatchSheet::Inputs => move_rows(&mut pa.inputs, rows, to),
            PatchSheet::Outputs => move_rows(&mut pa.outputs, rows, to),
            PatchSheet::Devices => move_rows(&mut pa.devices, rows, to),
            PatchSheet::Lines => move_rows(&mut pa.links, rows, to),
            PatchSheet::Pack => rows.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::paste;
    use crate::patch::check;

    fn grid(sheet: PatchSheet) -> PatchGrid<'static> {
        PatchGrid { sheet, problems: &[] }
    }

    #[test]
    fn building_a_patch_by_pasting() {
        let mut p = Project::default();
        let (_, e) = paste(&grid(PatchSheet::Devices), &mut p, 0, 0, "SB1\tStagebox\tDrum riser\t\t16\t8\nFOH\tdesk\tFOH\t\t48\t16\nMON\tConsole\tMonitor world\t\t48\t16");
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(p.locations.len(), 3, "new locations were added");
        let (_, e) = paste(&grid(PatchSheet::Lines), &mut p, 0, 0, "MC1\tmulticore\tSB1\tFOH\t16\t50 m");
        assert!(e.is_empty(), "{e:?}");
        // Inputs: the desk columns follow the devices.
        let cols: Vec<String> = grid(PatchSheet::Inputs).columns(&p).into_iter().map(|c| c.title).collect();
        let foh = cols.iter().position(|c| c == "FOH").unwrap();
        assert_eq!(cols[foh + 1], "MON");
        let g = grid(PatchSheet::Inputs);
        g.insert(&mut p, 0);
        g.set(&mut p, 0, 1, "Kick in").unwrap();
        g.set(&mut p, 0, 3, "Beta 91").unwrap();
        g.set(&mut p, 0, 7, "SB1").unwrap();
        g.set(&mut p, 0, 8, "1").unwrap();
        g.set(&mut p, 0, foh + 1, "1").unwrap();
        assert!(p.patch.inputs[0].phantom, "a known condenser switches 48V on");
        assert_eq!(p.patch.inputs[0].feeds.len(), 2, "a split to FOH and MON");
        // The next row follows on.
        g.insert(&mut p, 1);
        assert_eq!(p.patch.inputs[1].no, 2);
        assert_eq!(p.patch.inputs[1].port, Some(2));
        assert_eq!(g.text(&p, 1, 10), "2", "line channel from the port");
        assert_eq!(g.text(&p, 0, cols.len() - 1), "Kick in (Beta 91)  >  SB1 in 1 (Drum riser)  >  MC1 ch 1  >  FOH 1 / MON 1");
        assert!(g.set(&mut p, 0, 7, "SB9").is_err());
        assert!(g.set(&mut p, 0, 4, "huge").is_err());
    }

    #[test]
    fn problems_mark_their_cells() {
        let mut p = Project::default();
        paste(&grid(PatchSheet::Devices), &mut p, 0, 0, "SB1\tStagebox\t\t\t8\t4\nFOH\tdesk\t\t\t8\t8");
        let g = grid(PatchSheet::Inputs);
        paste(&g, &mut p, 0, 0, "1\tKick\t\tSM57\t\t\t\tSB1\t1\t\t\t1\n2\tSnare\t\tSM57\t\t\t\tSB1\t1\t\t\t2");
        let probs = check(&p);
        let g = PatchGrid { sheet: PatchSheet::Inputs, problems: &probs };
        let m = g.mark(&p, 1, 8).expect("port clash");
        assert!(m.warn && m.hint.contains("SB1 port 1"), "{}", m.hint);
        assert!(g.mark(&p, 1, 11).is_none());
        let n = grid(PatchSheet::Pack).len(&p);
        assert!(n >= 2);
        assert!(grid(PatchSheet::Pack).set(&mut p, 0, 0, "x").is_err());
    }
}
