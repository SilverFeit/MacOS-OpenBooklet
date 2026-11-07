//! `openbooklet-pdf` — Vector Quartz 2D (`CoreGraphics` + `CoreText`) PDF inspection,
//! imposition renderer, and printable duplex calibration sheet generator.

pub mod calibration;
pub mod ffi;
pub mod inspect;
pub mod renderer;

pub use calibration::{generate_duplex_calibration_pdf, generate_sample_document_pdf};
pub use inspect::{inspect_pdf_file, PdfDocumentInspection, ScopedCgPdfDocument};
pub use renderer::{render_imposed_pdf, RenderSummary};

#[cfg(test)]
mod tests {
    use super::*;
    use openbooklet_core::{ImpositionMode, JobConfig, PaperFormat};

    #[test]
    fn test_end_to_end_vector_pdf_imposition_and_calibration() {
        let tmp_dir = std::env::temp_dir();
        let sample_in = tmp_dir.join("openbooklet_test_in8.pdf");
        let booklet_out = tmp_dir.join("openbooklet_test_booklet.pdf");
        let deimpose_out = tmp_dir.join("openbooklet_test_deimpose.pdf");
        let cal_out = tmp_dir.join("openbooklet_test_cal.pdf");

        generate_sample_document_pdf(&sample_in, 8, PaperFormat::A4)
            .expect("generate sample 8-page PDF");
        let info = inspect_pdf_file(&sample_in).expect("inspect sample PDF");
        assert_eq!(info.page_count, 8);

        let mut cfg = JobConfig::default();
        cfg.mode = ImpositionMode::Booklet;
        cfg.draw_crop_marks = true;
        cfg.draw_registration_marks = true;
        cfg.draw_collating_marks = true;
        cfg.draw_tech_slug = true;
        cfg.duplex_offset_x_mm = 1.25;
        cfg.duplex_offset_y_mm = -0.75;

        let summary =
            render_imposed_pdf(&sample_in, &booklet_out, &cfg).expect("render booklet PDF");
        assert_eq!(summary.plan.total_physical_sheets, 2);

        let imposed_info = inspect_pdf_file(&booklet_out).expect("inspect imposed PDF");
        assert_eq!(imposed_info.page_count, 4); // 2 physical sheets * 2 sides

        // Now de-impose the 4 spread sides back into 8 sequential single pages!
        let mut de_cfg = JobConfig::default();
        de_cfg.mode = ImpositionMode::DeimposeBooklet;
        render_imposed_pdf(&booklet_out, &deimpose_out, &de_cfg).expect("de-impose booklet PDF");
        let deimposed_info = inspect_pdf_file(&deimpose_out).expect("inspect deimposed PDF");
        assert_eq!(deimposed_info.page_count, 8);

        // Generate duplex calibration sheet
        generate_duplex_calibration_pdf(&cal_out, PaperFormat::A4, true)
            .expect("generate duplex calibration sheet");
        let cal_info = inspect_pdf_file(&cal_out).expect("inspect calibration PDF");
        assert_eq!(cal_info.page_count, 2);

        let _ = std::fs::remove_file(&sample_in);
        let _ = std::fs::remove_file(&booklet_out);
        let _ = std::fs::remove_file(&deimpose_out);
        let _ = std::fs::remove_file(&cal_out);
    }
}
