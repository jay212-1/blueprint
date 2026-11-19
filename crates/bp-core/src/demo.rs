//! Demo projects: for trying Blueprint out and as test fixtures.

use crate::mic::{CastVariant, DcaAssign, VariantCast};
use crate::model::{
    Capsule, CapsuleKind, CastKind, Casting, Id, Meta, Person, Project, Role, Scene, Transmitter,
    TxStatus,
};

/// A small musical ("The Lighthouse Keeper", made up): 12 performers, 2 acts,
/// 15 scenes, 16 transmitters with 2 spares, several swaps (one of them too
/// tight), A/B miking for one role, double casting and a swing show.
pub fn musical() -> Project {
    let mut p = Project {
        meta: Meta {
            production: "The Lighthouse Keeper".into(),
            company: "Harbour Theatre Company".into(),
            venue: "Stadttheater, main stage".into(),
            dates: "Premiere 12 March 2027".into(),
            notes: "Demo project".into(),
        },
        ..Project::default()
    };

    let people = [
        ("Anna Berg", "AB", "red", "Allergic to medical tape: use skin glue"),
        ("Jonas Keller", "JK", "blue", ""),
        ("Felix Hartmann", "FH", "green", "Wears a beard piece"),
        ("Lea Sommer", "LS", "purple", "Wig: mic in the wig line"),
        ("Paul Brandt", "PB", "orange", ""),
        ("Sophie Weber", "SW", "yellow", ""),
        ("Mia Fischer", "MF", "red", "Alternate for Mara"),
        ("Tim Schulz", "TS", "", ""),
        ("Nina Wolf", "NW", "", ""),
        ("Ben Krüger", "BK", "", ""),
        ("Clara Neumann", "CN", "", ""),
        ("Lukas Braun", "LB", "", "Swing"),
    ];
    let mut pid = Vec::new();
    for (name, short, color, notes) in people {
        let id = p.alloc_id();
        p.cast.push(Person {
            id,
            name: name.into(),
            short: short.into(),
            color: color.into(),
            notes: notes.into(),
            ..Person::default()
        });
        pid.push(id);
    }

    // Role, lead (index into people), others.
    type RoleRow<'a> = (&'a str, usize, &'a [(usize, CastKind)]);
    let roles: [RoleRow; 12] = [
        ("Mara", 0, &[(6, CastKind::Alternate)]),
        ("Tobias", 1, &[(11, CastKind::Swing)]),
        ("Captain Wren", 2, &[]),
        ("Edda", 3, &[]),
        ("Old Fisher", 4, &[]),
        ("Lena", 5, &[]),
        ("Fisherman 1", 7, &[(11, CastKind::Swing)]),
        ("Fisherman 2", 9, &[]),
        ("Townswoman 1", 8, &[]),
        ("Townswoman 2", 10, &[]),
        ("Child", 6, &[]),
        ("Sailor", 11, &[]),
    ];
    let mut rid = Vec::new();
    for (name, lead, others) in roles {
        let id = p.alloc_id();
        let mut casting = vec![Casting { person: pid[lead], kind: CastKind::Lead }];
        casting.extend(others.iter().map(|(i, k)| Casting { person: pid[*i], kind: *k }));
        p.roles.push(Role { id, name: name.into(), casting, ..Role::default() });
        rid.push(id);
    }

    // Transmitters 1–14 and two spares, band G57.
    let mut tid = Vec::new();
    for i in 0..16 {
        let id = p.alloc_id();
        let spare = i >= 14;
        p.transmitters.push(Transmitter {
            id,
            channel: if spare { format!("S{}", i - 13) } else { (i + 1).to_string() },
            name: if spare { "Spare".into() } else { String::new() },
            model: "Bodypack TX".into(),
            band: if i < 8 { "G57".into() } else { "H68".into() },
            frequency: Some(if i < 8 { 470.200 + i as f64 * 0.825 } else { 520.150 + (i - 8) as f64 * 0.775 }),
            battery: "Li-ion pack".into(),
            status: if spare { TxStatus::Spare } else { TxStatus::Active },
            ..Transmitter::default()
        });
        tid.push(id);
    }

    for (i, color) in ["beige", "beige", "cocoa", "beige", "beige", "light", "beige", "cocoa", "beige", "light", "beige", "beige", "beige", "cocoa"]
        .iter()
        .enumerate()
    {
        let id = p.alloc_id();
        p.capsules.push(Capsule {
            id,
            kind: if i == 2 || i == 4 { CapsuleKind::Headset } else { CapsuleKind::Lavalier },
            model: if i == 2 || i == 4 { "Headset omni".into() } else { "Omni lav".into() },
            color: (*color).into(),
            serial: format!("L{:03}", 101 + i),
            spare: i >= 12,
            ..Capsule::default()
        });
    }

    // Scenes: number, title, length (s), roles on stage (indices).
    let scenes: [(&str, &str, f64, &[usize]); 15] = [
        ("1.1", "The Harbour at Dawn", 240.0, &[0, 4, 6, 7, 8, 9, 10]),
        ("1.2", "Nets and Knots", 200.0, &[1, 4, 6, 7]),
        ("1.3", "Letters from the Sea", 260.0, &[0, 3, 9]),
        ("1.4", "The Keeper's Light", 300.0, &[0, 1, 2]),
        ("1.5", "Storm Warning", 220.0, &[2, 4, 5, 6, 8, 10]),
        ("1.6", "Below Deck", 180.0, &[1, 2, 6, 7]),
        ("1.7", "Act One Finale", 330.0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
        ("2.1", "The Morning After", 200.0, &[3, 5, 8, 9]),
        ("2.2", "Wreckage", 240.0, &[1, 2, 6, 11]),
        ("2.3", "Edda's Lament", 210.0, &[3]),
        ("2.4", "The Search", 260.0, &[0, 1, 4, 6, 7, 11]),
        ("2.5", "Lantern Song", 280.0, &[0, 5]),
        ("2.6", "Homecoming", 230.0, &[0, 1, 2, 3, 4, 5]),
        ("2.7", "The Wedding", 250.0, &[0, 1, 3, 5, 8, 9, 11]),
        ("2.8", "Finale", 300.0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11]),
    ];
    let mut t = 0.0;
    let mut sid = Vec::new();
    for (k, (no, title, len, on)) in scenes.iter().enumerate() {
        let id = p.alloc_id();
        // 20 s changes, a 20 minute interval; 1.2 → 1.3 is a quick one.
        if k == 7 {
            t += 1200.0;
        } else if k == 2 {
            t += 25.0;
        } else if k > 0 {
            t += 45.0;
        }
        p.scenes.push(Scene {
            id,
            act: no[..1].into(),
            number: (*no).into(),
            title: (*title).into(),
            page: (k * 6 + 3).to_string(),
            start: Some(t),
            duration: Some(*len),
            roles: on.iter().map(|i| rid[*i]).collect(),
            ..Scene::default()
        });
        sid.push(id);
        t += len;
    }

    // Usual transmitters: 1–10 one per role, Child and Sailor share 11,
    // Fisherman 2 and Townswoman 2 share 8 (a tight swap after 1.2), Captain
    // Wren has a B mic on 12.
    let usual: [(usize, usize); 12] = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7), (8, 8), (9, 7), (10, 10), (11, 10)];
    for (role, tx) in usual {
        p.mic.track_mut(rid[role]).tx = Some(tid[tx]);
    }
    p.mic.track_mut(rid[2]).tx_b = Some(tid[11]);
    p.mic.track_mut(rid[2]).notes = "A/B: second pack on the costume belt".into();
    // Where both are on stage, Townswoman 2 takes transmitter 10.
    p.mic.set_tx_in(sid[0], rid[9], Some(tid[9]));
    p.mic.set_tx_in(sid[6], rid[9], Some(tid[9]));
    p.mic.set_tx_in(sid[13], rid[9], Some(tid[9]));
    p.mic.set_tx_in(sid[14], rid[9], Some(tid[9]));

    for (i, person) in pid.iter().enumerate() {
        let f = p.mic.fitting_mut(*person);
        f.capsule = p.capsules.get(i).map(|c| c.id);
        f.position = ["forehead", "hairline", "cheek", "wig", "cheek", "forehead"].get(i).unwrap_or(&"forehead").to_string();
    }
    p.mic.fitting_mut(pid[2]).capsule_b = Some(p.capsules[13].id);

    let n = p.mic.swap_note_mut(tid[7], sid[2]);
    n.location = "Quick change SL".into();
    n.by = "Mic wrangler SL".into();
    let n = p.mic.swap_note_mut(tid[10], sid[8]);
    n.location = "Dressing room 3".into();
    n.by = "A2".into();
    n.notes = "During the interval".into();

    let cast_b = p.alloc_id();
    p.mic.variants.push(CastVariant {
        id: cast_b,
        name: "Cast B".into(),
        notes: "Mia Fischer plays Mara".into(),
        cast: vec![VariantCast { role: rid[0], person: pid[6] }, VariantCast { role: rid[10], person: pid[11] }],
    });
    let swing = p.alloc_id();
    p.mic.variants.push(CastVariant {
        id: swing,
        name: "Swing show".into(),
        notes: "Lukas Braun on for Tobias, Mia Fischer plays the Sailor".into(),
        cast: vec![VariantCast { role: rid[1], person: pid[11] }, VariantCast { role: rid[11], person: pid[6] }],
    });

    // DCAs: leads on their own, the rest grouped.
    let dca = |p: &mut Project, scene: Id, dca: u32, label: &str, roles: &[usize]| {
        p.mic.dca.push(DcaAssign {
            scene,
            dca,
            label: label.into(),
            roles: roles.iter().map(|i| rid[*i]).collect(),
        });
    };
    for (k, s) in sid.iter().enumerate() {
        let on: Vec<usize> = scenes[k].3.to_vec();
        let leads: Vec<usize> = on.iter().copied().filter(|r| *r < 6).collect();
        let ens: Vec<usize> = on.iter().copied().filter(|r| *r >= 6).collect();
        for (j, r) in leads.iter().take(5).enumerate() {
            dca(&mut p, *s, j as u32 + 1, roles[*r].0, &[*r]);
        }
        if !ens.is_empty() {
            dca(&mut p, *s, 7, "Ensemble", &ens);
        }
    }

    for (i, tx) in tid.iter().enumerate().take(14) {
        let b = p.mic.battery_mut(*tx);
        b.interval = i < 6;
        if i == 2 {
            b.after_scene = Some(sid[4]);
            b.notes = "Rain effect in 1.5: check the pack".into();
        }
    }
    p
}

/// A band show ("Northbound Static", made up): five musicians, a song library
/// of 16 songs and two setlists that share them (a club show in two sets with
/// an encore, and a short festival set).
pub fn band() -> Project {
    use crate::setlist::{Entry, EntryKind, Setlist, Song, Transition};
    let mut p = Project {
        meta: Meta {
            production: "Northbound Static".into(),
            company: "Northbound Static".into(),
            venue: "Kulturhalle".into(),
            dates: "Tour 2027".into(),
            notes: "Demo project".into(),
        },
        ..Project::default()
    };
    for (name, short, notes) in [
        ("Ida Lorenz", "IL", "Lead vocal, acoustic guitar"),
        ("Malte Roth", "MR", "Electric guitar, backing vocal"),
        ("Yusuf Demir", "YD", "Bass"),
        ("Carla Vogt", "CV", "Keys, synth, backing vocal"),
        ("Henrik Jansen", "HJ", "Drums, click and playback"),
    ] {
        let id = p.alloc_id();
        p.cast.push(Person { id, name: name.into(), short: short.into(), notes: notes.into(), ..Person::default() });
    }
    // Title, key, bpm, length (s), lead, line-up, light, sound, colour, notes.
    type SongRow<'a> = (&'a str, &'a str, f64, f64, &'a str, &'a str, &'a str, &'a str, &'a str, &'a str);
    let songs: [SongRow; 16] = [
        ("Paper Satellites", "E", 128.0, 222.0, "Ida", "Full band", "LX 10", "Snap 1", "", "Count-in from playback"),
        ("Low Tide Radio", "A", 96.0, 248.0, "Ida", "Full band", "LX 12", "Snap 2", "", ""),
        ("Glass Harbour", "F#m", 110.0, 265.0, "Ida", "Full band, Carla on synth bass", "LX 14", "Snap 3", "blue", ""),
        ("Northbound", "D", 140.0, 201.0, "Malte", "Full band", "LX 16", "Snap 4", "", "Malte sings lead"),
        ("Small Hours", "C", 72.0, 284.0, "Ida", "Ida and Carla only", "LX 20", "Snap 5", "purple", "Acoustic: band leaves the stage"),
        ("Copper Lines", "G", 118.0, 236.0, "Ida", "Full band", "LX 22", "Snap 6", "", ""),
        ("Weather Report", "Bb", 124.0, 213.0, "Ida", "Full band", "LX 24", "Snap 7", "", ""),
        ("Static Bloom", "E", 132.0, 257.0, "Ida", "Full band, playback", "LX 26", "Snap 8", "orange", "Click on, strings from playback"),
        ("The Long Way North", "Am", 88.0, 312.0, "Ida", "Full band", "LX 30", "Snap 9", "", ""),
        ("Headlights", "B", 150.0, 189.0, "Ida", "Full band", "LX 32", "Snap 10", "", ""),
        ("Ferris Wheel", "Eb", 104.0, 242.0, "Carla", "Full band", "LX 34", "Snap 11", "", "Carla sings lead"),
        ("Signal Fires", "D", 126.0, 230.0, "Ida", "Full band", "LX 36", "Snap 12", "", ""),
        ("Undertow", "C#m", 98.0, 274.0, "Ida", "Full band", "LX 38", "Snap 13", "", ""),
        ("Postcards", "F", 116.0, 205.0, "Ida", "Full band", "LX 40", "Snap 14", "", ""),
        ("Lighthouse Code", "A", 136.0, 228.0, "Ida", "Full band, playback", "LX 44", "Snap 15", "red", "Confetti at the last chorus"),
        ("Goodnight, City", "G", 80.0, 296.0, "Ida", "Full band", "LX 50", "Snap 16", "", "Last song: house lights after"),
    ];
    let mut sid = Vec::new();
    for (title, key, bpm, len, lead, lineup, light, sound, color, notes) in songs {
        let id = p.alloc_id();
        p.songs.push(Song {
            id,
            title: title.into(),
            key: key.into(),
            bpm: Some(bpm),
            duration: Some(len),
            lead: lead.into(),
            lineup: lineup.into(),
            light_cue: light.into(),
            sound_cue: sound.into(),
            color: color.into(),
            notes: notes.into(),
        });
        sid.push(id);
    }
    enum E<'a> {
        S(usize, Transition),
        X(EntryKind, &'a str, f64, &'a str),
    }
    use Transition::{Attacca, Gap, Segue};
    let list = |p: &mut Project, name: &str, venue: &str, date: &str, start: f64, gap: f64, items: &[E]| {
        let id = p.alloc_id();
        let mut entries = Vec::new();
        for it in items {
            let eid = p.alloc_id();
            entries.push(match it {
                E::S(i, t) => Entry { id: eid, kind: EntryKind::Song, song: Some(sid[*i]), transition: *t, ..Entry::default() },
                E::X(k, title, len, notes) => Entry {
                    id: eid,
                    kind: *k,
                    title: (*title).into(),
                    duration: (*len > 0.0).then_some(*len),
                    notes: (*notes).into(),
                    ..Entry::default()
                },
            });
        }
        p.setlists.push(Setlist {
            id,
            name: name.into(),
            band: "Northbound Static".into(),
            venue: venue.into(),
            date: date.into(),
            start: Some(start),
            gap,
            entries,
            ..Setlist::default()
        });
    };
    list(
        &mut p,
        "Club show",
        "Kulturhalle",
        "Fri 16 April 2027",
        20.5 * 3600.0,
        25.0,
        &[
            E::X(EntryKind::Announcement, "Intro from playback", 60.0, "House lights out on the first note"),
            E::S(0, Attacca),
            E::S(1, Gap),
            E::S(2, Gap),
            E::X(EntryKind::Announcement, "Hello, introduce the band", 90.0, ""),
            E::S(3, Gap),
            E::S(5, Segue),
            E::S(6, Gap),
            E::X(EntryKind::Changeover, "Acoustic set-up", 120.0, "Two stools, acoustic guitar DI"),
            E::S(4, Gap),
            E::S(8, Gap),
            E::X(EntryKind::Break, "Interval", 1200.0, ""),
            E::S(7, Attacca),
            E::S(9, Gap),
            E::S(10, Gap),
            E::S(11, Gap),
            E::S(12, Segue),
            E::S(13, Gap),
            E::X(EntryKind::Encore, "Encore", 120.0, "Off and back on"),
            E::S(14, Gap),
            E::S(15, Gap),
        ],
    );
    list(
        &mut p,
        "Festival set",
        "Open-air, second stage",
        "Sat 10 July 2027",
        17.25 * 3600.0,
        15.0,
        &[
            E::S(0, Attacca),
            E::S(3, Gap),
            E::S(1, Gap),
            E::S(7, Gap),
            E::X(EntryKind::Announcement, "Hello festival", 45.0, ""),
            E::S(9, Gap),
            E::S(5, Gap),
            E::S(13, Gap),
            E::S(11, Attacca),
            E::S(12, Gap),
            E::S(14, Gap),
            E::S(15, Gap),
        ],
    );
    band_patch(&mut p);
    p
}

/// The band's patch: two stageboxes (drum riser and stage right), FOH and
/// monitor desks, a recorder, an IEM rack; a multicore and a Dante line; every
/// input split to FOH, monitors and the recording.
fn band_patch(p: &mut Project) {
    use crate::model::Location;
    use crate::patch::{Connector, Destination, Device, DeviceKind, Feed, Input, Link, LinkKind, Output, Stand};
    let loc = |p: &mut Project, name: &str| {
        let id = p.alloc_id();
        p.locations.push(Location { id, name: name.into(), ..Location::default() });
        id
    };
    let riser = loc(p, "Drum riser");
    let right = loc(p, "Stage right");
    let foh_at = loc(p, "FOH");
    let mon_at = loc(p, "Monitor world");
    let dev = |p: &mut Project, name: &str, kind, at, model: &str, ins, outs, connector| {
        let id = p.alloc_id();
        p.patch.devices.push(Device {
            id,
            name: name.into(),
            kind,
            location: Some(at),
            model: model.into(),
            inputs: ins,
            outputs: outs,
            connector,
            ..Device::default()
        });
        id
    };
    let sb1 = dev(p, "SB1", DeviceKind::Stagebox, riser, "Analog stagebox 16/8", 16, 8, Connector::Xlr);
    let sb2 = dev(p, "SB2", DeviceKind::Stagebox, right, "Dante stagebox 32/16", 32, 16, Connector::Xlr);
    let foh = dev(p, "FOH", DeviceKind::Console, foh_at, "Digital console", 64, 32, Connector::Network);
    let mon = dev(p, "MON", DeviceKind::Console, mon_at, "Digital console", 64, 32, Connector::Network);
    let rec = dev(p, "REC", DeviceKind::Recorder, foh_at, "Multitrack recorder", 64, 0, Connector::Network);
    let iem = dev(p, "IEM rack", DeviceKind::Iem, mon_at, "4 stereo IEM transmitters", 4, 4, Connector::Xlr);
    let link = |p: &mut Project, name: &str, kind, from, to, ch, len: &str| {
        let id = p.alloc_id();
        p.patch.links.push(Link { id, name: name.into(), kind, from: Some(from), to: Some(to), channels: ch, length: len.into(), ..Link::default() });
        id
    };
    link(p, "MC1", LinkKind::Multicore, sb1, foh, 16, "40 m");
    let dante = link(p, "Dante A", LinkKind::Dante, sb2, foh, 64, "CAT6 60 m");
    let person = |p: &Project, short: &str| p.cast.iter().find(|x| x.short == short).map(|x| x.id);
    // Name, source, mic, stand, 48V, stagebox (1/2), player, colour.
    type InRow<'a> = (&'a str, &'a str, &'a str, Stand, bool, u8, &'a str, &'a str);
    let inputs: [InRow; 26] = [
        ("Kick in", "Kick", "Beta 91", Stand::Floor, true, 1, "HJ", "red"),
        ("Kick out", "Kick", "D112", Stand::Small, false, 1, "HJ", "red"),
        ("Snare top", "Snare", "SM57", Stand::Clamp, false, 1, "HJ", "red"),
        ("Snare bottom", "Snare", "SM57", Stand::Clamp, false, 1, "HJ", "red"),
        ("Hi-hat", "Hi-hat", "KM184", Stand::Small, true, 1, "HJ", "red"),
        ("Tom 1", "Rack tom", "e604", Stand::Clamp, false, 1, "HJ", "red"),
        ("Tom 2", "Floor tom", "e604", Stand::Clamp, false, 1, "HJ", "red"),
        ("OH L", "Overhead", "C414", Stand::Tall, true, 1, "HJ", "red"),
        ("OH R", "Overhead", "C414", Stand::Tall, true, 1, "HJ", "red"),
        ("Bass DI", "Bass", "DI", Stand::None, false, 1, "YD", "blue"),
        ("Bass mic", "Bass cab", "MD421", Stand::Small, false, 1, "YD", "blue"),
        ("Gtr 1", "Electric guitar cab", "e906", Stand::None, false, 2, "MR", "green"),
        ("Gtr 2", "Electric guitar cab", "SM57", Stand::Small, false, 2, "MR", "green"),
        ("Ac gtr", "Acoustic guitar", "DI", Stand::None, false, 2, "IL", "yellow"),
        ("Keys L", "Stage piano", "DI", Stand::None, false, 2, "CV", "purple"),
        ("Keys R", "Stage piano", "DI", Stand::None, false, 2, "CV", "purple"),
        ("Synth L", "Synth", "DI", Stand::None, false, 2, "CV", "purple"),
        ("Synth R", "Synth", "DI", Stand::None, false, 2, "CV", "purple"),
        ("PB L", "Playback", "DI", Stand::None, false, 1, "HJ", "orange"),
        ("PB R", "Playback", "DI", Stand::None, false, 1, "HJ", "orange"),
        ("Click", "Click (monitors only)", "DI", Stand::None, false, 1, "HJ", "orange"),
        ("Vox Ida", "Lead vocal", "SM58", Stand::Tall, false, 2, "IL", "yellow"),
        ("BV Malte", "Backing vocal", "SM58", Stand::Tall, false, 2, "MR", "green"),
        ("BV Carla", "Backing vocal", "SM58", Stand::Tall, false, 2, "CV", "purple"),
        ("Spare vox", "Spare vocal", "SM58", Stand::Tall, false, 2, "", ""),
        ("Talkback", "MON talkback", "SM58", Stand::None, false, 2, "", ""),
    ];
    let (mut p1, mut p2) = (0, 0);
    for (k, (name, source, mic, stand, phantom, sb, who, color)) in inputs.iter().enumerate() {
        let id = p.alloc_id();
        let no = k as u32 + 1;
        let (stagebox, port, link_ch) = if *sb == 1 {
            p1 += 1;
            (sb1, p1, None)
        } else {
            p2 += 1;
            (sb2, p2, Some(p2))
        };
        let mut feeds = vec![Feed { device: foh, channel: no }, Feed { device: mon, channel: no }];
        if *name != "Click" && *name != "Talkback" {
            feeds.push(Feed { device: rec, channel: no });
        }
        if *name == "Click" {
            feeds.retain(|f| f.device == mon);
        }
        let performer = person(p, who);
        p.patch.inputs.push(Input {
            id,
            no,
            name: (*name).into(),
            source: (*source).into(),
            mic: (*mic).into(),
            stand: *stand,
            phantom: *phantom,
            performer,
            stagebox: Some(stagebox),
            port: Some(port),
            link: (*sb == 2).then_some(dante),
            link_ch,
            feeds,
            notes: match *name {
                "Kick in" => "Gate".into(),
                "Vox Ida" => "Insert: compressor".into(),
                "Talkback" => "Switched mic at monitor world".into(),
                _ => String::new(),
            },
            color: (*color).into(),
            ..Input::default()
        });
    }
    // The talkback mic is plugged at monitor world, not on stage.
    if let Some(t) = p.patch.inputs.last_mut() {
        t.stagebox = None;
        t.port = None;
        t.link = None;
        t.link_ch = None;
        t.cable = "XLR 5 m".into();
    }

    // Name, desk, bus, device, port, destination, target, for, mix notes.
    type OutRow<'a> = (&'a str, Id, &'a str, Id, u32, Destination, &'a str, &'a str, &'a str);
    let outs: [OutRow; 12] = [
        ("PA L", foh, "Main L", sb2, 1, Destination::Pa, "Main hang left", "", ""),
        ("PA R", foh, "Main R", sb2, 2, Destination::Pa, "Main hang right", "", ""),
        ("Subs", foh, "Matrix 1", sb2, 3, Destination::Pa, "Sub array", "", ""),
        ("Front fill", foh, "Matrix 2", sb2, 4, Destination::Pa, "Front fills", "", ""),
        ("Wedge Malte", mon, "Mix 1", sb2, 5, Destination::Monitor, "Wedge DS left", "MR", "Guitars, vocal, some kick"),
        ("Wedge Yusuf", mon, "Mix 2", sb1, 1, Destination::Monitor, "Wedge by the bass amp", "YD", "Kick, snare, click"),
        ("Drum fill", mon, "Mix 3", sb1, 2, Destination::Monitor, "Drum sub and top", "HJ", "Bass, keys, kick"),
        ("Sidefill", mon, "Mix 4/5", sb2, 6, Destination::Monitor, "Sidefills L and R", "", "Vocals, guitars"),
        ("IEM Ida", mon, "Mix 7/8", iem, 1, Destination::Iem, "IEM 1", "IL", "Lots of vocal, acoustic, no click"),
        ("IEM Carla", mon, "Mix 9/10", iem, 2, Destination::Iem, "IEM 2", "CV", "Keys, vocals, click"),
        ("IEM Henrik", mon, "Mix 11/12", iem, 3, Destination::Iem, "IEM 3", "HJ", "Click loud, bass, playback"),
        ("Broadcast", foh, "Matrix 3/4", rec, 1, Destination::Broadcast, "Stream feed", "", "Mix minus talk"),
    ];
    for (name, desk, bus, device, port, dest, target, who, mix) in outs {
        let id = p.alloc_id();
        let performer = person(p, who);
        p.patch.outputs.push(Output {
            id,
            name: name.into(),
            desk: Some(desk),
            bus: bus.into(),
            device: Some(device),
            port: Some(port),
            // SB2 outputs come back over Dante (returns 1–6), SB1 over the multicore.
            link: (device == sb2).then_some(dante),
            link_ch: (device == sb2).then_some(port),
            dest,
            target: target.into(),
            performer,
            mix_notes: mix.into(),
            ..Output::default()
        });
    }
    // The broadcast feed goes on an AES cable to the truck.
    if let Some(b) = p.patch.outputs.last_mut() {
        b.device = None;
        b.port = None;
        b.cable = "AES 30 m".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mic::{MicView, Problem};

    #[test]
    fn the_musical_has_what_the_demo_promises() {
        let p = musical();
        assert_eq!(p.cast.len(), 12);
        assert_eq!(p.scenes.len(), 15);
        assert_eq!(p.transmitters.len(), 16);
        assert_eq!(p.transmitters.iter().filter(|t| t.status == TxStatus::Spare).count(), 2);
        let v = MicView::build(&p);
        assert!(v.swaps.len() >= 3, "{:?}", v.swaps);
        let msgs: Vec<String> = v.problems.iter().map(|x| x.message(&p)).collect();
        // Every cast is playable.
        for var in p.mic.variants.clone() {
            let mut q = p.clone();
            q.mic.variant = Some(var.id);
            let v = MicView::build(&q);
            let bad: Vec<String> = v.problems.iter().filter(|x| !matches!(x, Problem::SwapTooShort { .. })).map(|x| x.message(&q)).collect();
            assert!(bad.is_empty(), "{}: {bad:#?}", var.name);
        }
        // The one intended problem: the swap after 1.2 is too short.
        assert_eq!(v.problems.len(), 1, "{msgs:#?}");
        assert!(matches!(v.problems[0], Problem::SwapTooShort { .. }));
        // The file survives saving and loading.
        let text = crate::project::to_string(&p).unwrap();
        assert_eq!(crate::project::parse(&text).unwrap(), p);
    }

    #[test]
    fn the_band_show_has_two_setlists_from_one_library() {
        use crate::setlist::timing;
        let p = band();
        assert_eq!(p.songs.len(), 16);
        assert_eq!(p.setlists.len(), 2);
        let club = timing(&p, &p.setlists[0]);
        assert_eq!(club.songs, 16);
        assert_eq!(club.parts.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Set 1", "Set 2", "Encore"]);
        assert_eq!(club.unknown, 0);
        let fest = timing(&p, &p.setlists[1]);
        // A festival slot of about 45 minutes.
        assert!((2400.0..2900.0).contains(&fest.total), "{}", fest.total);
        // The patch is clean, every input is split.
        let probs = crate::patch::check(&p);
        assert!(probs.is_empty(), "{:#?}", probs.iter().map(|x| x.message(&p)).collect::<Vec<_>>());
        assert_eq!(p.patch.inputs.len(), 26);
        assert!(p.patch.inputs.iter().filter(|i| i.feeds.len() == 3).count() >= 20);
        assert!(crate::patch::pack_list(&p).iter().any(|x| x.what == "DI box" && x.count == 9));
        // Shared songs: one library entry used in both.
        assert_eq!(p.song_uses(p.songs[0].id), 2);
        let text = crate::project::to_string(&p).unwrap();
        assert_eq!(crate::project::parse(&text).unwrap(), p);
    }
}
