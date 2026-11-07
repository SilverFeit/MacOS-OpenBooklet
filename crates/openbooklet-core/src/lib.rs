//! `openbooklet-core` — Clean-room PDF imposition, bookbinding signature, cut-and-stack,
//! de-imposition, duplex calibration, and prepress geometry engine.

pub mod geometry;
pub mod layout;
pub mod model;
pub mod preflight;
pub mod range;

pub use geometry::{
    compute_collating_mark_rect, compute_slot_rect, ComputedSlotRect, PagePrepressBoxes, PdfRect,
};
pub use layout::{compute_imposition_plan, ImposedSheetSide, ImpositionPlan, SlotPlacement};
pub use model::{
    mm_to_pt, pt_to_mm, ImpositionMode, JobConfig, PageCropRegion, PageNumberPosition, PageSource,
    PaperFormat, PrepressBoxMode, SheetSide, PT_PER_MM,
};
pub use preflight::{
    analyze_preflight, format_dry_run_table, PreflightNotice, PreflightReport, PreflightSeverity,
};
pub use range::parse_page_range;
