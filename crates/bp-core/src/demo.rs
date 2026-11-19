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
}
