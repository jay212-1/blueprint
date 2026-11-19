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
}

impl Doc {
    pub const ALL: [Doc; 6] = [
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
        }
    }

    /// Landscape unless chosen otherwise.
    pub fn landscape(self) -> bool {
        matches!(self, Doc::MicPlot | Doc::DcaSheet)
    }

    fn template(self) -> &'static str {
        match self {
            Doc::MicPlot => "mic-plot.typ",
            Doc::MicCards => "mic-cards.typ",
            _ => "table.typ",
        }
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
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Stage, Theme::Technical, Theme::Ink];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Stage => "Stage",
            Theme::Technical => "Technical",
            Theme::Ink => "Save ink",
        }
    }

    fn typst(self) -> &'static str {
        match self {
            Theme::Stage => "stage",
            Theme::Technical => "technical",
            Theme::Ink => "ink",
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

impl Options {
    pub fn for_doc(doc: Doc) -> Options {
        Options {
            paper: Paper::A4,
            landscape: doc.landscape(),
            theme: Theme::Technical,
            mark_changes: true,
        }
    }
}

const TEMPLATES: [(&str, &str); 4] = [
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

/// Lay out `doc` for the project.
pub fn print(p: &bp_core::model::Project, doc: Doc, opts: &Options) -> Result<Printed, String> {
    let data = build(p, doc, opts);
    compile(doc, &data)
}

/// Lay out already built data (the tests look at the data too).
pub fn compile(doc: Doc, data: &Data) -> Result<Printed, String> {
    let json = serde_json::to_string(&data.0).map_err(|e| e.to_string())?;
    let mut inputs = Dict::new();
    inputs.insert("data".into(), json.into_value());
    let out = engine().compile_with_input::<_, _, PagedDocument>(doc.template(), inputs);
    let warnings = out.warnings.iter().map(|w| w.message.to_string()).collect();
    match out.output {
        Ok(doc) => Ok(Printed { doc, warnings }),
        Err(e) => Err(format!("The print layout failed: {e}")),
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
