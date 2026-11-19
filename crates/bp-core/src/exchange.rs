//! The Showtime exchange format: what the live tools of the suite (Show
//! Master and others) need from a plan, as one versioned JSON file.
//! Blueprint writes it; anything can read it without knowing Blueprint's
//! project file. See `docs/blueprint/exchange-format.md`.
//!
//! Rules: `format` is always "showtime-exchange"; `version` grows when a
//! field changes its meaning (readers refuse newer major versions); new
//! fields may appear at any time (readers ignore what they don't know);
//! everything is plain text and numbers, no Blueprint ids.

use crate::mic::{dca_channels, tx_label, MicView};
use crate::model::Project;
use crate::patch::{device_label, link_label};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "showtime-exchange";
pub const VERSION: u32 = 1;
/// The file extension.
pub const EXTENSION: &str = "showtime.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Exchange {
    pub format: String,
    pub version: u32,
    /// The program that wrote it ("Blueprint 0.1.0").
    pub generator: String,
    pub production: Production,
    /// Input channels of the patch, in channel order.
    pub inputs: Vec<InputChannel>,
    pub outputs: Vec<OutputChannel>,
    /// The mic plan per scene: who is on which transmitter, DCAs.
    pub scenes: Vec<Scene>,
    /// The radio mics (transmitters) with their desk channels.
    pub transmitters: Vec<RadioMic>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Production {
    pub name: String,
    pub company: String,
    pub venue: String,
    pub dates: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct InputChannel {
    /// The input number of the patch.
    pub channel: u32,
    pub name: String,
    pub source: String,
    pub mic: String,
    pub phantom: bool,
    /// One of red, orange, yellow, green, blue, purple, or "".
    pub color: String,
    pub performer: String,
    pub stagebox: String,
    pub port: Option<u32>,
    pub line: String,
    pub line_channel: Option<u32>,
    /// The channel on each desk or recorder, by its name ("FOH": 1).
    pub desks: Vec<DeskChannel>,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct DeskChannel {
    pub desk: String,
    pub channel: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct OutputChannel {
    pub name: String,
    pub desk: String,
    /// Bus, mix or matrix on the desk.
    pub bus: String,
    pub device: String,
    pub port: Option<u32>,
    /// "pa", "monitor", "iem", "recording", "broadcast" or "other".
    pub destination: String,
    pub performer: String,
    pub mix_notes: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct RadioMic {
    /// The desk / receiver channel as written in the plan ("12").
    pub channel: String,
    pub name: String,
    pub model: String,
    pub frequency_mhz: Option<f64>,
    /// "active", "spare" or "defect".
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Scene {
    pub number: String,
    pub title: String,
    /// Who wears each transmitter in the scene (channel names for the desk).
    pub mics: Vec<SceneMic>,
    pub dcas: Vec<SceneDca>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct SceneMic {
    /// The transmitter's channel (as [`RadioMic::channel`]).
    pub channel: String,
    pub role: String,
    pub performer: String,
    /// The role's second (B) mic.
    pub b: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct SceneDca {
    /// 1-based.
    pub dca: u32,
    pub label: String,
    /// Transmitter channels on the DCA.
    pub channels: Vec<String>,
}

/// The project as exchange data.
pub fn export(p: &Project, generator: &str) -> Exchange {
    let pa = &p.patch;
    let desks = pa.desks();
    let person = |id: Option<crate::model::Id>| id.and_then(|i| p.person(i)).map(|x| x.name.clone()).unwrap_or_default();
    let mut inputs: Vec<InputChannel> = pa
        .inputs
        .iter()
        .map(|i| {
            let line = pa.input_line(i);
            InputChannel {
                channel: i.no,
                name: i.name.clone(),
                source: i.source.clone(),
                mic: i.mic.clone(),
                phantom: i.phantom,
                color: i.color.clone(),
                performer: person(i.performer),
                stagebox: device_label(p, i.stagebox),
                port: i.port,
                line: line.map(|(l, _)| link_label(p, Some(l))).unwrap_or_default(),
                line_channel: line.and_then(|(_, c)| c),
                desks: desks.iter().filter_map(|d| i.feed(*d).map(|channel| DeskChannel { desk: device_label(p, Some(*d)), channel })).collect(),
                notes: i.notes.clone(),
            }
        })
        .collect();
    inputs.sort_by_key(|i| i.channel);
    let outputs = pa
        .outputs
        .iter()
        .map(|o| OutputChannel {
            name: o.name.clone(),
            desk: device_label(p, o.desk),
            bus: o.bus.clone(),
            device: device_label(p, o.device),
            port: o.port,
            destination: format!("{:?}", o.dest).to_lowercase(),
            performer: person(o.performer),
            mix_notes: o.mix_notes.clone(),
        })
        .collect();
    let transmitters = p
        .transmitters
        .iter()
        .map(|t| RadioMic {
            channel: tx_label(p, t.id),
            name: t.name.clone(),
            model: t.model.clone(),
            frequency_mhz: t.frequency,
            status: format!("{:?}", t.status).to_lowercase(),
        })
        .collect();
    let view = MicView::build(p);
    let role = |id| p.role(id).map(|r| r.name.clone()).unwrap_or_default();
    let scenes = p
        .scenes
        .iter()
        .enumerate()
        .map(|(si, s)| {
            let mut mics = Vec::new();
            for (ci, tx) in view.txs.iter().enumerate() {
                for w in view.cells.get(si).and_then(|r| r.get(ci)).into_iter().flatten() {
                    mics.push(SceneMic { channel: tx_label(p, *tx), role: role(w.role), performer: person(w.person), b: w.b });
                }
            }
            let mut dcas: Vec<SceneDca> = p
                .mic
                .dca
                .iter()
                .filter(|d| d.scene == s.id)
                .map(|d| SceneDca { dca: d.dca, label: d.label.clone(), channels: dca_channels(p, s.id, d) })
                .collect();
            dcas.sort_by_key(|d| d.dca);
            Scene { number: s.number.clone(), title: s.title.clone(), mics, dcas }
        })
        .collect();
    Exchange {
        format: FORMAT.into(),
        version: VERSION,
        generator: generator.into(),
        production: Production {
            name: p.meta.production.clone(),
            company: p.meta.company.clone(),
            venue: p.meta.venue.clone(),
            dates: p.meta.dates.clone(),
        },
        inputs,
        outputs,
        scenes,
        transmitters,
    }
}

pub fn to_json(x: &Exchange) -> Result<String, String> {
    serde_json::to_string_pretty(x).map_err(|e| e.to_string())
}

/// Read an exchange file (any tool's), refusing other formats and newer
/// versions.
pub fn read(json: &str) -> Result<Exchange, String> {
    let x: Exchange = serde_json::from_str(json).map_err(|e| format!("not an exchange file: {e}"))?;
    if x.format != FORMAT {
        return Err(format!("not an exchange file (format \"{}\")", x.format));
    }
    if x.version > VERSION {
        return Err(format!("written by a newer program (version {}); update Blueprint", x.version));
    }
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_band_patch_goes_out_and_back() {
        let p = crate::demo::band();
        let x = export(&p, "Blueprint test");
        assert_eq!(x.inputs.len(), 26);
        let kick = &x.inputs[0];
        assert_eq!((kick.channel, kick.name.as_str(), kick.stagebox.as_str(), kick.port), (1, "Kick in", "SB1", Some(1)));
        assert_eq!(kick.line, "MC1");
        assert!(kick.desks.iter().any(|d| d.desk == "MON"));
        assert!(x.outputs.iter().any(|o| o.destination == "iem" && o.performer == "Ida Lorenz"));
        let json = to_json(&x).unwrap();
        assert!(json.contains("\"format\": \"showtime-exchange\""));
        assert_eq!(read(&json).unwrap(), x);
        // Unknown fields are fine, newer versions and other formats are not.
        assert!(read(&json.replacen("\"version\": 1", "\"version\": 1, \"later\": true", 1)).is_ok());
        assert!(read(&json.replacen("\"version\": 1", "\"version\": 2", 1)).is_err());
        assert!(read("{\"format\": \"other\"}").is_err());
    }

    #[test]
    fn the_musical_mic_plan_per_scene() {
        let p = crate::demo::musical();
        let x = export(&p, "Blueprint test");
        assert_eq!(x.scenes.len(), p.scenes.len());
        assert!(!x.transmitters.is_empty());
        let first = &x.scenes[0];
        assert!(!first.mics.is_empty());
        assert!(first.mics.iter().all(|m| !m.role.is_empty() && !m.channel.is_empty()));
        assert!(x.scenes.iter().any(|s| !s.dcas.is_empty()), "the demo programs DCAs");
    }
}
