//! Prepress PDF inspection: page count, rotation, and ISO 32000-1 boundary boxes.

use std::path::Path;

use openbooklet_core::{pt_to_mm, PagePrepressBoxes, PdfRect};

use crate::ffi::{
    cfurl_from_path, CFRelease, CGPDFBox, CGPDFDocumentCreateWithURL,
    CGPDFDocumentGetNumberOfPages, CGPDFDocumentGetPage, CGPDFDocumentRef, CGPDFDocumentRelease,
    CGPDFPageGetBoxRect, CGPDFPageGetRotationAngle, CGPDFPageRef, CGRect,
};

/// Summary of an inspected PDF document.
#[derive(Debug, Clone)]
pub struct PdfDocumentInspection {
    pub path_display: String,
    pub page_count: usize,
    pub pages: Vec<PagePrepressBoxes>,
    pub has_any_distinct_trim_box: bool,
    pub has_any_distinct_bleed_box: bool,
    pub has_any_distinct_crop_box: bool,
}

impl PdfDocumentInspection {
    pub fn to_json_pretty(&self) -> String {
        let first_box = self.pages.first().copied().unwrap_or(PagePrepressBoxes {
            media_box: PdfRect::new(0.0, 0.0, 595.28, 841.89),
            crop_box: PdfRect::new(0.0, 0.0, 595.28, 841.89),
            bleed_box: PdfRect::new(0.0, 0.0, 595.28, 841.89),
            trim_box: PdfRect::new(0.0, 0.0, 595.28, 841.89),
            art_box: PdfRect::new(0.0, 0.0, 595.28, 841.89),
            rotation_deg: 0,
        });
        format!(
            "{{\n  \"file\": \"{}\",\n  \"page_count\": {},\n  \"has_distinct_trim_box\": {},\n  \"has_distinct_bleed_box\": {},\n  \"has_distinct_crop_box\": {},\n  \"page_1\": {{\n    \"media_box_mm\": [{:.2}, {:.2}],\n    \"crop_box_mm\": [{:.2}, {:.2}],\n    \"bleed_box_mm\": [{:.2}, {:.2}],\n    \"trim_box_mm\": [{:.2}, {:.2}],\n    \"rotation_deg\": {}\n  }}\n}}",
            self.path_display.replace('"', "\\\""),
            self.page_count,
            self.has_any_distinct_trim_box,
            self.has_any_distinct_bleed_box,
            self.has_any_distinct_crop_box,
            pt_to_mm(first_box.media_box.width),
            pt_to_mm(first_box.media_box.height),
            pt_to_mm(first_box.crop_box.width),
            pt_to_mm(first_box.crop_box.height),
            pt_to_mm(first_box.bleed_box.width),
            pt_to_mm(first_box.bleed_box.height),
            pt_to_mm(first_box.trim_box.width),
            pt_to_mm(first_box.trim_box.height),
            first_box.rotation_deg,
        )
    }
}

#[inline]
fn cgrect_to_pdfrect(r: CGRect) -> PdfRect {
    PdfRect::new(r.origin.x, r.origin.y, r.size.width, r.size.height)
}

/// Inspect all pages and prepress boxes of a PDF document on disk.
pub fn inspect_pdf_file(path: &Path) -> Result<PdfDocumentInspection, String> {
    unsafe {
        let url = cfurl_from_path(path)
            .ok_or_else(|| format!("Invalid filesystem path: {}", path.display()))?;
        let doc = CGPDFDocumentCreateWithURL(url);
        CFRelease(url);

        if doc.is_null() {
            return Err(format!("Failed to open PDF document: {}", path.display()));
        }

        let count = CGPDFDocumentGetNumberOfPages(doc);
        let mut pages = Vec::with_capacity(count);
        let mut has_trim = false;
        let mut has_bleed = false;
        let mut has_crop = false;

        for idx in 1..=count {
            let page = CGPDFDocumentGetPage(doc, idx);
            if page.is_null() {
                continue;
            }
            let boxes = inspect_cgpdf_page(page);
            if boxes.has_distinct_trim_box() {
                has_trim = true;
            }
            if boxes.has_distinct_bleed_box() {
                has_bleed = true;
            }
            if boxes.has_distinct_crop_box() {
                has_crop = true;
            }
            pages.push(boxes);
        }

        CGPDFDocumentRelease(doc);

        Ok(PdfDocumentInspection {
            path_display: path.display().to_string(),
            page_count: count,
            pages,
            has_any_distinct_trim_box: has_trim,
            has_any_distinct_bleed_box: has_bleed,
            has_any_distinct_crop_box: has_crop,
        })
    }
}

/// Read all 5 ISO 32000-1 boundary boxes and rotation from a `CGPDFPageRef`.
pub unsafe fn inspect_cgpdf_page(page: CGPDFPageRef) -> PagePrepressBoxes {
    let media_box = cgrect_to_pdfrect(CGPDFPageGetBoxRect(page, CGPDFBox::Media));
    let crop_box = cgrect_to_pdfrect(CGPDFPageGetBoxRect(page, CGPDFBox::Crop));
    let bleed_box = cgrect_to_pdfrect(CGPDFPageGetBoxRect(page, CGPDFBox::Bleed));
    let trim_box = cgrect_to_pdfrect(CGPDFPageGetBoxRect(page, CGPDFBox::Trim));
    let art_box = cgrect_to_pdfrect(CGPDFPageGetBoxRect(page, CGPDFBox::Art));
    let rotation_deg = CGPDFPageGetRotationAngle(page);

    PagePrepressBoxes {
        media_box,
        crop_box,
        bleed_box,
        trim_box,
        art_box,
        rotation_deg,
    }
}

/// RAII wrapper around `CGPDFDocumentRef`.
pub struct ScopedCgPdfDocument {
    raw: CGPDFDocumentRef,
}

impl ScopedCgPdfDocument {
    pub fn open(path: &Path) -> Result<Self, String> {
        unsafe {
            let url = cfurl_from_path(path)
                .ok_or_else(|| format!("Invalid PDF path: {}", path.display()))?;
            let doc = CGPDFDocumentCreateWithURL(url);
            CFRelease(url);
            if doc.is_null() {
                Err(format!("Unable to open PDF file: {}", path.display()))
            } else {
                Ok(Self { raw: doc })
            }
        }
    }

    pub fn page_count(&self) -> usize {
        unsafe { CGPDFDocumentGetNumberOfPages(self.raw) }
    }

    pub fn page(&self, one_based_index: usize) -> Option<CGPDFPageRef> {
        unsafe {
            let p = CGPDFDocumentGetPage(self.raw, one_based_index);
            if p.is_null() {
                None
            } else {
                Some(p)
            }
        }
    }
}

impl Drop for ScopedCgPdfDocument {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                CGPDFDocumentRelease(self.raw);
            }
        }
    }
}
