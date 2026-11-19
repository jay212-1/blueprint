//! The patch: devices with ports at the locations (stageboxes, racks, desks,
//! splitters, amps, recorders, IEM transmitters), the lines between them
//! (multicore, Dante, AES50, MADI, AVB …), the input channels with their
//! whole signal path (source → stagebox port → line channel → desk channels,
//! one per desk for splits) and the outputs (desk bus → line → port → where
//! it goes). Plans refer to devices and lines by id; the checks, the cable
//! and stand list and the paths are worked out from it.

use crate::model::{Id, Project};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum DeviceKind {
    #[default]
    Stagebox,
    IoRack,
    Console,
    Splitter,
    Amp,
    Recorder,
    Iem,
    Other,
}

impl DeviceKind {
    pub const ALL: [DeviceKind; 8] = [
        DeviceKind::Stagebox,
        DeviceKind::IoRack,
        DeviceKind::Console,
        DeviceKind::Splitter,
        DeviceKind::Amp,
        DeviceKind::Recorder,
        DeviceKind::Iem,
        DeviceKind::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Stagebox => "Stagebox",
            DeviceKind::IoRack => "I/O rack",
            DeviceKind::Console => "Console",
            DeviceKind::Splitter => "Splitter",
            DeviceKind::Amp => "Amp",
            DeviceKind::Recorder => "Recorder",
            DeviceKind::Iem => "IEM transmitter",
            DeviceKind::Other => "Other",
        }
    }

    pub fn parse(v: &str) -> Option<DeviceKind> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(DeviceKind::Stagebox);
        }
        let alias = match v.as_str() {
            "desk" | "mixer" | "console" => Some(DeviceKind::Console),
            "rack" | "io" | "i/o" => Some(DeviceKind::IoRack),
            "iem" | "in-ear" => Some(DeviceKind::Iem),
            "recording" | "rec" => Some(DeviceKind::Recorder),
            "split" => Some(DeviceKind::Splitter),
            _ => None,
        };
        alias.or_else(|| {
            DeviceKind::ALL.into_iter().find(|k| {
                let l = k.label().to_ascii_lowercase();
                l == v || (v.len() >= 3 && l.starts_with(&v))
            })
        })
    }

    /// Takes input channels from the patch (gets a column in the input list).
    pub fn is_desk(self) -> bool {
        matches!(self, DeviceKind::Console | DeviceKind::Recorder)
    }
}

/// How the ports of a device are plugged.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Connector {
    #[default]
    Xlr,
    Jack,
    Aes,
    Network,
    Speakon,
    Other,
}

impl Connector {
    pub const ALL: [Connector; 6] = [
        Connector::Xlr,
        Connector::Jack,
        Connector::Aes,
        Connector::Network,
        Connector::Speakon,
        Connector::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Connector::Xlr => "XLR",
            Connector::Jack => "Jack",
            Connector::Aes => "AES",
            Connector::Network => "Network",
            Connector::Speakon => "Speakon",
            Connector::Other => "Other",
        }
    }

    pub fn parse(v: &str) -> Option<Connector> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(Connector::Xlr);
        }
        match v.as_str() {
            "trs" | "ts" | "klinke" | "6.3" => return Some(Connector::Jack),
            "rj45" | "ethercon" | "cat" | "net" => return Some(Connector::Network),
            "nl4" | "nl8" => return Some(Connector::Speakon),
            _ => {}
        }
        Connector::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(&v))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Device {
    pub id: Id,
    pub name: String,
    pub kind: DeviceKind,
    pub location: Option<Id>,
    pub model: String,
    /// Number of inputs (desk: input channels).
    pub inputs: u32,
    pub outputs: u32,
    pub connector: Connector,
    pub notes: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum LinkKind {
    #[default]
    Multicore,
    Dante,
    Aes50,
    Madi,
    Avb,
    Aes,
    Other,
}

impl LinkKind {
    pub const ALL: [LinkKind; 7] = [
        LinkKind::Multicore,
        LinkKind::Dante,
        LinkKind::Aes50,
        LinkKind::Madi,
        LinkKind::Avb,
        LinkKind::Aes,
        LinkKind::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Multicore => "Multicore",
            LinkKind::Dante => "Dante",
            LinkKind::Aes50 => "AES50",
            LinkKind::Madi => "MADI",
            LinkKind::Avb => "AVB",
            LinkKind::Aes => "AES/EBU",
            LinkKind::Other => "Other",
        }
    }

    pub fn parse(v: &str) -> Option<LinkKind> {
        let v = v.trim().to_ascii_lowercase().replace([' ', '-'], "");
        if v.is_empty() {
            return Some(LinkKind::Multicore);
        }
        if v == "analog" || v == "analogue" || v == "snake" {
            return Some(LinkKind::Multicore);
        }
        if v == "aes" || v == "aesebu" || v == "aes/ebu" {
            return Some(LinkKind::Aes);
        }
        LinkKind::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase().replace([' ', '-'], "");
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }

    /// Analog: the line channel is the stagebox port.
    pub fn is_analog(self) -> bool {
        self == LinkKind::Multicore
    }
}

/// A line between two devices (a multicore, a network link).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Link {
    pub id: Id,
    /// Cable label ("MC1", "Dante primary").
    pub name: String,
    pub kind: LinkKind,
    pub from: Option<Id>,
    pub to: Option<Id>,
    pub channels: u32,
    /// "50 m", "CAT6 80 m".
    pub length: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Stand {
    #[default]
    None,
    Small,
    Tall,
    Clamp,
    Floor,
}

impl Stand {
    pub const ALL: [Stand; 5] = [Stand::None, Stand::Small, Stand::Tall, Stand::Clamp, Stand::Floor];

    pub fn label(self) -> &'static str {
        match self {
            Stand::None => "",
            Stand::Small => "Small",
            Stand::Tall => "Tall",
            Stand::Clamp => "Clamp",
            Stand::Floor => "Floor",
        }
    }

    /// For the stand list.
    pub fn long(self) -> &'static str {
        match self {
            Stand::None => "",
            Stand::Small => "Small boom stand",
            Stand::Tall => "Tall boom stand",
            Stand::Clamp => "Clamp",
            Stand::Floor => "Floor / boundary plate",
        }
    }

    pub fn parse(v: &str) -> Option<Stand> {
        let v = v.trim().to_ascii_lowercase();
        Some(match v.as_str() {
            "" | "-" | "none" | "no" => Stand::None,
            "small" | "s" | "short" | "klein" => Stand::Small,
            "tall" | "t" | "big" | "large" | "l" | "groß" | "gross" => Stand::Tall,
            "clamp" | "c" | "clip" => Stand::Clamp,
            "floor" | "f" | "plate" | "boundary" => Stand::Floor,
            _ => return None,
        })
    }
}

/// One desk (or recorder) channel an input goes to.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(default)]
pub struct Feed {
    pub device: Id,
    pub channel: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Input {
    pub id: Id,
    /// Number in the input list.
    pub no: u32,
    pub name: String,
    /// Instrument or source ("Kick in", "Vocal Ida").
    pub source: String,
    /// Mic model or "DI".
    pub mic: String,
    pub stand: Stand,
    pub phantom: bool,
    pub performer: Option<Id>,
    pub stagebox: Option<Id>,
    pub port: Option<u32>,
    /// The line (taken from the stagebox when there is exactly one).
    pub link: Option<Id>,
    /// Channel on the line (an analog multicore uses the port).
    pub link_ch: Option<u32>,
    /// Desk channels: one per desk for splits.
    pub feeds: Vec<Feed>,
    /// "XLR 10 m" (empty: the stagebox connector).
    pub cable: String,
    pub notes: String,
    pub color: String,
}

impl Input {
    pub fn feed(&self, desk: Id) -> Option<u32> {
        self.feeds.iter().find(|f| f.device == desk).map(|f| f.channel)
    }

    pub fn set_feed(&mut self, desk: Id, channel: Option<u32>) {
        self.feeds.retain(|f| f.device != desk);
        if let Some(c) = channel {
            self.feeds.push(Feed { device: desk, channel: c });
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Destination {
    #[default]
    Pa,
    Monitor,
    Iem,
    Recording,
    Broadcast,
    Other,
}

impl Destination {
    pub const ALL: [Destination; 6] = [
        Destination::Pa,
        Destination::Monitor,
        Destination::Iem,
        Destination::Recording,
        Destination::Broadcast,
        Destination::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Destination::Pa => "PA",
            Destination::Monitor => "Monitor",
            Destination::Iem => "IEM",
            Destination::Recording => "Recording",
            Destination::Broadcast => "Broadcast",
            Destination::Other => "Other",
        }
    }

    pub fn parse(v: &str) -> Option<Destination> {
        let v = v.trim().to_ascii_lowercase();
        if v.is_empty() {
            return Some(Destination::Pa);
        }
        match v.as_str() {
            "wedge" | "mon" | "sidefill" => return Some(Destination::Monitor),
            "in-ear" | "inear" => return Some(Destination::Iem),
            "rec" => return Some(Destination::Recording),
            "tv" | "stream" | "feed" => return Some(Destination::Broadcast),
            _ => {}
        }
        Destination::ALL.into_iter().find(|k| {
            let l = k.label().to_ascii_lowercase();
            l == v || (v.len() >= 3 && l.starts_with(&v))
        })
    }

    /// Monitor ways (the monitor / IEM list).
    pub fn is_monitor(self) -> bool {
        matches!(self, Destination::Monitor | Destination::Iem)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Output {
    pub id: Id,
    /// "PA L", "Wedge Ida", "IEM Malte".
    pub name: String,
    pub desk: Option<Id>,
    /// Bus, matrix or output of the desk ("Mix 3", "Matrix 1", "Main L").
    pub bus: String,
    pub link: Option<Id>,
    pub link_ch: Option<u32>,
    /// The device it comes out of (stagebox, amp rack, IEM transmitter).
    pub device: Option<Id>,
    pub port: Option<u32>,
    pub dest: Destination,
    /// Speaker, wedge position or feed ("Wedge DS centre", "Sub array").
    pub target: String,
    pub performer: Option<Id>,
    /// "Ida: more vocal, less kick".
    pub mix_notes: String,
    pub cable: String,
    pub notes: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Patch {
    pub devices: Vec<Device>,
    pub links: Vec<Link>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

impl Patch {
    pub fn device(&self, id: Id) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    pub fn link(&self, id: Id) -> Option<&Link> {
        self.links.iter().find(|l| l.id == id)
    }

    pub fn device_named(&self, name: &str) -> Option<&Device> {
        let n = name.trim();
        self.devices
            .iter()
            .find(|d| !n.is_empty() && d.name.trim().eq_ignore_ascii_case(n))
    }

    pub fn link_named(&self, name: &str) -> Option<&Link> {
        let n = name.trim();
        self.links
            .iter()
            .find(|l| !n.is_empty() && l.name.trim().eq_ignore_ascii_case(n))
    }

    /// Desks and recorders, in device order: one feed column each.
    pub fn desks(&self) -> Vec<Id> {
        self.devices.iter().filter(|d| d.kind.is_desk()).map(|d| d.id).collect()
    }

    /// The line from (or to) a device, when there is exactly one.
    pub fn only_link(&self, device: Id) -> Option<Id> {
        let mut it = self
            .links
            .iter()
            .filter(|l| l.from == Some(device) || l.to == Some(device));
        let first = it.next()?;
        it.next().is_none().then_some(first.id)
    }

    /// The line an input travels on, and its channel there.
    pub fn input_line(&self, i: &Input) -> Option<(Id, Option<u32>)> {
        let link = i.link.or_else(|| i.stagebox.and_then(|s| self.only_link(s)))?;
        let analog = self.link(link).is_some_and(|l| l.kind.is_analog());
        Some((link, i.link_ch.or(if analog { i.port } else { None })))
    }

    /// The line an output travels on, and its channel there.
    pub fn output_line(&self, o: &Output) -> Option<(Id, Option<u32>)> {
        let link = o.link.or_else(|| o.device.and_then(|s| self.only_link(s)))?;
        let analog = self.link(link).is_some_and(|l| l.kind.is_analog());
        Some((link, o.link_ch.or(if analog { o.port } else { None })))
    }

    /// The next free input number.
    pub fn next_input_no(&self) -> u32 {
        self.inputs.iter().map(|i| i.no).max().unwrap_or(0) + 1
    }

    pub(crate) fn prune(&mut self, people: &[Id], locations: &[Id]) {
        let devices: Vec<Id> = self.devices.iter().map(|d| d.id).collect();
        let links: Vec<Id> = self.links.iter().map(|l| l.id).collect();
        let keep = |x: &mut Option<Id>, ids: &[Id]| {
            if x.is_some_and(|v| !ids.contains(&v)) {
                *x = None;
            }
        };
        for d in &mut self.devices {
            keep(&mut d.location, locations);
        }
        for l in &mut self.links {
            keep(&mut l.from, &devices);
            keep(&mut l.to, &devices);
        }
        for i in &mut self.inputs {
            keep(&mut i.stagebox, &devices);
            keep(&mut i.link, &links);
            keep(&mut i.performer, people);
            i.feeds.retain(|f| devices.contains(&f.device));
        }
        for o in &mut self.outputs {
            keep(&mut o.desk, &devices);
            keep(&mut o.device, &devices);
            keep(&mut o.link, &links);
            keep(&mut o.performer, people);
        }
    }
}

/// "SB1 (Drum riser)".
pub fn device_label(p: &Project, id: Option<Id>) -> String {
    let Some(d) = id.and_then(|i| p.patch.device(i)) else { return String::new() };
    d.name.clone()
}

pub fn link_label(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.patch.link(i)).map(|l| l.name.clone()).unwrap_or_default()
}

pub fn location_name(p: &Project, id: Option<Id>) -> String {
    id.and_then(|i| p.locations.iter().find(|l| l.id == i))
        .map(|l| l.name.clone())
        .unwrap_or_default()
}

/// The steps of an input's signal path (source, stagebox port, line, desks).
pub fn input_path(p: &Project, i: &Input) -> Vec<String> {
    let pa = &p.patch;
    let mut steps = Vec::new();
    let what = if i.source.trim().is_empty() { i.name.trim() } else { i.source.trim() };
    steps.push(match (what.is_empty(), i.mic.trim().is_empty()) {
        (_, true) => what.to_string(),
        (true, false) => i.mic.trim().to_string(),
        (false, false) => format!("{what} ({})", i.mic.trim()),
    });
    if let Some(sb) = i.stagebox.and_then(|s| pa.device(s)) {
        let at = location_name(p, sb.location);
        let port = i.port.map(|x| format!(" in {x}")).unwrap_or_default();
        steps.push(if at.is_empty() { format!("{}{port}", sb.name) } else { format!("{}{port} ({at})", sb.name) });
    }
    if let Some((l, ch)) = pa.input_line(i) {
        if let Some(l) = pa.link(l) {
            steps.push(match ch {
                Some(c) => format!("{} ch {c}", l.name),
                None => l.name.clone(),
            });
        }
    }
    let feeds: Vec<String> = pa
        .desks()
        .iter()
        .filter_map(|d| i.feed(*d).map(|c| (d, c)))
        .filter_map(|(d, c)| pa.device(*d).map(|x| format!("{} {c}", x.name)))
        .collect();
    if !feeds.is_empty() {
        steps.push(feeds.join(" / "));
    }
    steps
}

/// The steps of an output's path (desk bus, line, device port, where to).
pub fn output_path(p: &Project, o: &Output) -> Vec<String> {
    let pa = &p.patch;
    let mut steps = Vec::new();
    let desk = device_label(p, o.desk);
    let from = [desk.as_str(), o.bus.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if !from.is_empty() {
        steps.push(from);
    }
    if let Some((l, ch)) = pa.output_line(o) {
        if let Some(l) = pa.link(l) {
            steps.push(match ch {
                Some(c) => format!("{} ch {c}", l.name),
                None => l.name.clone(),
            });
        }
    }
    if let Some(d) = o.device.and_then(|s| pa.device(s)) {
        let port = o.port.map(|x| format!(" out {x}")).unwrap_or_default();
        steps.push(format!("{}{port}", d.name));
    }
    let to = [o.dest.label(), o.target.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(": ");
    steps.push(to);
    steps
}

/// Mics that need phantom power (models known to be condensers, or the word
/// "condenser" in the mic column).
const CONDENSERS: [&str; 22] = [
    "condenser", "km184", "km 184", "c414", "c 414", "c451", "sm81", "e614", "e 614", "at4050",
    "at4040", "at4033", "ksm32", "ksm137", "nt5", "nt55", "u87", "tlm", "c214", "m930", "beta 91",
    "beta91",
];

pub fn needs_phantom(mic: &str) -> bool {
    let m = mic.to_ascii_lowercase();
    !m.is_empty() && CONDENSERS.iter().any(|c| m.contains(c))
}

/// A DI box (the mic column says "DI").
pub fn is_di(mic: &str) -> bool {
    let m = mic.trim().to_ascii_lowercase();
    m == "di" || m.starts_with("di ") || m.ends_with(" di") || m.contains("di box") || m.contains("di-box")
}

/// What is wrong in the patch.
#[derive(Clone, Debug, PartialEq)]
pub enum PatchProblem {
    /// Two inputs (or two outputs) on the same port of a device.
    PortTwice { device: Id, port: u32, output: bool, rows: Vec<usize> },
    /// Two inputs on the same desk channel.
    DeskChannelTwice { device: Id, channel: u32, rows: Vec<usize> },
    /// Two signals on the same channel of a line.
    LineChannelTwice { link: Id, channel: u32, output: bool, rows: Vec<usize> },
    /// An input that reaches no desk, an output that goes nowhere.
    NoTarget { row: usize, output: bool },
    /// A port number above the device's port count.
    PortOutOfRange { device: Id, port: u32, output: bool, row: usize },
    /// A desk channel above the desk's channel count.
    DeskChannelOutOfRange { device: Id, channel: u32, row: usize },
    /// More channels on a line than it carries (or a channel above it).
    LineFull { link: Id, used: u32 },
    LineChannelOutOfRange { link: Id, channel: u32, output: bool, row: usize },
    /// A condenser mic without phantom power.
    NoPhantom { row: usize },
}

impl PatchProblem {
    pub fn message(&self, p: &Project) -> String {
        let pa = &p.patch;
        let dev = |id: &Id| pa.device(*id).map(|d| d.name.clone()).unwrap_or_default();
        let line = |id: &Id| pa.link(*id).map(|d| d.name.clone()).unwrap_or_default();
        let what = |output: bool| if output { "outputs" } else { "inputs" };
        let name = |row: &usize, output: bool| {
            if output {
                pa.outputs.get(*row).map(|o| format!("Output \"{}\"", o.name)).unwrap_or_default()
            } else {
                pa.inputs.get(*row).map(|i| format!("Input {} \"{}\"", i.no, i.name)).unwrap_or_default()
            }
        };
        let names = |rows: &[usize], output: bool| {
            rows.iter()
                .filter_map(|r| {
                    if output {
                        pa.outputs.get(*r).map(|o| o.name.clone())
                    } else {
                        pa.inputs.get(*r).map(|i| format!("{} {}", i.no, i.name))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            PatchProblem::PortTwice { device, port, output, rows } => {
                format!("{} port {port} has two {}: {}", dev(device), what(*output), names(rows, *output))
            }
            PatchProblem::DeskChannelTwice { device, channel, rows } => {
                format!("{} channel {channel} has two inputs: {}", dev(device), names(rows, false))
            }
            PatchProblem::LineChannelTwice { link, channel, output, rows } => {
                format!("{} channel {channel} carries two signals: {}", line(link), names(rows, *output))
            }
            PatchProblem::NoTarget { row, output } => {
                if *output {
                    format!("{} goes nowhere (no device or port)", name(row, true))
                } else {
                    format!("{} reaches no desk", name(row, false))
                }
            }
            PatchProblem::PortOutOfRange { device, port, output, row } => {
                let n = pa.device(*device).map(|d| if *output { d.outputs } else { d.inputs }).unwrap_or(0);
                format!("{}: {} has only {n} {} (port {port})", name(row, *output), dev(device), what(*output))
            }
            PatchProblem::DeskChannelOutOfRange { device, channel, row } => {
                let n = pa.device(*device).map(|d| d.inputs).unwrap_or(0);
                format!("{}: {} has only {n} channels (channel {channel})", name(row, false), dev(device))
            }
            PatchProblem::LineFull { link, used } => {
                let n = pa.link(*link).map(|l| l.channels).unwrap_or(0);
                format!("{} carries {used} signals but has only {n} channels", line(link))
            }
            PatchProblem::LineChannelOutOfRange { link, channel, output, row } => {
                let n = pa.link(*link).map(|l| l.channels).unwrap_or(0);
                format!("{}: {} has only {n} channels (channel {channel})", name(row, *output), line(link))
            }
            PatchProblem::NoPhantom { row } => format!("{}: condenser mic without phantom power (48V)", name(row, false)),
        }
    }

    /// Input row (or output row) the problem points at.
    pub fn place(&self) -> (bool, Option<usize>) {
        match self {
            PatchProblem::PortTwice { output, rows, .. } | PatchProblem::LineChannelTwice { output, rows, .. } => {
                (*output, rows.get(1).copied())
            }
            PatchProblem::DeskChannelTwice { rows, .. } => (false, rows.get(1).copied()),
            PatchProblem::NoTarget { row, output }
            | PatchProblem::PortOutOfRange { row, output, .. }
            | PatchProblem::LineChannelOutOfRange { row, output, .. } => (*output, Some(*row)),
            PatchProblem::DeskChannelOutOfRange { row, .. } | PatchProblem::NoPhantom { row } => (false, Some(*row)),
            PatchProblem::LineFull { .. } => (false, None),
        }
    }

    /// The rows marked in the tables: (output?, row).
    pub fn rows(&self) -> Vec<(bool, usize)> {
        match self {
            PatchProblem::PortTwice { output, rows, .. } | PatchProblem::LineChannelTwice { output, rows, .. } => {
                rows.iter().map(|r| (*output, *r)).collect()
            }
            PatchProblem::DeskChannelTwice { rows, .. } => rows.iter().map(|r| (false, *r)).collect(),
            _ => match self.place() {
                (o, Some(r)) => vec![(o, r)],
                _ => Vec::new(),
            },
        }
    }
}

/// Every problem in the patch.
pub fn check(p: &Project) -> Vec<PatchProblem> {
    let pa = &p.patch;
    let mut out = Vec::new();
    let mut ports: BTreeMap<(Id, u32, bool), Vec<usize>> = BTreeMap::new();
    let mut desk: BTreeMap<(Id, u32), Vec<usize>> = BTreeMap::new();
    let mut line: BTreeMap<(Id, u32, bool), Vec<usize>> = BTreeMap::new();
    let mut used: BTreeMap<Id, std::collections::BTreeSet<(bool, u32)>> = BTreeMap::new();
    for (row, i) in pa.inputs.iter().enumerate() {
        if let (Some(d), Some(port)) = (i.stagebox, i.port) {
            ports.entry((d, port, false)).or_default().push(row);
            if pa.device(d).is_some_and(|x| port == 0 || port > x.inputs) {
                out.push(PatchProblem::PortOutOfRange { device: d, port, output: false, row });
            }
        }
        for f in &i.feeds {
            desk.entry((f.device, f.channel)).or_default().push(row);
            if pa.device(f.device).is_some_and(|x| f.channel == 0 || f.channel > x.inputs) {
                out.push(PatchProblem::DeskChannelOutOfRange { device: f.device, channel: f.channel, row });
            }
        }
        if let Some((l, Some(ch))) = pa.input_line(i) {
            // On an analog multicore the channel is the port: a clash there
            // is already a port clash.
            if i.link_ch.is_some() || pa.link(l).is_some_and(|x| !x.kind.is_analog()) {
                line.entry((l, ch, false)).or_default().push(row);
            }
            used.entry(l).or_default().insert((false, ch));
            if pa.link(l).is_some_and(|x| ch == 0 || ch > x.channels) {
                out.push(PatchProblem::LineChannelOutOfRange { link: l, channel: ch, output: false, row });
            }
        }
        if i.feeds.is_empty() && !(i.name.trim().is_empty() && i.source.trim().is_empty()) {
            out.push(PatchProblem::NoTarget { row, output: false });
        }
        if needs_phantom(&i.mic) && !i.phantom {
            out.push(PatchProblem::NoPhantom { row });
        }
    }
    for (row, o) in pa.outputs.iter().enumerate() {
        if let (Some(d), Some(port)) = (o.device, o.port) {
            ports.entry((d, port, true)).or_default().push(row);
            if pa.device(d).is_some_and(|x| port == 0 || port > x.outputs) {
                out.push(PatchProblem::PortOutOfRange { device: d, port, output: true, row });
            }
        }
        if let Some((l, Some(ch))) = pa.output_line(o) {
            // On an analog multicore the channel is the port: a clash there
            // is already a port clash.
            if o.link_ch.is_some() || pa.link(l).is_some_and(|x| !x.kind.is_analog()) {
                line.entry((l, ch, true)).or_default().push(row);
            }
            used.entry(l).or_default().insert((true, ch));
            if pa.link(l).is_some_and(|x| ch == 0 || ch > x.channels) {
                out.push(PatchProblem::LineChannelOutOfRange { link: l, channel: ch, output: true, row });
            }
        }
        if o.device.is_none() && o.target.trim().is_empty() && !o.name.trim().is_empty() {
            out.push(PatchProblem::NoTarget { row, output: true });
        }
    }
    for ((device, port, output), rows) in ports {
        if rows.len() > 1 {
            out.push(PatchProblem::PortTwice { device, port, output, rows });
        }
    }
    for ((device, channel), rows) in desk {
        if rows.len() > 1 {
            out.push(PatchProblem::DeskChannelTwice { device, channel, rows });
        }
    }
    for ((link, channel, output), rows) in line {
        // An analog multicore has separate send and return pairs; a
        // network line carries both ways too: one direction at a time.
        if rows.len() > 1 {
            out.push(PatchProblem::LineChannelTwice { link, channel, output, rows });
        }
    }
    for (link, chans) in used {
        let Some(l) = pa.link(link) else { continue };
        let sends = chans.iter().filter(|(o, _)| !o).count() as u32;
        let returns = chans.iter().filter(|(o, _)| *o).count() as u32;
        let n = sends.max(returns);
        if l.channels > 0 && n > l.channels {
            out.push(PatchProblem::LineFull { link, used: n });
        }
    }
    out
}

/// One line of the cable and stand list.
#[derive(Clone, Debug, PartialEq)]
pub struct PackItem {
    pub group: &'static str,
    pub what: String,
    pub count: u32,
    pub notes: String,
}

/// Cables, DI boxes, stands and lines, counted from the patch.
pub fn pack_list(p: &Project) -> Vec<PackItem> {
    let pa = &p.patch;
    let mut cables: BTreeMap<String, u32> = BTreeMap::new();
    let mut stands: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut mics: BTreeMap<String, u32> = BTreeMap::new();
    let mut di = 0;
    let connector = |d: Option<Id>| {
        d.and_then(|x| pa.device(x)).map(|x| x.connector.label()).unwrap_or("XLR").to_string()
    };
    for i in &pa.inputs {
        if i.name.trim().is_empty() && i.source.trim().is_empty() && i.mic.trim().is_empty() {
            continue;
        }
        let c = if i.cable.trim().is_empty() { connector(i.stagebox) } else { i.cable.trim().to_string() };
        *cables.entry(c).or_default() += 1;
        if is_di(&i.mic) {
            di += 1;
            // The instrument cable into the DI.
            *cables.entry("Jack (instrument)".into()).or_default() += 1;
        } else if !i.mic.trim().is_empty() {
            *mics.entry(i.mic.trim().to_string()).or_default() += 1;
        }
        if i.stand != Stand::None {
            *stands.entry(i.stand.long()).or_default() += 1;
        }
    }
    for o in &pa.outputs {
        if o.name.trim().is_empty() || o.dest == Destination::Iem {
            continue;
        }
        let c = if o.cable.trim().is_empty() { connector(o.device) } else { o.cable.trim().to_string() };
        *cables.entry(c).or_default() += 1;
    }
    let mut out = Vec::new();
    for l in &pa.links {
        let from = device_label(p, l.from);
        let to = device_label(p, l.to);
        let mut notes = vec![format!("{} ch", l.channels)];
        if !from.is_empty() || !to.is_empty() {
            notes.push(format!("{from} to {to}"));
        }
        if !l.length.is_empty() {
            notes.push(l.length.clone());
        }
        let kind = l.kind.label();
        let what = if l.name.to_ascii_lowercase().contains(&kind.to_ascii_lowercase()) { l.name.clone() } else { format!("{kind} {}", l.name) };
        out.push(PackItem { group: "Lines", what, count: 1, notes: notes.join(", ") });
    }
    for (what, count) in cables {
        out.push(PackItem { group: "Cables", what, count, notes: String::new() });
    }
    if di > 0 {
        out.push(PackItem { group: "DI boxes", what: "DI box".into(), count: di, notes: String::new() });
    }
    for (what, count) in mics {
        out.push(PackItem { group: "Mics", what, count, notes: String::new() });
    }
    for (what, count) in stands {
        out.push(PackItem { group: "Stands", what: what.into(), count, notes: String::new() });
    }
    out
}

/// How many inputs and outputs of a device are in use.
pub fn device_use(p: &Project, id: Id) -> (usize, usize) {
    let pa = &p.patch;
    let ins = pa.inputs.iter().filter(|i| i.stagebox == Some(id) || i.feed(id).is_some()).count();
    let outs = pa.outputs.iter().filter(|o| o.device == Some(id)).count();
    (ins, outs)
}

/// How many channels of a line are in use (sends, returns).
pub fn line_use(p: &Project, id: Id) -> (usize, usize) {
    let pa = &p.patch;
    let ins = pa.inputs.iter().filter(|i| pa.input_line(i).is_some_and(|(l, _)| l == id)).count();
    let outs = pa.outputs.iter().filter(|o| pa.output_line(o).is_some_and(|(l, _)| l == id)).count();
    (ins, outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Location;

    fn setup() -> Project {
        let mut p = Project::default();
        let stage = p.alloc_id();
        p.locations.push(Location { id: stage, name: "Stage left".into(), ..Location::default() });
        let dev = |p: &mut Project, name: &str, kind, ins, outs| {
            let id = p.alloc_id();
            p.patch.devices.push(Device { id, name: name.into(), kind, location: Some(stage), inputs: ins, outputs: outs, ..Device::default() });
            id
        };
        let sb = dev(&mut p, "SB1", DeviceKind::Stagebox, 4, 2);
        let foh = dev(&mut p, "FOH", DeviceKind::Console, 8, 8);
        let mon = dev(&mut p, "MON", DeviceKind::Console, 8, 8);
        let id = p.alloc_id();
        p.patch.links.push(Link { id, name: "MC1".into(), kind: LinkKind::Multicore, from: Some(sb), to: Some(foh), channels: 4, ..Link::default() });
        for (k, (name, mic)) in [("Kick", "Beta 91"), ("Snare", "SM57"), ("Bass", "DI")].iter().enumerate() {
            let id = p.alloc_id();
            let n = k as u32 + 1;
            p.patch.inputs.push(Input {
                id,
                no: n,
                name: (*name).into(),
                mic: (*mic).into(),
                stagebox: Some(sb),
                port: Some(n),
                feeds: vec![Feed { device: foh, channel: n }, Feed { device: mon, channel: n }],
                stand: if k == 1 { Stand::Clamp } else { Stand::None },
                phantom: k == 0,
                ..Input::default()
            });
        }
        p
    }

    #[test]
    fn signal_path_with_a_split() {
        let p = setup();
        let path = input_path(&p, &p.patch.inputs[1]);
        assert_eq!(path, ["Snare (SM57)", "SB1 in 2 (Stage left)", "MC1 ch 2", "FOH 2 / MON 2"]);
        assert!(check(&p).is_empty(), "{:?}", check(&p));
    }

    #[test]
    fn checks_find_clashes_and_overflows() {
        let mut p = setup();
        // Snare onto the kick's port, desk channel and line channel.
        p.patch.inputs[1].port = Some(1);
        let foh = p.patch.desks()[0];
        p.patch.inputs[2].set_feed(foh, Some(9));
        p.patch.inputs[0].phantom = false;
        let id = p.alloc_id();
        p.patch.inputs.push(Input { id, no: 4, name: "Spare".into(), ..Input::default() });
        let probs = check(&p);
        let msgs: Vec<String> = probs.iter().map(|x| x.message(&p)).collect();
        assert!(probs.iter().any(|x| matches!(x, PatchProblem::PortTwice { port: 1, .. })), "{msgs:#?}");
        // The multicore channel follows the port: no second message.
        assert!(!probs.iter().any(|x| matches!(x, PatchProblem::LineChannelTwice { .. })), "{msgs:#?}");
        assert!(probs.iter().any(|x| matches!(x, PatchProblem::DeskChannelOutOfRange { channel: 9, .. })), "{msgs:#?}");
        assert!(probs.iter().any(|x| matches!(x, PatchProblem::NoPhantom { row: 0 })), "{msgs:#?}");
        assert!(probs.iter().any(|x| matches!(x, PatchProblem::NoTarget { row: 3, output: false })), "{msgs:#?}");
        assert!(msgs.iter().any(|m| m.contains("FOH has only 8 channels")), "{msgs:#?}");
    }

    #[test]
    fn a_full_line() {
        let mut p = setup();
        p.patch.links[0].channels = 2;
        let probs = check(&p);
        assert!(probs.iter().any(|x| matches!(x, PatchProblem::LineFull { used: 3, .. })), "{probs:?}");
    }

    #[test]
    fn counting_cables_and_stands() {
        let p = setup();
        let list = pack_list(&p);
        let get = |what: &str| list.iter().find(|x| x.what == what).map(|x| x.count);
        assert_eq!(get("XLR"), Some(3));
        assert_eq!(get("DI box"), Some(1));
        assert_eq!(get("Jack (instrument)"), Some(1));
        assert_eq!(get("Clamp"), Some(1));
        assert_eq!(get("Multicore MC1"), Some(1));
        assert_eq!(get("SM57"), Some(1));
    }

    #[test]
    fn deleting_a_device_clears_references() {
        let mut p = setup();
        let sb = p.patch.devices[0].id;
        p.patch.devices.retain(|d| d.id != sb);
        p.prune();
        assert!(p.patch.inputs.iter().all(|i| i.stagebox.is_none()));
        assert_eq!(p.patch.links[0].from, None);
    }

    #[test]
    fn parsing_words() {
        assert_eq!(DeviceKind::parse("desk"), Some(DeviceKind::Console));
        assert_eq!(LinkKind::parse("aes-50"), Some(LinkKind::Aes50));
        assert_eq!(LinkKind::parse("dan"), Some(LinkKind::Dante));
        assert_eq!(Stand::parse("tall"), Some(Stand::Tall));
        assert_eq!(Destination::parse("wedge"), Some(Destination::Monitor));
        assert!(needs_phantom("Neumann KM184"));
        assert!(!needs_phantom("SM58"));
        assert!(is_di("DI"));
        assert!(is_di("Active DI"));
        assert!(!is_di("Dixon"));
    }
}
