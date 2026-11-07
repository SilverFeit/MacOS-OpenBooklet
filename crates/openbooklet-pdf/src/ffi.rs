//! Safe and minimal Rust FFI bindings to macOS `CoreFoundation`, `CoreGraphics`, and `CoreText`
//! for 100% vector PDF inspection, imposition rendering, and typography.

#![allow(non_camel_case_types, non_upper_case_globals, dead_code)]

use std::ffi::c_void;
use std::path::Path;
use std::ptr;

pub type CGFloat = f64;
pub type CFIndex = isize;
pub type CFTypeRef = *const c_void;
pub type CFAllocatorRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFURLRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFMutableAttributedStringRef = *mut c_void;
pub type CFAttributedStringRef = *const c_void;
pub type CGContextRef = *mut c_void;
pub type CGPDFDocumentRef = *mut c_void;
pub type CGPDFPageRef = *mut c_void;
pub type CGColorSpaceRef = *mut c_void;
pub type CGColorRef = *mut c_void;
pub type CTFontRef = *const c_void;
pub type CTLineRef = *const c_void;

pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

impl CGRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: CGPoint { x, y },
            size: CGSize { width, height },
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CFRange {
    pub location: CFIndex,
    pub length: CFIndex,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CGPDFBox {
    Media = 0,
    Crop = 1,
    Bleed = 2,
    Trim = 3,
    Art = 4,
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: u32,
        is_external_representation: u8,
    ) -> CFStringRef;
    pub fn CFURLCreateFromFileSystemRepresentation(
        allocator: CFAllocatorRef,
        buffer: *const u8,
        buf_len: CFIndex,
        is_directory: u8,
    ) -> CFURLRef;
    pub fn CFAttributedStringCreateMutable(
        alloc: CFAllocatorRef,
        max_length: CFIndex,
    ) -> CFMutableAttributedStringRef;
    pub fn CFAttributedStringReplaceString(
        a_str: CFMutableAttributedStringRef,
        range: CFRange,
        replacement: CFStringRef,
    );
    pub fn CFAttributedStringSetAttribute(
        a_str: CFMutableAttributedStringRef,
        range: CFRange,
        attr_name: CFStringRef,
        value: CFTypeRef,
    );
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGPDFDocumentCreateWithURL(url: CFURLRef) -> CGPDFDocumentRef;
    pub fn CGPDFDocumentRelease(document: CGPDFDocumentRef);
    pub fn CGPDFDocumentGetNumberOfPages(document: CGPDFDocumentRef) -> usize;
    pub fn CGPDFDocumentGetPage(document: CGPDFDocumentRef, page_number: usize) -> CGPDFPageRef;
    pub fn CGPDFPageGetBoxRect(page: CGPDFPageRef, box_type: CGPDFBox) -> CGRect;
    pub fn CGPDFPageGetRotationAngle(page: CGPDFPageRef) -> i32;

    pub fn CGPDFContextCreateWithURL(
        url: CFURLRef,
        media_box: *const CGRect,
        auxiliary_info: CFDictionaryRef,
    ) -> CGContextRef;
    pub fn CGPDFContextClose(context: CGContextRef);
    pub fn CGPDFContextBeginPage(context: CGContextRef, page_info: CFDictionaryRef);
    pub fn CGPDFContextEndPage(context: CGContextRef);

    pub fn CGContextRelease(c: CGContextRef);
    pub fn CGContextSaveGState(c: CGContextRef);
    pub fn CGContextRestoreGState(c: CGContextRef);
    pub fn CGContextTranslateCTM(c: CGContextRef, tx: CGFloat, ty: CGFloat);
    pub fn CGContextScaleCTM(c: CGContextRef, sx: CGFloat, sy: CGFloat);
    pub fn CGContextRotateCTM(c: CGContextRef, angle: CGFloat);
    pub fn CGContextClipToRect(c: CGContextRef, rect: CGRect);
    pub fn CGContextSetRGBFillColor(
        c: CGContextRef,
        red: CGFloat,
        green: CGFloat,
        blue: CGFloat,
        alpha: CGFloat,
    );
    pub fn CGContextSetRGBStrokeColor(
        c: CGContextRef,
        red: CGFloat,
        green: CGFloat,
        blue: CGFloat,
        alpha: CGFloat,
    );
    pub fn CGContextSetLineWidth(c: CGContextRef, width: CGFloat);
    pub fn CGContextSetLineDash(
        c: CGContextRef,
        phase: CGFloat,
        lengths: *const CGFloat,
        count: usize,
    );
    pub fn CGContextBeginPath(c: CGContextRef);
    pub fn CGContextMoveToPoint(c: CGContextRef, x: CGFloat, y: CGFloat);
    pub fn CGContextAddLineToPoint(c: CGContextRef, x: CGFloat, y: CGFloat);
    pub fn CGContextStrokePath(c: CGContextRef);
    pub fn CGContextFillRect(c: CGContextRef, rect: CGRect);
    pub fn CGContextStrokeRect(c: CGContextRef, rect: CGRect);
    pub fn CGContextStrokeEllipseInRect(c: CGContextRef, rect: CGRect);
    pub fn CGContextDrawPDFPage(c: CGContextRef, page: CGPDFPageRef);
    pub fn CGContextSetTextPosition(c: CGContextRef, x: CGFloat, y: CGFloat);

    pub fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
    pub fn CGColorSpaceRelease(space: CGColorSpaceRef);
    pub fn CGColorCreate(space: CGColorSpaceRef, components: *const CGFloat) -> CGColorRef;
    pub fn CGColorRelease(color: CGColorRef);
}

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    pub static kCTFontAttributeName: CFStringRef;
    pub static kCTForegroundColorAttributeName: CFStringRef;

    pub fn CTFontCreateWithName(
        name: CFStringRef,
        size: CGFloat,
        matrix: *const c_void,
    ) -> CTFontRef;
    pub fn CTLineCreateWithAttributedString(attr_string: CFAttributedStringRef) -> CTLineRef;
    pub fn CTLineDraw(line: CTLineRef, context: CGContextRef);
    pub fn CTLineGetTypographicBounds(
        line: CTLineRef,
        ascent: *mut CGFloat,
        descent: *mut CGFloat,
        leading: *mut CGFloat,
    ) -> f64;
}

/// Helper to create a `CFStringRef` from a Rust `&str`. Caller must call `CFRelease`.
pub unsafe fn cfstring_from_str(s: &str) -> CFStringRef {
    CFStringCreateWithBytes(
        ptr::null(),
        s.as_ptr(),
        s.len() as CFIndex,
        kCFStringEncodingUTF8,
        0,
    )
}

/// Helper to create a `CFURLRef` from a filesystem `Path`. Caller must call `CFRelease`.
pub unsafe fn cfurl_from_path(path: &Path) -> Option<CFURLRef> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    let url = CFURLCreateFromFileSystemRepresentation(
        ptr::null(),
        bytes.as_ptr(),
        bytes.len() as CFIndex,
        0,
    );
    if url.is_null() {
        None
    } else {
        Some(url)
    }
}

/// Draw a single line of vector text in `CGContextRef` at `(x, y)` using CoreText.
pub unsafe fn draw_vector_text(
    ctx: CGContextRef,
    text: &str,
    x: f64,
    y: f64,
    font_size: f64,
    bold: bool,
    rgb: (f64, f64, f64),
    center_horizontally: bool,
) {
    let font_name = if bold { "Helvetica-Bold" } else { "Helvetica" };
    let cf_font_name = cfstring_from_str(font_name);
    let font = CTFontCreateWithName(cf_font_name, font_size, ptr::null());
    let cf_text = cfstring_from_str(text);

    let attr_str = CFAttributedStringCreateMutable(ptr::null(), 0);
    CFAttributedStringReplaceString(attr_str, CFRange { location: 0, length: 0 }, cf_text);

    let full_range = CFRange {
        location: 0,
        length: text.encode_utf16().count() as CFIndex,
    };
    CFAttributedStringSetAttribute(attr_str, full_range, kCTFontAttributeName, font);

    let color_space = CGColorSpaceCreateDeviceRGB();
    let comps = [rgb.0, rgb.1, rgb.2, 1.0];
    let color = CGColorCreate(color_space, comps.as_ptr());
    CFAttributedStringSetAttribute(attr_str, full_range, kCTForegroundColorAttributeName, color);

    let line = CTLineCreateWithAttributedString(attr_str as CFAttributedStringRef);
    let width = CTLineGetTypographicBounds(line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
    let draw_x = if center_horizontally {
        x - width * 0.5
    } else {
        x
    };

    CGContextSaveGState(ctx);
    CGContextSetTextPosition(ctx, draw_x, y);
    CTLineDraw(line, ctx);
    CGContextRestoreGState(ctx);

    CFRelease(line);
    CGColorRelease(color);
    CGColorSpaceRelease(color_space);
    CFRelease(attr_str as CFTypeRef);
    CFRelease(cf_text);
    CFRelease(font);
    CFRelease(cf_font_name);
}
