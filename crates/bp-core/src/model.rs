//! The project: one production with its master data (cast, roles, scenes,
//! transmitters, capsules, locations). Plans refer to these by id only, so a
//! renamed role is renamed everywhere.

use serde::{Deserialize, Serialize};

/// Current project file schema. Raise it with every incompatible change and
/// add a migration step in `project::migrate`.
pub const SCHEMA_VERSION: u32 = 1;

/// Stable id of anything in a project (never reused within a project).
pub type Id = u64;

/// Colour names used for colour coding (same set as the other suite tools).
pub const COLORS: [&str; 7] = ["", "red", "orange", "yellow", "green", "blue", "purple"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Project {
    pub schema_version: u32,
    pub next_id: Id,
    pub meta: Meta,
    pub cast: Vec<Person>,
    pub roles: Vec<Role>,
    pub scenes: Vec<Scene>,
    pub transmitters: Vec<Transmitter>,
    pub capsules: Vec<Capsule>,
    pub locations: Vec<Location>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            next_id: 1,
            meta: Meta::default(),
            cast: Vec::new(),
            roles: Vec::new(),
            scenes: Vec::new(),
            transmitters: Vec::new(),
            capsules: Vec::new(),
            locations: Vec::new(),
        }
    }
}

impl Project {
    /// A fresh id.
    pub fn alloc_id(&mut self) -> Id {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }

    pub fn person(&self, id: Id) -> Option<&Person> {
        self.cast.iter().find(|p| p.id == id)
    }

    /// The person with this name or short name (case-insensitive).
    pub fn person_named(&self, name: &str) -> Option<&Person> {
        let n = name.trim();
        if n.is_empty() {
            return None;
        }
        self.cast
            .iter()
            .find(|p| p.name.trim().eq_ignore_ascii_case(n))
            .or_else(|| {
                self.cast
                    .iter()
                    .find(|p| !p.short.is_empty() && p.short.trim().eq_ignore_ascii_case(n))
            })
    }

    pub fn role(&self, id: Id) -> Option<&Role> {
        self.roles.iter().find(|r| r.id == id)
    }

    pub fn role_named(&self, name: &str) -> Option<&Role> {
        let n = name.trim();
        self.roles
            .iter()
            .find(|r| !n.is_empty() && r.name.trim().eq_ignore_ascii_case(n))
    }

    /// Remove references to people that no longer exist (after deleting cast).
    pub fn prune(&mut self) {
        let people: Vec<Id> = self.cast.iter().map(|p| p.id).collect();
        let roles: Vec<Id> = self.roles.iter().map(|r| r.id).collect();
        for r in &mut self.roles {
            r.casting.retain(|c| people.contains(&c.person));
        }
        for s in &mut self.scenes {
            s.roles.retain(|r| roles.contains(r));
        }
    }
}

/// The production: printed in the head and foot of every plan.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Meta {
    pub production: String,
    pub company: String,
    pub venue: String,
    /// Free text, e.g. "Premiere 14 March 2027, shows until 30 April".
    pub dates: String,
    pub notes: String,
}

/// A performer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Person {
    pub id: Id,
    pub name: String,
    /// Short name for tight plans ("AL").
    pub short: String,
    pub contact: String,
    /// E.g. "allergic to tape", "wears a wig".
    pub notes: String,
    pub color: String,
}

/// How a person plays a role.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CastKind {
    /// First cast.
    #[default]
    Lead,
    /// Shares the role on a schedule (double casting).
    Alternate,
    /// Takes over when the lead is out.
    Cover,
    /// Covers several roles.
    Swing,
}

impl CastKind {
    pub const ALL: [CastKind; 4] = [
        CastKind::Lead,
        CastKind::Alternate,
        CastKind::Cover,
        CastKind::Swing,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CastKind::Lead => "lead",
            CastKind::Alternate => "alternate",
            CastKind::Cover => "cover",
            CastKind::Swing => "swing",
        }
    }

    pub fn parse(s: &str) -> Option<CastKind> {
        let s = s.trim().to_ascii_lowercase();
        CastKind::ALL.into_iter().find(|k| k.label() == s || (s == "alt" && *k == CastKind::Alternate))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(default)]
pub struct Casting {
    pub person: Id,
    pub kind: CastKind,
}

/// A role (character). Several people can play it (double casting, covers).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Role {
    pub id: Id,
    pub name: String,
    pub casting: Vec<Casting>,
    pub notes: String,
    pub color: String,
}

impl Role {
    /// The first lead, else the first person cast.
    pub fn lead(&self) -> Option<Id> {
        self.casting
            .iter()
            .find(|c| c.kind == CastKind::Lead)
            .or_else(|| self.casting.first())
            .map(|c| c.person)
    }
}

/// A scene or musical number.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Scene {
    pub id: Id,
    /// "1", "2" … (free text: "Prologue" works too).
    pub act: String,
    /// "1.3", "5" …
    pub number: String,
    /// Scene or song title.
    pub title: String,
    /// Page in the script.
    pub page: String,
    /// Approximate start in the show, seconds from curtain up.
    pub start: Option<f64>,
    /// Approximate length in seconds.
    pub duration: Option<f64>,
    /// Roles on stage in this scene (filled in the mic plan).
    pub roles: Vec<Id>,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TxStatus {
    #[default]
    Active,
    Spare,
    Defect,
}

impl TxStatus {
    pub const ALL: [TxStatus; 3] = [TxStatus::Active, TxStatus::Spare, TxStatus::Defect];

    pub fn label(self) -> &'static str {
        match self {
            TxStatus::Active => "active",
            TxStatus::Spare => "spare",
            TxStatus::Defect => "defect",
        }
    }
}

/// A wireless transmitter (beltpack).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Transmitter {
    pub id: Id,
    /// Channel on the desk / receiver, e.g. "12".
    pub channel: String,
    pub name: String,
    /// Maker and model, e.g. "Shure ADX1M".
    pub model: String,
    /// Frequency band, e.g. "G57".
    pub band: String,
    /// MHz.
    pub frequency: Option<f64>,
    pub color: String,
    /// Battery or rechargeable pack, e.g. "AA", "SB900B".
    pub battery: String,
    pub status: TxStatus,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleKind {
    #[default]
    Headset,
    Lavalier,
    Handheld,
}

impl CapsuleKind {
    pub const ALL: [CapsuleKind; 3] = [
        CapsuleKind::Headset,
        CapsuleKind::Lavalier,
        CapsuleKind::Handheld,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CapsuleKind::Headset => "headset",
            CapsuleKind::Lavalier => "lavalier",
            CapsuleKind::Handheld => "handheld",
        }
    }
}

/// A microphone capsule.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Capsule {
    pub id: Id,
    pub kind: CapsuleKind,
    pub model: String,
    /// Colour of the capsule (skin tones: "beige", "cocoa" …).
    pub color: String,
    pub serial: String,
    pub notes: String,
}

/// A place where gear stands: "Stage left", "FOH", "Drum riser" …
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Location {
    pub id: Id,
    pub name: String,
    pub notes: String,
    pub color: String,
}
