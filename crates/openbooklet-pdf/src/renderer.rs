//! Vector Quartz 2D (`CoreGraphics` + `CoreText`) PDF imposition and prepress renderer.

use std::f64::consts::PI;
use std::path::Path;
use std::ptr;

use openbooklet_core::{
    analyze_preflight, compute_collating_mark_rect, compute_imposition_plan, compute_slot_rect,
    ImposedSheetSide, ImpositionMode, ImpositionPlan, JobConfig, PageCropRegion,
    PageNumberPosition, PageSource, PdfRect, PreflightReport, SheetSide,
};

use crate::ffi::{
    cfurl_from_path, draw_vector_text, CFRelease, CGContextAddLineToPoint, CGContextBeginPath,
    CGContextClipToRect, CGContextDrawPDFPage, CGContextFillRect, CGContextMoveToPoint,
    CGContextRef, CGContextRelease, CGContextRestoreGState, CGContextRotateCTM,
    CGContextSaveGState, CGContextScaleCTM, CGContextSetLineDash, CGContextSetLineWidth,
    CGContextSetRGBFillColor, CGContextSetRGBStrokeColor, CGContextStrokeEllipseInRect,
    CGContextStrokePath, CGContextTranslateCTM, CGPDFContextBeginPage, CGPDFContextClose,
    CGPDFContextCreateWithURL, CGPDFContextEndPage, CGRect,
};
use crate::inspect::{inspect_cgpdf_page, ScopedCgPdfDocument};

/// Result of executing a full vector PDF imposition render.
#[derive(Debug, Clone)]
pub struct RenderSummary {
    pub output_path: String,
    pub plan: ImpositionPlan,
    pub preflight: PreflightReport,
}

/// Render an imposed (or de-imposed) vector PDF from `input_path` to `output_path`.
pub fn render_imposed_pdf(
    input_path: &Path,
    output_path: &Path,
    config: &JobConfig,
) -> Result<RenderSummary, String> {
    let doc = ScopedCgPdfDocument::open(input_path)?;
    let source_pages = doc.page_count();
    if source_pages == 0 {
        return Err("Source PDF contains 0 pages.".to_string());
    }

    let mut has_distinct_trim = false;
    for idx in 1..=source_pages {
        if let Some(p) = doc.page(idx) {
            let b = unsafe { inspect_cgpdf_page(p) };
            if b.has_distinct_trim_box() {
                has_distinct_trim = true;
                break;
            }
        }
    }

    let plan = compute_imposition_plan(config, source_pages);
    let preflight = analyze_preflight(config, &plan, has_distinct_trim);
    let (sheet_w, sheet_h) = config.sheet_dimensions_pt();

    let job_title = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("document.pdf");

    unsafe {
        let out_url = cfurl_from_path(output_path)
            .ok_or_else(|| format!("Invalid output path: {}", output_path.display()))?;
        let media_box = CGRect::new(0.0, 0.0, sheet_w, sheet_h);
        let ctx = CGPDFContextCreateWithURL(out_url, &media_box, ptr::null());
        CFRelease(out_url);

        if ctx.is_null() {
            return Err(format!(
                "CoreGraphics could not create output PDF context at {}",
                output_path.display()
            ));
        }

        for side in &plan.sides {
            CGPDFContextBeginPage(ctx, ptr::null());
            CGContextSaveGState(ctx);

            // White sheet background
            CGContextSetRGBFillColor(ctx, 1.0, 1.0, 1.0, 1.0);
            CGContextFillRect(ctx, media_box);

            // Calendar flip on Verso if enabled (rotates entire sheet side 180 deg around center)
            if side.side == SheetSide::Verso && config.calendar_flip_verso {
                CGContextTranslateCTM(ctx, sheet_w * 0.5, sheet_h * 0.5);
                CGContextRotateCTM(ctx, PI);
                CGContextTranslateCTM(ctx, -sheet_w * 0.5, -sheet_h * 0.5);
            }

            // Render every slot on this sheet side
            for slot in &side.slots {
                let computed = compute_slot_rect(config, side, slot, sheet_w, sheet_h);
                let slot_rect = computed.slot_rect;

                let mut crop_target_rect = slot_rect;
                if let PageSource::DocumentPage { page_number, crop } = slot.source {
                    if let Some(src_page) = doc.page(page_number) {
                        crop_target_rect =
                            draw_source_page_in_slot(ctx, src_page, crop, slot_rect, config);
                    }
                }

                if config.draw_crop_marks {
                    draw_corner_crop_marks(ctx, crop_target_rect);
                }

                if config.page_number_position != PageNumberPosition::None {
                    if let Some(reading_num) = slot.logical_reading_number {
                        if slot.source != PageSource::Blank {
                            draw_slot_page_number(
                                ctx,
                                slot_rect,
                                slot.col,
                                side.grid.0,
                                reading_num,
                                config.page_number_position,
                            );
                        }
                    }
                }
            }

            // Fold or Guillotine Cut guide lines
            if config.draw_fold_guides {
                draw_fold_or_cut_guides(ctx, config.mode, side.grid, sheet_w, sheet_h);
            }

            // Registration bullseye targets
            if config.draw_registration_marks {
                draw_registration_targets(ctx, sheet_w, sheet_h);
            }

            // Stepped Spine Collating Marks on outermost sheet of each signature
            if config.draw_collating_marks {
                if let Some((mark_rect, sig_num)) =
                    compute_collating_mark_rect(side, sheet_w, sheet_h)
                {
                    draw_spine_collating_mark(ctx, mark_rect, sig_num);
                }
            }

            // Technical prepress slug
            if config.draw_tech_slug {
                draw_technical_slug(ctx, job_title, config, side, sheet_w, sheet_h);
            }

            CGContextRestoreGState(ctx);
            CGPDFContextEndPage(ctx);
        }

        CGPDFContextClose(ctx);
        CGContextRelease(ctx);
    }

    Ok(RenderSummary {
        output_path: output_path.display().to_string(),
        plan,
        preflight,
    })
}

unsafe fn draw_source_page_in_slot(
    ctx: CGContextRef,
    src_page: crate::ffi::CGPDFPageRef,
    crop_region: PageCropRegion,
    slot_rect: PdfRect,
    config: &JobConfig,
) -> PdfRect {
    let boxes = inspect_cgpdf_page(src_page);
    let active_box = boxes.resolve_box(config.pdf_box_mode);

    // Determine effective source slice (FullPage, LeftHalf, or RightHalf)
    let slice_box = match crop_region {
        PageCropRegion::FullPage => active_box,
        PageCropRegion::LeftHalf => PdfRect::new(
            active_box.x,
            active_box.y,
            active_box.width * 0.5,
            active_box.height,
        ),
        PageCropRegion::RightHalf => PdfRect::new(
            active_box.x + active_box.width * 0.5,
            active_box.y,
            active_box.width * 0.5,
            active_box.height,
        ),
    };

    if !slice_box.is_valid() {
        return slot_rect;
    }

    let total_rot_deg = (boxes.rotation_deg + config.extra_rotation_deg).rem_euclid(360);
    let swaps_axes = total_rot_deg == 90 || total_rot_deg == 270;
    let eff_src_w = if swaps_axes {
        slice_box.height
    } else {
        slice_box.width
    };
    let eff_src_h = if swaps_axes {
        slice_box.width
    } else {
        slice_box.height
    };

    let base_scale = if config.auto_scale {
        let sx = slot_rect.width / eff_src_w.max(1.0);
        let sy = slot_rect.height / eff_src_h.max(1.0);
        sx.min(sy)
    } else {
        1.0
    };
    let final_scale = (base_scale * config.scale_factor).max(0.05);

    CGContextSaveGState(ctx);
    CGContextClipToRect(
        ctx,
        CGRect::new(slot_rect.x, slot_rect.y, slot_rect.width, slot_rect.height),
    );

    // Move origin to center of target slot
    let slot_cx = slot_rect.x + slot_rect.width * 0.5;
    let slot_cy = slot_rect.y + slot_rect.height * 0.5;
    CGContextTranslateCTM(ctx, slot_cx, slot_cy);

    let mirror_x = if config.mirror_horizontal { -1.0 } else { 1.0 };
    let mirror_y = if config.mirror_vertical { -1.0 } else { 1.0 };
    CGContextScaleCTM(ctx, final_scale * mirror_x, final_scale * mirror_y);

    if config.extra_rotation_deg != 0 {
        let rad = -(config.extra_rotation_deg as f64) * PI / 180.0;
        CGContextRotateCTM(ctx, rad);
    }

    // Clip to the slice region in PDF page coordinates so LeftHalf/RightHalf or TrimBox never leaks outside
    let slice_cx = slice_box.x + slice_box.width * 0.5;
    let slice_cy = slice_box.y + slice_box.height * 0.5;
    CGContextTranslateCTM(ctx, -slice_cx, -slice_cy);
    CGContextClipToRect(
        ctx,
        CGRect::new(slice_box.x, slice_box.y, slice_box.width, slice_box.height),
    );

    CGContextDrawPDFPage(ctx, src_page);
    CGContextRestoreGState(ctx);

    // Compute exact projected TrimBox (or active slice box) on the sheet for precision crop marks
    let trim_ref = if crop_region == PageCropRegion::FullPage && boxes.has_distinct_trim_box() {
        boxes.trim_box
    } else {
        slice_box
    };
    let (proj_w, proj_h) = if swaps_axes {
        (trim_ref.height * final_scale, trim_ref.width * final_scale)
    } else {
        (trim_ref.width * final_scale, trim_ref.height * final_scale)
    };
    PdfRect::new(
        slot_cx - proj_w * 0.5,
        slot_cy - proj_h * 0.5,
        proj_w,
        proj_h,
    )
}

unsafe fn draw_fold_or_cut_guides(
    ctx: CGContextRef,
    mode: ImpositionMode,
    grid: (usize, usize),
    sheet_w: f64,
    sheet_h: f64,
) {
    let (cols, rows) = grid;
    if cols <= 1 && rows <= 1 {
        return;
    }

    CGContextSaveGState(ctx);
    let is_cut_and_stack = matches!(mode, ImpositionMode::CutAndStack2Up | ImpositionMode::CutAndStack4Up);
    if is_cut_and_stack {
        // Cut & Stack uses a crisp cyan-gray guillotine cut line
        CGContextSetRGBStrokeColor(ctx, 0.10, 0.45, 0.65, 0.70);
        CGContextSetLineWidth(ctx, 0.65);
        let dash = [6.0, 3.0];
        CGContextSetLineDash(ctx, 0.0, dash.as_ptr(), dash.len());
    } else {
        // Fold guide uses a subtle warm gray dashed line
        CGContextSetRGBStrokeColor(ctx, 0.45, 0.45, 0.45, 0.60);
        CGContextSetLineWidth(ctx, 0.5);
        let dash = [4.0, 4.0];
        CGContextSetLineDash(ctx, 0.0, dash.as_ptr(), dash.len());
    }

    CGContextBeginPath(ctx);
    if cols > 1 {
        let mid_x = sheet_w * 0.5;
        CGContextMoveToPoint(ctx, mid_x, 0.0);
        CGContextAddLineToPoint(ctx, mid_x, sheet_h);
    }
    if rows > 1 {
        let mid_y = sheet_h * 0.5;
        CGContextMoveToPoint(ctx, 0.0, mid_y);
        CGContextAddLineToPoint(ctx, sheet_w, mid_y);
    }
    CGContextStrokePath(ctx);
    CGContextRestoreGState(ctx);
}

unsafe fn draw_corner_crop_marks(ctx: CGContextRef, r: PdfRect) {
    let mark_len = 12.0;
    let gap = 3.0;
    let x0 = r.x;
    let y0 = r.y;
    let x1 = r.x + r.width;
    let y1 = r.y + r.height;

    CGContextSaveGState(ctx);
    CGContextSetRGBStrokeColor(ctx, 0.08, 0.08, 0.08, 0.90);
    CGContextSetLineWidth(ctx, 0.45);
    CGContextBeginPath(ctx);

    // Bottom-Left
    CGContextMoveToPoint(ctx, x0 - gap - mark_len, y0);
    CGContextAddLineToPoint(ctx, x0 - gap, y0);
    CGContextMoveToPoint(ctx, x0, y0 - gap - mark_len);
    CGContextAddLineToPoint(ctx, x0, y0 - gap);

    // Bottom-Right
    CGContextMoveToPoint(ctx, x1 + gap, y0);
    CGContextAddLineToPoint(ctx, x1 + gap + mark_len, y0);
    CGContextMoveToPoint(ctx, x1, y0 - gap - mark_len);
    CGContextAddLineToPoint(ctx, x1, y0 - gap);

    // Top-Left
    CGContextMoveToPoint(ctx, x0 - gap - mark_len, y1);
    CGContextAddLineToPoint(ctx, x0 - gap, y1);
    CGContextMoveToPoint(ctx, x0, y1 + gap);
    CGContextAddLineToPoint(ctx, x0, y1 + gap + mark_len);

    // Top-Right
    CGContextMoveToPoint(ctx, x1 + gap, y1);
    CGContextAddLineToPoint(ctx, x1 + gap + mark_len, y1);
    CGContextMoveToPoint(ctx, x1, y1 + gap);
    CGContextAddLineToPoint(ctx, x1, y1 + gap + mark_len);

    CGContextStrokePath(ctx);
    CGContextRestoreGState(ctx);
}

unsafe fn draw_registration_targets(ctx: CGContextRef, sheet_w: f64, sheet_h: f64) {
    let margin = 15.0;
    let targets = [
        (sheet_w * 0.5, sheet_h - margin),
        (sheet_w * 0.5, margin),
        (margin, sheet_h * 0.5),
        (sheet_w - margin, sheet_h * 0.5),
    ];

    CGContextSaveGState(ctx);
    CGContextSetRGBStrokeColor(ctx, 0.0, 0.0, 0.0, 0.85);
    CGContextSetLineWidth(ctx, 0.45);

    let r_outer = 4.5;
    let r_inner = 2.0;
    let cross = 7.0;

    for (tx, ty) in targets {
        CGContextStrokeEllipseInRect(
            ctx,
            CGRect::new(tx - r_outer, ty - r_outer, r_outer * 2.0, r_outer * 2.0),
        );
        CGContextStrokeEllipseInRect(
            ctx,
            CGRect::new(tx - r_inner, ty - r_inner, r_inner * 2.0, r_inner * 2.0),
        );
        CGContextBeginPath(ctx);
        CGContextMoveToPoint(ctx, tx - cross, ty);
        CGContextAddLineToPoint(ctx, tx + cross, ty);
        CGContextMoveToPoint(ctx, tx, ty - cross);
        CGContextAddLineToPoint(ctx, tx, ty + cross);
        CGContextStrokePath(ctx);
    }

    CGContextRestoreGState(ctx);
}

unsafe fn draw_spine_collating_mark(ctx: CGContextRef, rect: PdfRect, sig_number: usize) {
    CGContextSaveGState(ctx);
    CGContextSetRGBFillColor(ctx, 0.05, 0.05, 0.05, 1.0);
    CGContextFillRect(ctx, CGRect::new(rect.x, rect.y, rect.width, rect.height));

    let label = format!("{}", sig_number);
    draw_vector_text(
        ctx,
        &label,
        rect.x + rect.width * 0.5,
        rect.y + 4.0,
        5.5,
        true,
        (1.0, 1.0, 1.0),
        true,
    );
    CGContextRestoreGState(ctx);
}

unsafe fn draw_slot_page_number(
    ctx: CGContextRef,
    slot_rect: PdfRect,
    col: usize,
    total_cols: usize,
    reading_num: usize,
    position: PageNumberPosition,
) {
    let text = format!("{}", reading_num);
    let pad_x = 18.0;
    let pad_y = 14.0;

    let x = match position {
        PageNumberPosition::BottomCenter | PageNumberPosition::TopCenter => {
            slot_rect.x + slot_rect.width * 0.5
        }
        PageNumberPosition::BottomOuter | PageNumberPosition::TopOuter => {
            if total_cols > 1 && col == 0 {
                slot_rect.x + pad_x
            } else if total_cols > 1 {
                slot_rect.x + slot_rect.width - pad_x
            } else {
                slot_rect.x + slot_rect.width * 0.5
            }
        }
        PageNumberPosition::None => return,
    };

    let y = match position {
        PageNumberPosition::BottomOuter | PageNumberPosition::BottomCenter => slot_rect.y + pad_y,
        PageNumberPosition::TopOuter | PageNumberPosition::TopCenter => {
            slot_rect.y + slot_rect.height - pad_y - 8.0
        }
        PageNumberPosition::None => return,
    };

    draw_vector_text(ctx, &text, x, y, 8.5, false, (0.20, 0.20, 0.20), true);
}

unsafe fn draw_technical_slug(
    ctx: CGContextRef,
    job_title: &str,
    config: &JobConfig,
    side: &ImposedSheetSide,
    sheet_w: f64,
    sheet_h: f64,
) {
    let extra_note = match config.mode {
        ImpositionMode::CutAndStack2Up => " | CUT & STACK: Cut center line, stack Left pile on top of Right pile",
        ImpositionMode::CutAndStack4Up => " | CUT & STACK 4-UP: Stack Top-Left -> Top-Right -> Bottom-Left -> Bottom-Right",
        _ => "",
    };

    let slug = format!(
        "OpenBooklet  |  Job: {}  |  Mode: {}  |  Sheet {} ({})  |  Sig {}/{}  |  {:.0}x{:.0} pt  |  Duplex dX={:+.1}mm dY={:+.1}mm{}",
        job_title,
        config.mode.as_slug().to_uppercase(),
        side.physical_sheet_number,
        side.side.label(),
        side.signature_index + 1,
        side.total_signatures,
        sheet_w,
        sheet_h,
        config.duplex_offset_x_mm,
        config.duplex_offset_y_mm,
        extra_note
    );

    draw_vector_text(ctx, &slug, 24.0, 6.5, 6.5, false, (0.35, 0.35, 0.38), false);
}
