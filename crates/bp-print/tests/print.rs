//! Every print of the demo musical in every theme lays out; the data holds
//! what the plan says. `BLUEPRINT_SHOTS=<folder>` writes the PDFs and the
//! first pages as PNG.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bp_core::demo::musical;
use bp_print::{build, compile, print, Doc, Options, Paper, Theme};

fn out(name: &str, printed: &bp_print::Printed) {
    let Some(dir) = std::env::var_os("BLUEPRINT_SHOTS").map(std::path::PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join(format!("{name}.pdf")), printed.pdf().unwrap()).unwrap();
    std::fs::write(dir.join(format!("{name}.png")), printed.png(0, 1.6).unwrap()).unwrap();
}

#[test]
fn every_document_in_every_theme() {
    let mut p = musical();
    // The band's patch in the same project, so every print has something.
    p.patch = bp_core::demo::band().patch;
    p.locations = bp_core::demo::band().locations;
    p.cast.extend(bp_core::demo::band().cast);
    for doc in Doc::MIC.into_iter().chain(Doc::PATCH) {
        for theme in Theme::ALL {
            let opts = Options { theme, ..Options::for_doc(doc) };
            let printed = print(&p, doc, &opts).unwrap_or_else(|e| panic!("{doc:?} {theme:?}: {e}"));
            assert!(printed.pages() >= 1);
            assert!(printed.pdf().unwrap().starts_with(b"%PDF"));
            out(&format!("{doc:?}-{theme:?}"), &printed);
        }
    }
}

#[test]
fn mic_plot_data_and_pages() {
    let mut p = musical();
    let opts = Options::for_doc(Doc::MicPlot);
    let d = build(&p, Doc::MicPlot, &opts);
    let blocks = d.get()["blocks"].as_array().unwrap();
    let cols: usize = blocks.iter().map(|b| b["columns"].as_array().unwrap().len()).sum();
    assert_eq!(cols, 16, "all transmitters, spares too");
    let first = &blocks[0]["rows"][0];
    assert_eq!(first["no"], "1.1");
    assert_eq!(first["cells"][0]["text"], "Mara");
    assert_eq!(first["cells"][0]["who"], "AB");
    assert_eq!(d.get()["revision"].as_str().unwrap().split(' ').next(), Some("Draft"));

    // Rev 1, a change, Rev 2: the changed cell is marked.
    let snap = bp_core::mic::MicView::build(&p).snapshot(&p);
    p.mic.revision.issue(snap, "2026-11-01".into());
    let mara = p.roles[0].id;
    let s3 = p.scenes[3].id;
    p.mic.set_tx_in(s3, mara, Some(p.transmitters[12].id));
    let snap = bp_core::mic::MicView::build(&p).snapshot(&p);
    p.mic.revision.issue(snap, "2026-11-02".into());
    let d = build(&p, Doc::MicPlot, &opts);
    assert_eq!(d.get()["revision"], "Rev 2 · 2026-11-02");
    let changed: usize = d.get()["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| b["rows"].as_array().unwrap().iter())
        .flat_map(|r| r["cells"].as_array().unwrap().iter())
        .filter(|c| c["changed"] == true)
        .count();
    assert_eq!(changed, 2, "TX 1 lost Mara, TX 13 got her");
    let printed = compile(Doc::MicPlot, &d).unwrap();
    out("MicPlot-Rev2", &printed);

    // A3 fits all transmitters in one block, A4 portrait needs several pages.
    let a3 = Options { paper: Paper::A3, ..opts };
    assert_eq!(build(&p, Doc::MicPlot, &a3).get()["blocks"].as_array().unwrap().len(), 1);
    let tall = Options { landscape: false, ..opts };
    assert!(build(&p, Doc::MicPlot, &tall).get()["blocks"].as_array().unwrap().len() > 1);
}

#[test]
fn cards_and_swaps() {
    let p = musical();
    let d = build(&p, Doc::MicCards, &Options::for_doc(Doc::MicCards));
    let cards = d.get()["cards"].as_array().unwrap();
    let anna = cards.iter().find(|c| c["name"] == "Anna Berg").unwrap();
    assert_eq!(anna["txs"][0]["tx"], "1");
    assert!(anna["notes"].as_str().unwrap().contains("tape"));
    let felix = cards.iter().find(|c| c["name"] == "Felix Hartmann").unwrap();
    assert_eq!(felix["txs"].as_array().unwrap().len(), 2, "A and B mic");

    let d = build(&p, Doc::SwapList, &Options::for_doc(Doc::SwapList));
    let rows = d.get()["sections"][0]["rows"].as_array().unwrap();
    assert!(rows.iter().any(|r| r["warn"] == true), "the tight swap is marked");
    assert!(rows.iter().any(|r| r["cells"][5] == "Quick change SL"));
}

#[test]
fn setlists_fit_one_page_in_every_theme() {
    let mut p = bp_core::demo::band();
    let ids: Vec<u64> = p.setlists.iter().map(|s| s.id).collect();
    for id in &ids {
        let doc = Doc::Setlist(*id);
        for theme in doc.themes() {
            let opts = Options { theme: *theme, ..Options::for_doc(doc) };
            let printed = print(&p, doc, &opts).unwrap_or_else(|e| panic!("{theme:?}: {e}"));
            assert_eq!(printed.pages(), 1, "{theme:?}");
            assert!(printed.warnings.is_empty(), "{:?}", printed.warnings);
            let fit = printed.fit.expect("the template reports its size");
            assert!(fit >= 6.0, "{theme:?} {fit}");
            let name = &p.setlist(*id).unwrap().name;
            out(&format!("Setlist-{name}-{theme:?}"), &printed);
        }
    }
    // The festival set is shorter, so its type is larger.
    let size = |p: &bp_core::model::Project, id| {
        let doc = Doc::Setlist(id);
        print(p, doc, &Options::for_doc(doc)).unwrap().fit.unwrap()
    };
    assert!(size(&p, ids[1]) > size(&p, ids[0]));

    // Data: numbers, specials, transitions, times of day.
    let d = build(&p, Doc::Setlist(ids[0]), &Options::for_doc(Doc::Setlist(ids[0])));
    let e = d.get()["entries"].as_array().unwrap();
    assert_eq!(e[0]["kind"], "announcement");
    assert_eq!(e[0]["clock"], "20:30");
    assert_eq!(e[1]["no"], "1");
    assert_eq!(e[1]["next"], "→");
    assert_eq!(d.get()["parts"].as_array().unwrap().len(), 3);

    // Far too long for one page: it says so.
    let id = ids[0];
    let more = p.setlist(id).unwrap().entries.clone();
    for _ in 0..6 {
        let mut copy = more.clone();
        for e in &mut copy {
            e.id = p.alloc_id();
        }
        p.setlist_mut(id).unwrap().entries.extend(copy);
    }
    let doc = Doc::Setlist(id);
    let printed = print(&p, doc, &Options { theme: Theme::Technical, ..Options::for_doc(doc) }).unwrap();
    assert!(printed.pages() > 1);
    assert_eq!(printed.warnings.len(), 1, "{:?}", printed.warnings);
}

#[test]
fn a_setlist_with_a_logo() {
    let mut p = bp_core::demo::band();
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><circle cx="20" cy="20" r="18" fill="#1d5fa0"/></svg>"##;
    p.setlists[0].set_logo(svg);
    let doc = Doc::Setlist(p.setlists[0].id);
    for theme in doc.themes() {
        let printed = print(&p, doc, &Options { theme: *theme, ..Options::for_doc(doc) }).unwrap();
        assert_eq!(printed.pages(), 1);
        out(&format!("Setlist-logo-{theme:?}"), &printed);
    }
    // Something that is not a picture is left out instead of failing.
    p.setlists[0].set_logo(b"not an image");
    assert!(print(&p, doc, &Options::for_doc(doc)).is_ok());
}

#[test]
fn patch_prints() {
    let p = bp_core::demo::band();
    for doc in Doc::PATCH {
        let printed = print(&p, doc, &Options::for_doc(doc)).unwrap_or_else(|e| panic!("{doc:?}: {e}"));
        out(&format!("Band-{doc:?}"), &printed);
        if doc == Doc::StageboxPlan {
            // One page per box with something plugged in (SB1, SB2, IEM rack).
            assert_eq!(printed.pages(), 3);
        }
    }
    let d = build(&p, Doc::InputList, &Options::for_doc(Doc::InputList));
    let head = d.get()["head"].as_array().unwrap();
    assert!(head.iter().any(|h| h == "MON") && head.iter().any(|h| h == "REC"));
    let first = &d.get()["sections"][0]["rows"][0]["cells"];
    assert_eq!(first[6], "SB1 / 1");
    assert_eq!(first[7], "MC1 / 1");
    assert_eq!(d.get()["revision"].as_str().unwrap().split(' ').next(), Some("Printed"));
    let m = build(&p, Doc::MonitorList, &Options::for_doc(Doc::MonitorList));
    let titles: Vec<&str> = m.get()["sections"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
    assert!(titles[0].starts_with("Ida Lorenz"), "{titles:?}");
    assert_eq!(titles.last(), Some(&"Shared"));
    let l = build(&p, Doc::Labels, &Options::for_doc(Doc::Labels));
    assert_eq!(l.get()["strips"].as_array().unwrap().len(), 5, "FOH, MON, REC, SB1, SB2");
}

#[test]
fn every_plan_prints_in_every_theme() {
    let p = bp_core::demo::band();
    assert_eq!(p.plans.len(), 6, "one of each kind");
    for plan in &p.plans {
        let doc = Doc::Plan(plan.id);
        for theme in [Theme::Stage, Theme::Technical, Theme::Ink] {
            let opts = Options { theme, ..bp_print::options_for(&p, doc) };
            let printed = print(&p, doc, &opts).unwrap_or_else(|e| panic!("{}: {e}", plan.name));
            assert!(printed.pages() >= 1);
            out(&format!("Plan-{}-{theme:?}", plan.name.replace([' ', '/'], "-")), &printed);
        }
    }
    let stage = &p.plans[0];
    assert!(bp_print::options_for(&p, Doc::Plan(stage.id)).landscape);
    let d = build(&p, Doc::Plan(stage.id), &bp_print::options_for(&p, Doc::Plan(stage.id)));
    assert_eq!(d.get()["kind"], "stage");
    assert!(d.get()["stage"]["shapes"].as_array().unwrap().len() > 50);
    let items = d.get()["stage"]["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["ch"] == "1, 2, 3, 4, 5, 6, 7"), "{items:?}");
    // The guest list: the check box column prints as a box.
    let guests = p.plans.iter().find(|x| x.name == "Guest list").unwrap();
    let d = build(&p, Doc::Plan(guests.id), &bp_print::options_for(&p, Doc::Plan(guests.id)));
    let row = &d.get()["sections"][0]["rows"][0]["cells"];
    assert_eq!(row[4], "\u{1}check:1");
    // Contacts come in groups.
    let c = p.plans.iter().find(|x| x.kind.plan_type() == bp_core::plans::PlanType::Contacts).unwrap();
    let d = build(&p, Doc::Plan(c.id), &bp_print::options_for(&p, Doc::Plan(c.id)));
    let titles: Vec<&str> = d.get()["sections"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["Band", "Crew", "Venue", "Promoter"]);
}

#[test]
fn production_pack() {
    let p = bp_core::demo::band();
    let docs = bp_print::default_package(&p);
    assert!(docs.contains(&Doc::InputList) && !docs.contains(&Doc::Labels));
    assert!(!docs.contains(&Doc::MicPlot), "the band has no mic plan");
    let items: Vec<(Doc, Options)> = docs.iter().map(|d| (*d, bp_print::options_for(&p, *d))).collect();
    let single: usize = items.iter().map(|(d, o)| print(&p, *d, o).unwrap().pages()).sum();
    let pack = bp_print::print_package(&p, &items, &Options::for_doc(Doc::Package)).unwrap();
    assert_eq!(pack.pages(), single + 1, "a one-page cover, then every print");
    assert!(pack.pdf().unwrap().starts_with(b"%PDF"));
    out("Package-band", &pack);
    // The musical has a mic plan.
    let m = musical();
    assert!(bp_print::default_package(&m).contains(&Doc::MicPlot));
    let pack = print(&m, Doc::Package, &Options::for_doc(Doc::Package)).unwrap();
    assert!(pack.pages() > 3);
}
