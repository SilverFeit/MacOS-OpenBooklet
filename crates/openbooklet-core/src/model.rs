//! Core domain models for OpenBooklet imposition, prepress boxes, and calibration.

pub const PT_PER_MM: f64 = 72.0 / 25.4;

#[inline]
pub fn mm_to_pt(mm: f64) -> f64 {
    mm * PT_PER_MM
}

#[inline]
pub fn pt_to_mm(pt: f64) -> f64 {
    pt / PT_PER_MM
}

/// Supported imposition and de-imposition modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpositionMode {
    /// Single saddle-stitch booklet (4 pages per folded sheet).
    Booklet,
    /// Multi-signature bookbinding (custom sheets per signature for sewn/glued spines).
    Signatures,
    /// Guillotine 2-Up Cut & Stack (cut down the center and place left pile on right pile).
    CutAndStack2Up,
    /// Guillotine 4-Up Cut & Stack (2x2 grid cut into 4 sequential stacks).
    CutAndStack4Up,
    /// Reverse an already-imposed 2-Up saddle-stitch booklet back to sequential 1..N reading order.
    DeimposeBooklet,
    /// Split 2-Up spreads horizontally into sequential single pages (Left, Right).
    Split2Up,
    /// Standard 2-Up side-by-side sequential layout.
    TwoUp,
    /// 4-Up 2x2 grid ordered row-by-row.
    FourUpRows,
    /// 4-Up 2x2 grid ordered column-by-column.
    FourUpCols,
    /// Step-and-repeat: 2 identical copies of each page per sheet side.
    NCopies2,
    /// Step-and-repeat: 4 identical copies of each page per sheet side.
    NCopies4,
    /// 1 page per sheet side with prepress framing, scaling, and marks.
    OneUp,
}

impl ImpositionMode {
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::Booklet => "booklet",
            Self::Signatures => "signatures",
            Self::CutAndStack2Up => "cut-stack-2up",
            Self::CutAndStack4Up => "cut-stack-4up",
            Self::DeimposeBooklet => "deimpose",
            Self::Split2Up => "split-2up",
            Self::TwoUp => "2up",
            Self::FourUpRows => "4up-rows",
            Self::FourUpCols => "4up-cols",
            Self::NCopies2 => "2-copies",
            Self::NCopies4 => "4-copies",
            Self::OneUp => "1up",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "booklet" | "saddle" | "single-booklet" => Some(Self::Booklet),
            "signatures" | "signature" | "cuadernillos" | "quires" => Some(Self::Signatures),
            "cut-stack-2up" | "cut-and-stack" | "cut-stack" | "guillotine-2up" => {
                Some(Self::CutAndStack2Up)
            }
            "cut-stack-4up" | "guillotine-4up" => Some(Self::CutAndStack4Up),
            "deimpose" | "deimpose-booklet" | "linearize" | "unbooklet" => {
                Some(Self::DeimposeBooklet)
            }
            "split-2up" | "split" | "spread-split" => Some(Self::Split2Up),
            "2up" | "two-up" => Some(Self::TwoUp),
            "4up" | "4up-rows" | "four-up-rows" => Some(Self::FourUpRows),
            "4up-cols" | "four-up-cols" => Some(Self::FourUpCols),
            "2-copies" | "2copies" | "step-2" => Some(Self::NCopies2),
            "4-copies" | "4copies" | "step-4" => Some(Self::NCopies4),
            "1up" | "one-up" | "single" => Some(Self::OneUp),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Booklet => "Single Booklet (Saddle-Stitch)",
            Self::Signatures => "Multi-Signature Book (Sewn / Glued)",
            Self::CutAndStack2Up => "Guillotine Cut & Stack (2-Up)",
            Self::CutAndStack4Up => "Guillotine Cut & Stack (4-Up)",
            Self::DeimposeBooklet => "De-Impose Booklet -> Linear PDF",
            Self::Split2Up => "Split 2-Up Spreads -> Single Pages",
            Self::TwoUp => "2-Up Sequential Side-by-Side",
            Self::FourUpRows => "4-Up Grid (2x2 by Rows)",
            Self::FourUpCols => "4-Up Grid (2x2 by Columns)",
            Self::NCopies2 => "Step & Repeat (2 Identical Copies)",
            Self::NCopies4 => "Step & Repeat (4 Identical Copies)",
            Self::OneUp => "1-Up Prepress Frame & Marks",
        }
    }

    pub fn slots_per_side(self) -> (usize, usize) {
        match self {
            Self::Booklet
            | Self::Signatures
            | Self::CutAndStack2Up
            | Self::TwoUp
            | Self::NCopies2 => (2, 1),
            Self::CutAndStack4Up
            | Self::FourUpRows
            | Self::FourUpCols
            | Self::NCopies4 => (2, 2),
            Self::DeimposeBooklet | Self::Split2Up | Self::OneUp => (1, 1),
        }
    }

    pub fn is_deimpose_or_split(self) -> bool {
        matches!(self, Self::DeimposeBooklet | Self::Split2Up)
    }

    pub fn default_landscape(self) -> bool {
        matches!(
            self,
            Self::Booklet
                | Self::Signatures
                | Self::CutAndStack2Up
                | Self::TwoUp
                | Self::NCopies2
        )
    }
}

/// Standard and commercial press sheet formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaperFormat {
    A4,
    Letter,
    Legal,
    A3,
    Tabloid,
    Sra3,
    A5,
    Custom { width_mm: f64, height_mm: f64 },
}

impl PaperFormat {
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "letter",
            Self::Legal => "legal",
            Self::A3 => "a3",
            Self::Tabloid => "tabloid",
            Self::Sra3 => "sra3",
            Self::A5 => "a5",
            Self::Custom { .. } => "custom",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "a4" => Some(Self::A4),
            "letter" | "us-letter" => Some(Self::Letter),
            "legal" | "us-legal" => Some(Self::Legal),
            "a3" => Some(Self::A3),
            "tabloid" | "ledger" => Some(Self::Tabloid),
            "sra3" => Some(Self::Sra3),
            "a5" => Some(Self::A5),
            _ => None,
        }
    }

    pub fn dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::Letter => (215.9, 279.4),
            Self::Legal => (215.9, 355.6),
            Self::A3 => (297.0, 420.0),
            Self::Tabloid => (279.4, 431.8),
            Self::Sra3 => (320.0, 450.0),
            Self::A5 => (148.0, 210.0),
            Self::Custom {
                width_mm,
                height_mm,
            } => (width_mm.max(20.0), height_mm.max(20.0)),
        }
    }

    pub fn dimensions_pt(self, landscape: bool) -> (f64, f64) {
        let (w_mm, h_mm) = self.dimensions_mm();
        let (short_mm, long_mm) = if w_mm <= h_mm {
            (w_mm, h_mm)
        } else {
            (h_mm, w_mm)
        };
        if landscape {
            (mm_to_pt(long_mm), mm_to_pt(short_mm))
        } else {
            (mm_to_pt(short_mm), mm_to_pt(long_mm))
        }
    }
}

/// Prepress PDF geometry box selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepressBoxMode {
    /// Automatically prefer TrimBox if defined, then CropBox, then MediaBox.
    Auto,
    MediaBox,
    CropBox,
    TrimBox,
    BleedBox,
}

impl PrepressBoxMode {
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::MediaBox => "media",
            Self::CropBox => "crop",
            Self::TrimBox => "trim",
            Self::BleedBox => "bleed",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "media" | "mediabox" => Some(Self::MediaBox),
            "crop" | "cropbox" => Some(Self::CropBox),
            "trim" | "trimbox" => Some(Self::TrimBox),
            "bleed" | "bleedbox" => Some(Self::BleedBox),
            _ => None,
        }
    }
}

/// Sub-region of a source page to extract (used by De-Impose and Split 2-Up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCropRegion {
    FullPage,
    LeftHalf,
    RightHalf,
}

/// Logical page reference placed into a sheet slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSource {
    Blank,
    DocumentPage {
        /// 1-based page number in the source PDF.
        page_number: usize,
        crop: PageCropRegion,
    },
}

/// Side of a physical duplex press sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetSide {
    Recto,
    Verso,
}

impl SheetSide {
    pub fn label(self) -> &'static str {
        match self {
            Self::Recto => "Recto (Front)",
            Self::Verso => "Verso (Back)",
        }
    }
}

/// Optional page number stamp placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageNumberPosition {
    None,
    BottomOuter,
    BottomCenter,
    TopOuter,
    TopCenter,
}

impl PageNumberPosition {
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BottomOuter => "bottom-outer",
            Self::BottomCenter => "bottom-center",
            Self::TopOuter => "top-outer",
            Self::TopCenter => "top-center",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" | "off" => Some(Self::None),
            "bottom-outer" | "outer" => Some(Self::BottomOuter),
            "bottom-center" | "center" => Some(Self::BottomCenter),
            "top-outer" => Some(Self::TopOuter),
            "top-center" => Some(Self::TopCenter),
            _ => None,
        }
    }
}

/// Complete configuration for an imposition job.
#[derive(Debug, Clone)]
pub struct JobConfig {
    pub mode: ImpositionMode,
    pub paper: PaperFormat,
    /// None = follow mode default; Some(true) = Landscape; Some(false) = Portrait.
    pub landscape_override: Option<bool>,
    /// Number of physical folded sheets per signature (default 4 = 16 pages).
    pub sheets_per_signature: usize,
    /// Right-to-left binding (Manga / Arabic / Hebrew).
    pub right_to_left: bool,
    /// Rotate Verso sides by 180 degrees for top-bound calendars or short-edge flip.
    pub calendar_flip_verso: bool,
    /// Page range filter ("all", "odd", "even", or "1-16, 20").
    pub page_range: String,
    /// Blank flyleaf (endpaper) pages inserted at the front of the book.
    pub flyleaves_front: usize,
    /// Blank flyleaf (endpaper) pages appended at the back of the book.
    pub flyleaves_back: usize,
    /// Creep (shingling) compensation in millimeters from outer to inner sheet.
    pub creep_mm: f64,
    /// Duplex calibration X offset in mm applied exclusively to Verso (Back) sheets.
    pub duplex_offset_x_mm: f64,
    /// Duplex calibration Y offset in mm applied exclusively to Verso (Back) sheets.
    pub duplex_offset_y_mm: f64,
    /// Auto-scale source page to fit target slot.
    pub auto_scale: bool,
    /// Custom scale factor (1.0 = 100%). Applied on top of auto-scale or raw size.
    pub scale_factor: f64,
    /// Global horizontal shift in mm.
    pub shift_x_mm: f64,
    /// Global vertical shift in mm.
    pub shift_y_mm: f64,
    /// Extra central spine gutter margin in mm.
    pub spine_gutter_mm: f64,
    /// Outer sheet margin in mm.
    pub outer_margin_mm: f64,
    /// Additional page rotation in degrees (0, 90, 180, 270).
    pub extra_rotation_deg: i32,
    /// Mirror horizontally.
    pub mirror_horizontal: bool,
    /// Mirror vertically.
    pub mirror_vertical: bool,
    /// Prepress PDF box mode.
    pub pdf_box_mode: PrepressBoxMode,
    /// Draw dashed fold or guillotine cut guide lines.
    pub draw_fold_guides: bool,
    /// Draw perimeter corner crop marks.
    pub draw_crop_marks: bool,
    /// Draw prepress registration bullseye targets.
    pub draw_registration_marks: bool,
    /// Draw stepped spine collating marks on signatures.
    pub draw_collating_marks: bool,
    /// Draw technical prepress slug (job name, sheet/side, signature, calibration info).
    pub draw_tech_slug: bool,
    /// Optional page number stamp position.
    pub page_number_position: PageNumberPosition,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            mode: ImpositionMode::Booklet,
            paper: PaperFormat::A4,
            landscape_override: None,
            sheets_per_signature: 4,
            right_to_left: false,
            calendar_flip_verso: false,
            page_range: "all".to_string(),
            flyleaves_front: 0,
            flyleaves_back: 0,
            creep_mm: 0.0,
            duplex_offset_x_mm: 0.0,
            duplex_offset_y_mm: 0.0,
            auto_scale: true,
            scale_factor: 1.0,
            shift_x_mm: 0.0,
            shift_y_mm: 0.0,
            spine_gutter_mm: 0.0,
            outer_margin_mm: 0.0,
            extra_rotation_deg: 0,
            mirror_horizontal: false,
            mirror_vertical: false,
            pdf_box_mode: PrepressBoxMode::Auto,
            draw_fold_guides: true,
            draw_crop_marks: false,
            draw_registration_marks: false,
            draw_collating_marks: false,
            draw_tech_slug: false,
            page_number_position: PageNumberPosition::None,
        }
    }
}

impl JobConfig {
    pub fn is_landscape(&self) -> bool {
        self.landscape_override
            .unwrap_or_else(|| self.mode.default_landscape())
    }

    pub fn sheet_dimensions_pt(&self) -> (f64, f64) {
        self.paper.dimensions_pt(self.is_landscape())
    }

    pub fn to_json_pretty(&self) -> String {
        format!(
            "{{\n  \"mode\": \"{}\",\n  \"paper\": \"{}\",\n  \"landscape\": {},\n  \"sheets_per_signature\": {},\n  \"right_to_left\": {},\n  \"calendar_flip_verso\": {},\n  \"page_range\": \"{}\",\n  \"flyleaves_front\": {},\n  \"flyleaves_back\": {},\n  \"creep_mm\": {:.3},\n  \"duplex_offset_x_mm\": {:.3},\n  \"duplex_offset_y_mm\": {:.3},\n  \"auto_scale\": {},\n  \"scale_factor\": {:.3},\n  \"shift_x_mm\": {:.3},\n  \"shift_y_mm\": {:.3},\n  \"spine_gutter_mm\": {:.3},\n  \"outer_margin_mm\": {:.3},\n  \"extra_rotation_deg\": {},\n  \"mirror_horizontal\": {},\n  \"mirror_vertical\": {},\n  \"pdf_box_mode\": \"{}\",\n  \"draw_fold_guides\": {},\n  \"draw_crop_marks\": {},\n  \"draw_registration_marks\": {},\n  \"draw_collating_marks\": {},\n  \"draw_tech_slug\": {},\n  \"page_number_position\": \"{}\"\n}}",
            self.mode.as_slug(),
            self.paper.as_slug(),
            self.is_landscape(),
            self.sheets_per_signature,
            self.right_to_left,
            self.calendar_flip_verso,
            self.page_range.replace('"', "\\\""),
            self.flyleaves_front,
            self.flyleaves_back,
            self.creep_mm,
            self.duplex_offset_x_mm,
            self.duplex_offset_y_mm,
            self.auto_scale,
            self.scale_factor,
            self.shift_x_mm,
            self.shift_y_mm,
            self.spine_gutter_mm,
            self.outer_margin_mm,
            self.extra_rotation_deg,
            self.mirror_horizontal,
            self.mirror_vertical,
            self.pdf_box_mode.as_slug(),
            self.draw_fold_guides,
            self.draw_crop_marks,
            self.draw_registration_marks,
            self.draw_collating_marks,
            self.draw_tech_slug,
            self.page_number_position.as_slug(),
        )
    }
}
