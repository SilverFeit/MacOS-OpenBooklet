//! Printable 2-Page Duplex Printer Calibration Sheet & Sample PDF Generator.

use std::path::Path;
use std::ptr;

use openbooklet_core::{mm_to_pt, PaperFormat};

use crate::ffi::{
    cfurl_from_path, draw_vector_text, CFRelease, CGContextAddLineToPoint, CGContextBeginPath,
    CGContextFillRect, CGContextMoveToPoint, CGContextRelease, CGContextRestoreGState,
    CGContextSaveGState, CGContextSetLineWidth, CGContextSetRGBFillColor,
    CGContextSetRGBStrokeColor, CGContextStrokeEllipseInRect, CGContextStrokePath,
    CGContextStrokeRect, CGPDFContextBeginPage, CGPDFContextClose, CGPDFContextCreateWithURL,
    CGPDFContextEndPage, CGRect,
};

/// Generate a printable 2-page Duplex Alignment & Vernier Calibration Sheet PDF.
pub fn generate_duplex_calibration_pdf(
    output_path: &Path,
    paper: PaperFormat,
    landscape: bool,
) -> Result<(), String> {
    let (sheet_w, sheet_h) = paper.dimensions_pt(landscape);
    let cx = sheet_w * 0.5;
    let cy = sheet_h * 0.5;

    unsafe {
        let out_url = cfurl_from_path(output_path)
            .ok_or_else(|| format!("Invalid calibration output path: {}", output_path.display()))?;
        let media_box = CGRect::new(0.0, 0.0, sheet_w, sheet_h);
        let ctx = CGPDFContextCreateWithURL(out_url, &media_box, ptr::null());
        CFRelease(out_url);

        if ctx.is_null() {
            return Err("Could not create calibration PDF context.".to_string());
        }

        // --- PAGE 1: RECTO (FRONT) ---
        CGPDFContextBeginPage(ctx, ptr::null());
        CGContextSaveGState(ctx);
        CGContextSetRGBFillColor(ctx, 1.0, 1.0, 1.0, 1.0);
        CGContextFillRect(ctx, media_box);

        draw_vector_text(
            ctx,
            "OPENBOOKLET — DUPLEX ALIGNMENT CALIBRATION SHEET (SIDE 1: RECTO / FRONT)",
            cx,
            sheet_h - 46.0,
            11.0,
            true,
            (0.08, 0.12, 0.22),
            true,
        );
        draw_vector_text(
            ctx,
            "1. Print this 2-page PDF Duplex (100% Actual Size / No Scaling, Long-Edge Flip).",
            cx,
            sheet_h - 66.0,
            8.5,
            false,
            (0.25, 0.28, 0.32),
            true,
        );
        draw_vector_text(
            ctx,
            "2. Hold the printed sheet up to a light source and read where the RED Back-Side crosshair lands on this Front millimeter ruler.",
            cx,
            sheet_h - 80.0,
            8.5,
            false,
            (0.25, 0.28, 0.32),
            true,
        );
        draw_vector_text(
            ctx,
            "3. Enter the opposite X / Y values (in mm) under Duplex Calibration in OpenBooklet so Front & Back align to 0.0 mm.",
            cx,
            sheet_h - 94.0,
            8.5,
            false,
            (0.25, 0.28, 0.32),
            true,
        );

        // Draw X and Y millimeter rulers from -15 mm to +15 mm around center (cx, cy)
        CGContextSetRGBStrokeColor(ctx, 0.10, 0.15, 0.25, 0.90);
        CGContextSetLineWidth(ctx, 0.6);
        let span_pt = mm_to_pt(20.0);

        CGContextBeginPath(ctx);
        CGContextMoveToPoint(ctx, cx - span_pt, cy);
        CGContextAddLineToPoint(ctx, cx + span_pt, cy);
        CGContextMoveToPoint(ctx, cx, cy - span_pt);
        CGContextAddLineToPoint(ctx, cx, cy + span_pt);
        CGContextStrokePath(ctx);

        for mm in -15i32..=15i32 {
            let offset = mm_to_pt(mm as f64);
            let tick_h = if mm % 5 == 0 { 8.0 } else { 4.0 };

            CGContextBeginPath(ctx);
            // X-axis tick
            CGContextMoveToPoint(ctx, cx + offset, cy - tick_h);
            CGContextAddLineToPoint(ctx, cx + offset, cy + tick_h);
            // Y-axis tick
            CGContextMoveToPoint(ctx, cx - tick_h, cy + offset);
            CGContextAddLineToPoint(ctx, cx + tick_h, cy + offset);
            CGContextStrokePath(ctx);

            if mm != 0 && mm % 5 == 0 {
                let lbl = format!("{:+}", mm);
                draw_vector_text(
                    ctx,
                    &lbl,
                    cx + offset,
                    cy - 18.0,
                    6.5,
                    false,
                    (0.15, 0.20, 0.30),
                    true,
                );
                draw_vector_text(
                    ctx,
                    &lbl,
                    cx + 16.0,
                    cy + offset - 2.5,
                    6.5,
                    false,
                    (0.15, 0.20, 0.30),
                    false,
                );
            }
        }

        // Concentric target rings on Recto
        for radius_mm in [2.0, 5.0, 10.0] {
            let r = mm_to_pt(radius_mm);
            CGContextStrokeEllipseInRect(ctx, CGRect::new(cx - r, cy - r, r * 2.0, r * 2.0));
        }

        CGContextRestoreGState(ctx);
        CGPDFContextEndPage(ctx);

        // --- PAGE 2: VERSO (BACK) ---
        CGPDFContextBeginPage(ctx, ptr::null());
        CGContextSaveGState(ctx);
        CGContextSetRGBFillColor(ctx, 1.0, 1.0, 1.0, 1.0);
        CGContextFillRect(ctx, media_box);

        draw_vector_text(
            ctx,
            "OPENBOOKLET — DUPLEX ALIGNMENT CALIBRATION SHEET (SIDE 2: VERSO / BACK)",
            cx,
            sheet_h - 46.0,
            11.0,
            true,
            (0.75, 0.12, 0.18),
            true,
        );

        // Bold crimson crosshair at exact center (cx, cy)
        CGContextSetRGBStrokeColor(ctx, 0.82, 0.10, 0.15, 0.95);
        CGContextSetLineWidth(ctx, 1.1);
        let arm = mm_to_pt(18.0);
        CGContextBeginPath(ctx);
        CGContextMoveToPoint(ctx, cx - arm, cy);
        CGContextAddLineToPoint(ctx, cx + arm, cy);
        CGContextMoveToPoint(ctx, cx, cy - arm);
        CGContextAddLineToPoint(ctx, cx, cy + arm);
        CGContextStrokePath(ctx);

        let r_verso = mm_to_pt(3.5);
        CGContextStrokeEllipseInRect(
            ctx,
            CGRect::new(cx - r_verso, cy - r_verso, r_verso * 2.0, r_verso * 2.0),
        );

        CGContextRestoreGState(ctx);
        CGPDFContextEndPage(ctx);

        CGPDFContextClose(ctx);
        CGContextRelease(ctx);
    }

    Ok(())
}

/// Generate a multi-page numbered vector sample PDF document for testing and previews.
pub fn generate_sample_document_pdf(
    output_path: &Path,
    num_pages: usize,
    paper: PaperFormat,
) -> Result<(), String> {
    let total = num_pages.max(1);
    let (w, h) = paper.dimensions_pt(false);

    unsafe {
        let out_url = cfurl_from_path(output_path)
            .ok_or_else(|| format!("Invalid sample PDF path: {}", output_path.display()))?;
        let media_box = CGRect::new(0.0, 0.0, w, h);
        let ctx = CGPDFContextCreateWithURL(out_url, &media_box, ptr::null());
        CFRelease(out_url);

        if ctx.is_null() {
            return Err("Could not create sample PDF context.".to_string());
        }

        for p in 1..=total {
            CGPDFContextBeginPage(ctx, ptr::null());
            CGContextSaveGState(ctx);

            CGContextSetRGBFillColor(ctx, 0.99, 0.99, 1.0, 1.0);
            CGContextFillRect(ctx, media_box);

            // Outer frame
            CGContextSetRGBStrokeColor(ctx, 0.18, 0.26, 0.42, 0.85);
            CGContextSetLineWidth(ctx, 2.0);
            CGContextStrokeRect(ctx, CGRect::new(28.0, 28.0, w - 56.0, h - 56.0));

            // Inner decorative rule
            CGContextSetRGBStrokeColor(ctx, 0.55, 0.62, 0.74, 0.55);
            CGContextSetLineWidth(ctx, 0.75);
            CGContextStrokeRect(ctx, CGRect::new(36.0, 36.0, w - 72.0, h - 72.0));

            // Large centered page number
            let num_str = format!("{}", p);
            draw_vector_text(
                ctx,
                &num_str,
                w * 0.5,
                h * 0.5 - 24.0,
                72.0,
                true,
                (0.12, 0.18, 0.32),
                true,
            );

            let sub_str = format!("OpenBooklet Sample Page {} of {}", p, total);
            draw_vector_text(
                ctx,
                &sub_str,
                w * 0.5,
                h * 0.5 - 56.0,
                13.0,
                false,
                (0.35, 0.40, 0.48),
                true,
            );

            CGContextRestoreGState(ctx);
            CGPDFContextEndPage(ctx);
        }

        CGPDFContextClose(ctx);
        CGContextRelease(ctx);
    }

    Ok(())
}
