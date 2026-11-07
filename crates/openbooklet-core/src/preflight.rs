//! Preflight analysis, diagnostics, and human/JSON reporting for imposition jobs.

use crate::layout::{ImpositionPlan, SlotPlacement};
use crate::model::{JobConfig, PageCropRegion, PageSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightSeverity {
    Ok,
    Info,
    Warning,
}

#[derive(Debug, Clone)]
pub struct PreflightNotice {
    pub severity: PreflightSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PreflightReport {
    pub status_badge: String,
    pub notices: Vec<PreflightNotice>,
}

/// Evaluate a computed `ImpositionPlan` and `JobConfig` to produce actionable prepress diagnostics.
pub fn analyze_preflight(
    config: &JobConfig,
    plan: &ImpositionPlan,
    has_distinct_trim_boxes: bool,
) -> PreflightReport {
    let mut notices = Vec::new();

    if plan.padded_blank_pages > 0 {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Info,
            code: "PAD_BLANKS",
            message: format!(
                "Auto-padded +{} blank page(s) to complete {} sheet(s) ({} total pages).",
                plan.padded_blank_pages, plan.total_physical_sheets, plan.total_logical_pages
            ),
        });
    } else {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Ok,
            code: "EXACT_MULTIPLE",
            message: format!(
                "Exact sheet fit: {} pages across {} duplex sheet(s) ({} signature(s)).",
                plan.total_logical_pages, plan.total_physical_sheets, plan.total_signatures
            ),
        });
    }

    if config.flyleaves_front > 0 || config.flyleaves_back > 0 {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Info,
            code: "FLYLEAVES_ADDED",
            message: format!(
                "Flyleaves active: +{} front / +{} back blank endpaper page(s).",
                config.flyleaves_front, config.flyleaves_back
            ),
        });
    }

    if config.duplex_offset_x_mm.abs() > 0.01 || config.duplex_offset_y_mm.abs() > 0.01 {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Info,
            code: "DUPLEX_CALIBRATED",
            message: format!(
                "Duplex Verso calibration active: X = {:+.2} mm, Y = {:+.2} mm.",
                config.duplex_offset_x_mm, config.duplex_offset_y_mm
            ),
        });
    }

    if has_distinct_trim_boxes {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Info,
            code: "TRIMBOX_DETECTED",
            message: format!(
                "Prepress TrimBox detected in source PDF (active box mode: {}).",
                config.pdf_box_mode.as_slug()
            ),
        });
    }

    if config.scale_factor < 0.75 || config.scale_factor > 1.25 {
        notices.push(PreflightNotice {
            severity: PreflightSeverity::Warning,
            code: "EXTREME_SCALE",
            message: format!(
                "Custom scale factor is {:.0}%, which may clip bleed or leave wide margins.",
                config.scale_factor * 100.0
            ),
        });
    }

    let status_badge = if plan.padded_blank_pages > 0 {
        format!(
            "Preflight: {} src + {} flyleaf + {} pad = {} pp • {} sheets ({} sig)",
            plan.selected_page_count,
            plan.flyleaves_front + plan.flyleaves_back,
            plan.padded_blank_pages,
            plan.total_logical_pages,
            plan.total_physical_sheets,
            plan.total_signatures
        )
    } else {
        format!(
            "Preflight OK: {} pp exact • {} physical sheets ({} sig)",
            plan.total_logical_pages, plan.total_physical_sheets, plan.total_signatures
        )
    };

    PreflightReport {
        status_badge,
        notices,
    }
}

fn format_slot_short(slot: &SlotPlacement) -> String {
    match slot.source {
        PageSource::Blank => "Blank".to_string(),
        PageSource::DocumentPage { page_number, crop } => match crop {
            PageCropRegion::FullPage => format!("p.{}", page_number),
            PageCropRegion::LeftHalf => format!("p.{}[L]", page_number),
            PageCropRegion::RightHalf => format!("p.{}[R]", page_number),
        },
    }
}

/// Render a human-readable dry-run table of all sheets and sides.
pub fn format_dry_run_table(config: &JobConfig, plan: &ImpositionPlan) -> String {
    let mut out = String::new();
    let (w_pt, h_pt) = config.sheet_dimensions_pt();
    out.push_str(&format!(
        "OpenBooklet Dry-Run Imposition Plan\n\
         ==================================\n\
         Mode               : {} ({})\n\
         Paper              : {} ({:.1} x {:.1} pt, {})\n\
         Source Pages       : {} (selected: {})\n\
         Flyleaves          : +{} front, +{} back\n\
         Auto-Padded Blanks : {}\n\
         Total Logical Pages: {}\n\
         Physical Sheets    : {} ({} sides, {} signature(s))\n\
         Duplex Calibration : Verso dX = {:+.2} mm, dY = {:+.2} mm\n\
         Creep Shingling    : {:.2} mm\n\n",
        config.mode.display_name(),
        config.mode.as_slug(),
        config.paper.as_slug().to_uppercase(),
        w_pt,
        h_pt,
        if config.is_landscape() {
            "Landscape"
        } else {
            "Portrait"
        },
        plan.source_page_count,
        plan.selected_page_count,
        plan.flyleaves_front,
        plan.flyleaves_back,
        plan.padded_blank_pages,
        plan.total_logical_pages,
        plan.total_physical_sheets,
        plan.sides.len(),
        plan.total_signatures,
        config.duplex_offset_x_mm,
        config.duplex_offset_y_mm,
        config.creep_mm,
    ));

    for side in &plan.sides {
        let slots_desc: Vec<String> = side.slots.iter().map(format_slot_short).collect();
        out.push_str(&format!(
            "  Sheet {:02} | {:<13} | Sig {:02}/{:02} (Sheet {:02}/{:02}) -> [ {} ]\n",
            side.physical_sheet_number,
            side.side.label(),
            side.signature_index + 1,
            side.total_signatures,
            side.sheet_index_in_signature + 1,
            side.sheets_in_signature,
            slots_desc.join(" | ")
        ));
    }

    out
}
