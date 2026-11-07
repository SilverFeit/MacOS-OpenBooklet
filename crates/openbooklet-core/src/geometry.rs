//! Geometric calculations for slots, PDF prepress boxes, creep, duplex calibration, and marks.

use crate::layout::{ImposedSheetSide, SlotPlacement};
use crate::model::{mm_to_pt, JobConfig, PrepressBoxMode, SheetSide};

/// Standard 2D rectangle in PostScript points (`origin_x, origin_y, width, height`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl PdfRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.width > 1.0 && self.height > 1.0
    }

    pub fn area(&self) -> f64 {
        (self.width.max(0.0)) * (self.height.max(0.0))
    }

    pub fn differs_from(&self, other: &Self) -> bool {
        (self.x - other.x).abs() > 0.5
            || (self.y - other.y).abs() > 0.5
            || (self.width - other.width).abs() > 0.5
            || (self.height - other.height).abs() > 0.5
    }
}

/// All five ISO 32000-1 PDF page boundary boxes inspected from a source page.
#[derive(Debug, Clone, Copy)]
pub struct PagePrepressBoxes {
    pub media_box: PdfRect,
    pub crop_box: PdfRect,
    pub bleed_box: PdfRect,
    pub trim_box: PdfRect,
    pub art_box: PdfRect,
    pub rotation_deg: i32,
}

impl PagePrepressBoxes {
    pub fn has_distinct_trim_box(&self) -> bool {
        self.trim_box.is_valid() && self.trim_box.differs_from(&self.media_box)
    }

    pub fn has_distinct_bleed_box(&self) -> bool {
        self.bleed_box.is_valid() && self.bleed_box.differs_from(&self.media_box)
    }

    pub fn has_distinct_crop_box(&self) -> bool {
        self.crop_box.is_valid() && self.crop_box.differs_from(&self.media_box)
    }

    /// Resolve the active framing box according to `PrepressBoxMode`.
    pub fn resolve_box(&self, mode: PrepressBoxMode) -> PdfRect {
        match mode {
            PrepressBoxMode::MediaBox => self.media_box,
            PrepressBoxMode::CropBox => {
                if self.crop_box.is_valid() {
                    self.crop_box
                } else {
                    self.media_box
                }
            }
            PrepressBoxMode::TrimBox => {
                if self.trim_box.is_valid() {
                    self.trim_box
                } else if self.crop_box.is_valid() {
                    self.crop_box
                } else {
                    self.media_box
                }
            }
            PrepressBoxMode::BleedBox => {
                if self.bleed_box.is_valid() {
                    self.bleed_box
                } else if self.crop_box.is_valid() {
                    self.crop_box
                } else {
                    self.media_box
                }
            }
            PrepressBoxMode::Auto => {
                if self.has_distinct_trim_box() {
                    self.trim_box
                } else if self.has_distinct_crop_box() {
                    self.crop_box
                } else {
                    self.media_box
                }
            }
        }
    }
}

/// Computed target rectangle on the output sheet for a single slot.
#[derive(Debug, Clone, Copy)]
pub struct ComputedSlotRect {
    /// Bounding rectangle of the slot cell (after margins, duplex shift, and creep).
    pub slot_rect: PdfRect,
}

/// Compute the target slot rectangle on a sheet of size `(sheet_w, sheet_h)` for a given `SlotPlacement`.
pub fn compute_slot_rect(
    config: &JobConfig,
    side: &ImposedSheetSide,
    slot: &SlotPlacement,
    sheet_w: f64,
    sheet_h: f64,
) -> ComputedSlotRect {
    let (cols, rows) = side.grid;
    let margin_pt = mm_to_pt(config.outer_margin_mm.max(0.0));
    let gutter_pt = mm_to_pt(config.spine_gutter_mm.max(0.0));

    let usable_w = (sheet_w - 2.0 * margin_pt - (if cols > 1 { gutter_pt } else { 0.0 })).max(36.0);
    let usable_h = (sheet_h - 2.0 * margin_pt).max(36.0);

    let cell_w = usable_w / (cols as f64);
    let cell_h = usable_h / (rows as f64);

    let mut x = margin_pt + (slot.col as f64) * cell_w;
    if cols > 1 && slot.col >= 1 {
        x += gutter_pt;
    }

    // Row 0 is the top row in reading order -> higher Y in PDF coordinates
    let inverted_row = rows.saturating_sub(1).saturating_sub(slot.row);
    let mut y = margin_pt + (inverted_row as f64) * cell_h;

    // Apply global shift + creep displacement
    x += mm_to_pt(config.shift_x_mm) + slot.creep_shift_x_pt;
    y += mm_to_pt(config.shift_y_mm);

    // Apply Duplex Calibration offset exclusively on Verso (Back) sheets
    if side.side == SheetSide::Verso {
        x += mm_to_pt(config.duplex_offset_x_mm);
        y += mm_to_pt(config.duplex_offset_y_mm);
    }

    ComputedSlotRect {
        slot_rect: PdfRect::new(x, y, cell_w, cell_h),
    }
}

/// Compute the stepped spine collating mark rectangle for the outermost sheet of a signature.
pub fn compute_collating_mark_rect(
    side: &ImposedSheetSide,
    sheet_w: f64,
    sheet_h: f64,
) -> Option<(PdfRect, usize)> {
    // Collating marks are placed on the outer fold of the outermost sheet (sheet_index_in_signature == 0, Recto)
    if side.side != SheetSide::Recto || side.sheet_index_in_signature != 0 {
        return None;
    }

    let mark_w = 5.0;
    let mark_h = 14.0;
    let top_margin = 52.0;
    let usable_spine = (sheet_h - 2.0 * top_margin - mark_h).max(mark_h);
    let step_offset = ((side.signature_index as f64) * mark_h) % usable_spine;
    let x = (sheet_w * 0.5) - (mark_w * 0.5);
    let y = sheet_h - top_margin - mark_h - step_offset;

    Some((
        PdfRect::new(x, y, mark_w, mark_h),
        side.signature_index + 1,
    ))
}
