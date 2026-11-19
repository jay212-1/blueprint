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
    let p = musical();
    for doc in Doc::ALL {
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
