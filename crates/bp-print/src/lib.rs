//! Blueprint printing: a plan becomes JSON data, a Typst template
//! (`templates/*.typ`, kept apart from the code) lays it out, and Typst makes
//! the PDF and the preview pages. Only the fonts shipped in `fonts/` are used
//! (Inter, JetBrains Mono, Source Serif), so a PDF looks the same on every
//! computer and nothing is downloaded.

mod data;

pub use data::{build, Data};

use std::sync::OnceLock;
use typst::foundations::{Dict, IntoValue};
use typst_layout::PagedDocument;
use typst_as_lib::{TypstEngine, TypstTemplateCollection};

/// What can be printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Doc {
    MicPlot,
    MicCards,
    SwapList,
    Frequencies,
    DcaSheet,
    BatteryPlan,
    InputList,
    StageboxPlan,
    OutputList,
    MonitorList,
    PackList,
    Labels,
    /// One setlist (by id).
    Setlist(bp_core::model::Id),
    /// One of the other plans (by id).
    Plan(bp_core::model::Id),
    /// A cover with contents and the chosen prints in one PDF.
    Package,
}

impl Doc {
    /// The prints of the patch.
    pub const PATCH: [Doc; 6] = [
        Doc::InputList,
        Doc::StageboxPlan,
        Doc::OutputList,
        Doc::MonitorList,
        Doc::PackList,
        Doc::Labels,
    ];

    /// The prints of the mic plan.
    pub const MIC: [Doc; 6] = [
        Doc::MicPlot,
        Doc::MicCards,
        Doc::SwapList,
        Doc::Frequencies,
        Doc::DcaSheet,
        Doc::BatteryPlan,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Doc::MicPlot => "Mic plot",
            Doc::MicCards => "Mic cards",
            Doc::SwapList => "Swap list",
            Doc::Frequencies => "Frequency list",
            Doc::DcaSheet => "DCA sheet",
            Doc::BatteryPlan => "Battery plan",
            Doc::InputList => "Input list",
            Doc::StageboxPlan => "Stagebox patch",
            Doc::OutputList => "Output list",
            Doc::MonitorList => "Monitor / IEM list",
            Doc::PackList => "Cables and stands",
            Doc::Labels => "Labels and scribble strips",
            Doc::Setlist(_) => "Setlist",
            Doc::Plan(_) => "Plan",
            Doc::Package => "Production pack",
        }
    }

    /// Landscape unless chosen otherwise.
    pub fn landscape(self) -> bool {
        matches!(self, Doc::MicPlot | Doc::DcaSheet | Doc::InputList | Doc::OutputList)
    }

    fn template(self) -> &'static str {
        match self {
            Doc::MicPlot => "mic-plot.typ",
            Doc::MicCards => "mic-cards.typ",
            Doc::Setlist(_) => "setlist.typ",
            Doc::Labels => "labels.typ",
            Doc::Plan(_) => "plan.typ",
            Doc::Package => "package.typ",
            _ => "table.typ",
        }
    }

    /// The themes offered for this document.
    pub fn themes(self) -> &'static [Theme] {
        match self {
            Doc::Setlist(_) => &[Theme::Stage, Theme::Technical, Theme::Elegant],
            _ => &[Theme::Stage, Theme::Technical, Theme::Ink],
        }
    }

    /// Prints that keep the plan's revision (the mic plan's).
    pub fn has_revision(self) -> bool {
        Doc::MIC.contains(&self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Paper {
    #[default]
    A4,
    A3,
    Letter,
}

impl Paper {
    pub const ALL: [Paper; 3] = [Paper::A4, Paper::A3, Paper::Letter];

    pub fn label(self) -> &'static str {
        match self {
            Paper::A4 => "A4",
            Paper::A3 => "A3",
            Paper::Letter => "Letter",
        }
    }

    fn typst(self) -> &'static str {
        match self {
            Paper::A4 => "a4",
            Paper::A3 => "a3",
            Paper::Letter => "us-letter",
        }
    }

    /// Width and height in cm (portrait).
    pub fn size_cm(self) -> (f64, f64) {
        match self {
            Paper::A4 => (21.0, 29.7),
            Paper::A3 => (29.7, 42.0),
            Paper::Letter => (21.59, 27.94),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Theme {
    /// Big type, high contrast (for the wings).
    Stage,
    /// Compact, every detail.
    #[default]
    Technical,
    /// Black and white, no fills.
    Ink,
    /// Serif and centred (setlists for guests and programmes).
    Elegant,
}

impl Theme {
    pub const ALL: [Theme; 4] = [Theme::Stage, Theme::Technical, Theme::Ink, Theme::Elegant];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Stage => "Stage",
            Theme::Technical => "Technical",
            Theme::Ink => "Save ink",
            Theme::Elegant => "Elegant",
        }
    }

    /// What the theme is for (shown when the pointer rests on it).
    pub fn hint(self, doc: Doc) -> &'static str {
        match (self, doc) {
            (Theme::Stage, Doc::Setlist(_)) => "For the floor: big titles, keys and transitions",
            (Theme::Technical, Doc::Setlist(_)) => "For FOH and the crew: times, tempo, line-up and cues",
            (Theme::Stage, _) => "Big type, high contrast: for the wings and quick changes",
            (Theme::Technical, _) => "Compact, every detail",
            (Theme::Ink, _) => "Black and white, no fills",
            (Theme::Elegant, _) => "Serif and centred: for guests, programmes and the client",
        }
    }

    /// The label used for this document ("Technical" is "FOH" on setlists).
    pub fn label_for(self, doc: Doc) -> &'static str {
        match (self, doc) {
            (Theme::Technical, Doc::Setlist(_)) => "Technical / FOH",
            _ => self.label(),
        }
    }

    fn typst(self) -> &'static str {
        match self {
            Theme::Stage => "stage",
            Theme::Technical => "technical",
            Theme::Ink => "ink",
            Theme::Elegant => "elegant",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub paper: Paper,
    pub landscape: bool,
    pub theme: Theme,
    /// Mark what changed since the revision before.
    pub mark_changes: bool,
}

/// The options a document starts with in this project (stage plots, intercom
/// matrices and wide free tables are landscape).
pub fn options_for(p: &bp_core::model::Project, doc: Doc) -> Options {
    use bp_core::plans::PlanKind;
    let mut o = Options::for_doc(doc);
    if let Doc::Plan(id) = doc {
        o.landscape = match p.plan(id).map(|x| &x.kind) {
            Some(PlanKind::StagePlot(_)) => true,
            Some(PlanKind::Intercom(i)) => i.channels.len() > 4,
            Some(PlanKind::Table(t)) => t.columns.len() > 5,
            Some(PlanKind::Contacts(_)) => true,
            _ => false,
        };
    }
    o
}

impl Options {
    pub fn for_doc(doc: Doc) -> Options {
        Options {
            paper: Paper::A4,
            landscape: doc.landscape(),
            theme: match doc {
                Doc::Setlist(_) | Doc::StageboxPlan => Theme::Stage,
                _ => Theme::Technical,
            },
            mark_changes: true,
        }
    }
}

const TEMPLATES: [(&str, &str); 8] = [
    ("package.typ", include_str!("../templates/package.typ")),
    ("plan.typ", include_str!("../templates/plan.typ")),
    ("labels.typ", include_str!("../templates/labels.typ")),
    ("setlist.typ", include_str!("../templates/setlist.typ")),
    ("common.typ", include_str!("../templates/common.typ")),
    ("mic-plot.typ", include_str!("../templates/mic-plot.typ")),
    ("mic-cards.typ", include_str!("../templates/mic-cards.typ")),
    ("table.typ", include_str!("../templates/table.typ")),
];

const FONTS: [&[u8]; 11] = [
    include_bytes!("../fonts/Inter-Regular.ttf"),
    include_bytes!("../fonts/Inter-Italic.ttf"),
    include_bytes!("../fonts/Inter-SemiBold.ttf"),
    include_bytes!("../fonts/Inter-Bold.ttf"),
    include_bytes!("../fonts/Inter-BoldItalic.ttf"),
    include_bytes!("../fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../fonts/JetBrainsMono-Bold.ttf"),
    include_bytes!("../fonts/SourceSerif4-Regular.ttf"),
    include_bytes!("../fonts/SourceSerif4-It.ttf"),
    include_bytes!("../fonts/SourceSerif4-Semibold.ttf"),
    include_bytes!("../fonts/SourceSerif4-Bold.ttf"),
];

fn engine() -> &'static TypstEngine<TypstTemplateCollection> {
    static ENGINE: OnceLock<TypstEngine<TypstTemplateCollection>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        TypstEngine::builder()
            .fonts(FONTS)
            .with_static_source_file_resolver(TEMPLATES)
            .build()
    })
}

/// A laid out document: its pages for the preview and the PDF.
pub struct Printed {
    doc: PagedDocument,
    pub warnings: Vec<String>,
    /// The type size (pt) a setlist was fitted at.
    pub fit: Option<f64>,
}

impl Printed {
    pub fn pages(&self) -> usize {
        self.doc.pages().len()
    }

    pub fn pdf(&self) -> Result<Vec<u8>, String> {
        typst_pdf::pdf(&self.doc, &typst_pdf::PdfOptions::default())
            .map_err(|e| format!("{e:?}"))
    }

    /// Page `i` as RGBA pixels (`width`, `height`, data) at `pixel_per_pt`.
    pub fn render(&self, i: usize, pixel_per_pt: f32) -> Option<(u32, u32, Vec<u8>)> {
        let page = self.doc.pages().get(i)?;
        let pixmap = typst_render::render(
            page,
            &typst_render::RenderOptions {
                pixel_per_pt: (pixel_per_pt as f64).into(),
                ..Default::default()
            },
        );
        let (w, h) = (pixmap.width(), pixmap.height());
        // tiny-skia keeps premultiplied alpha; pages are opaque, so this is RGBA.
        Some((w, h, pixmap.take()))
    }

    pub fn png(&self, i: usize, pixel_per_pt: f32) -> Option<Vec<u8>> {
        let page = self.doc.pages().get(i)?;
        typst_render::render(
            page,
            &typst_render::RenderOptions {
                pixel_per_pt: (pixel_per_pt as f64).into(),
                ..Default::default()
            },
        )
        .encode_png()
        .ok()
    }
}

/// The prints that have something in them for this project, in the
/// order of a production pack.
pub fn available(p: &bp_core::model::Project) -> Vec<Doc> {
    let mut out = Vec::new();
    if !p.transmitters.is_empty() && !p.scenes.is_empty() {
        out.extend(Doc::MIC);
    }
    if !p.patch.inputs.is_empty() || !p.patch.outputs.is_empty() {
        out.extend(Doc::PATCH);
    }
    out.extend(p.setlists.iter().map(|s| Doc::Setlist(s.id)));
    out.extend(p.plans.iter().map(|x| Doc::Plan(x.id)));
    out
}

/// What a production pack holds unless chosen otherwise: everything but
/// the prints that are cut up (mic cards, labels).
pub fn default_package(p: &bp_core::model::Project) -> Vec<Doc> {
    available(p).into_iter().filter(|d| !matches!(d, Doc::MicCards | Doc::Labels)).collect()
}

/// The name of a print in this project (a setlist's or plan's own name).
pub fn doc_name(p: &bp_core::model::Project, doc: Doc) -> String {
    let named = match doc {
        Doc::Setlist(id) => p.setlist(id).map(|s| s.name.trim().to_string()),
        Doc::Plan(id) => p.plan(id).map(|x| if x.name.trim().is_empty() { x.kind.plan_type().label().to_string() } else { x.name.trim().to_string() }),
        _ => None,
    };
    named.filter(|n| !n.is_empty()).unwrap_or_else(|| doc.title().to_string())
}

/// A production pack: a cover with the contents, then every print with
/// its own options, in one document.
pub fn print_package(p: &bp_core::model::Project, items: &[(Doc, Options)], opts: &Options) -> Result<Printed, String> {
    let mut parts = Vec::new();
    let mut warnings = Vec::new();
    for (doc, o) in items.iter().filter(|(d, _)| *d != Doc::Package) {
        let printed = print(p, *doc, o).map_err(|e| format!("{}: {e}", doc_name(p, *doc)))?;
        warnings.extend(printed.warnings.iter().map(|w| format!("{}: {w}", doc_name(p, *doc))));
        parts.push((*doc, printed));
    }
    // The cover twice: the second time with the page numbers it makes room for.
    let mut cover_pages = 1;
    let mut cover = None;
    for _ in 0..2 {
        let mut page = cover_pages + 1;
        let entries: Vec<serde_json::Value> = parts
            .iter()
            .map(|(d, pr)| {
                let first = page;
                page += pr.pages();
                let pages = if pr.pages() > 1 { format!("{first}-{}", page - 1) } else { first.to_string() };
                serde_json::json!({ "title": doc_name(p, *d), "kind": d.title(), "pages": pages })
            })
            .collect();
        let data = data::package(p, opts, entries, page - 1);
        let c = compile(Doc::Package, &data)?;
        let n = c.pages();
        cover = Some(c);
        if n == cover_pages {
            break;
        }
        cover_pages = n;
    }
    let cover = cover.ok_or("the cover could not be laid out")?;
    let mut pages: typst::ecow::EcoVec<typst_layout::Page> = cover.doc.pages().iter().cloned().collect();
    for (_, pr) in &parts {
        pages.extend(pr.doc.pages().iter().cloned());
    }
    let info = typst::model::Document::info(&cover.doc).clone();
    Ok(Printed { doc: PagedDocument::new(pages, info), warnings, fit: None })
}

/// Lay out `doc` for the project.
pub fn print(p: &bp_core::model::Project, doc: Doc, opts: &Options) -> Result<Printed, String> {
    if doc == Doc::Package {
        let items: Vec<(Doc, Options)> = default_package(p).into_iter().map(|d| (d, options_for(p, d))).collect();
        return print_package(p, &items, opts);
    }
    let data = build(p, doc, opts);
    let logo = match doc {
        Doc::Setlist(id) => p.setlist(id).and_then(|s| s.logo_bytes()).filter(|b| is_image(b)),
        _ => None,
    };
    compile_with(doc, &data, logo)
}

/// PNG, JPEG, GIF or SVG (anything else would stop the layout).
pub fn is_image(b: &[u8]) -> bool {
    b.starts_with(b"\x89PNG")
        || b.starts_with(&[0xff, 0xd8, 0xff])
        || b.starts_with(b"GIF8")
        || std::str::from_utf8(&b[..b.len().min(512)]).is_ok_and(|t| t.contains("<svg"))
}

/// Lay out already built data (the tests look at the data too).
pub fn compile(doc: Doc, data: &Data) -> Result<Printed, String> {
    compile_with(doc, data, None)
}

fn compile_with(doc: Doc, data: &Data, logo: Option<Vec<u8>>) -> Result<Printed, String> {
    let json = serde_json::to_string(&data.0).map_err(|e| e.to_string())?;
    let mut inputs = Dict::new();
    inputs.insert("data".into(), json.into_value());
    if let Some(l) = logo {
        inputs.insert("logo".into(), typst::foundations::Bytes::new(l).into_value());
    }
    let out = engine().compile_with_input::<_, _, PagedDocument>(doc.template(), inputs);
    let mut warnings: Vec<String> = out.warnings.iter().map(|w| w.message.to_string()).collect();
    match out.output {
        Ok(d) => {
            let fit = fit_size(&d);
            if let (Doc::Setlist(_), true) = (doc, d.pages().len() > 1) {
                warnings.push(
                    "The setlist does not fit on one page even at the smallest type: shorten it, choose a larger paper or another theme."
                        .into(),
                );
            }
            Ok(Printed { doc: d, warnings, fit })
        }
        Err(e) => Err(format!("The print layout failed: {e}")),
    }
}

/// The type size the template chose to fit the page (`<fit-size>`).
fn fit_size(d: &PagedDocument) -> Option<f64> {
    use typst::introspection::Introspector as _;
    let label = typst::foundations::Label::new(typst::utils::PicoStr::intern("fit-size"))?;
    let found = d.introspector().query_label(label).ok()?;
    let meta = found.to_packed::<typst::introspection::MetadataElem>()?;
    match &meta.value {
        typst::foundations::Value::Float(f) => Some(*f),
        typst::foundations::Value::Int(i) => Some(*i as f64),
        _ => None,
    }
}

impl Options {
    pub(crate) fn paper_name(&self) -> &'static str {
        self.paper.typst()
    }
    pub(crate) fn theme_name(&self) -> &'static str {
        self.theme.typst()
    }
}
