//! Pure Rust Retina AppIcon.icns & Documentation Asset Generator using macOS CoreGraphics + ImageIO.

#![allow(non_camel_case_types, non_upper_case_globals)]

use std::env;
use std::ffi::c_void;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

type CGFloat = f64;
type CFIndex = isize;
type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFURLRef = *const c_void;
type CGContextRef = *mut c_void;
type CGColorSpaceRef = *mut c_void;
type CGImageRef = *mut c_void;
type CGImageDestinationRef = *mut c_void;
type CGPathRef = *const c_void;

const kCFStringEncodingUTF8: u32 = 0x0800_0100;
const kCGImageAlphaPremultipliedLast: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: CGFloat,
    y: CGFloat,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: CGFloat,
    height: CGFloat,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

impl CGRect {
    const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: CGPoint { x, y },
            size: CGSize { width, height },
        }
    }
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFStringCreateWithBytes(
        alloc: *const c_void,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: u32,
        is_external: u8,
    ) -> CFStringRef;
    fn CFURLCreateFromFileSystemRepresentation(
        alloc: *const c_void,
        buffer: *const u8,
        buf_len: CFIndex,
        is_dir: u8,
    ) -> CFURLRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
    fn CGColorSpaceRelease(cs: CGColorSpaceRef);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: CGColorSpaceRef,
        bitmap_info: u32,
    ) -> CGContextRef;
    fn CGBitmapContextCreateImage(context: CGContextRef) -> CGImageRef;
    fn CGContextRelease(c: CGContextRef);
    fn CGImageRelease(image: CGImageRef);

    fn CGContextSaveGState(c: CGContextRef);
    fn CGContextRestoreGState(c: CGContextRef);
    fn CGContextSetRGBFillColor(c: CGContextRef, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextSetRGBStrokeColor(c: CGContextRef, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextSetLineWidth(c: CGContextRef, w: CGFloat);
    fn CGContextFillRect(c: CGContextRef, rect: CGRect);
    fn CGContextStrokeEllipseInRect(c: CGContextRef, rect: CGRect);
    fn CGContextBeginPath(c: CGContextRef);
    fn CGContextMoveToPoint(c: CGContextRef, x: CGFloat, y: CGFloat);
    fn CGContextAddLineToPoint(c: CGContextRef, x: CGFloat, y: CGFloat);
    fn CGContextStrokePath(c: CGContextRef);
    fn CGContextAddPath(c: CGContextRef, path: CGPathRef);
    fn CGContextFillPath(c: CGContextRef);

    fn CGPathCreateWithRoundedRect(
        rect: CGRect,
        corner_width: CGFloat,
        corner_height: CGFloat,
        transform: *const c_void,
    ) -> CGPathRef;
    fn CGPathRelease(path: CGPathRef);
}

#[link(name = "ImageIO", kind = "framework")]
extern "C" {
    fn CGImageDestinationCreateWithURL(
        url: CFURLRef,
        img_type: CFStringRef,
        count: usize,
        options: *const c_void,
    ) -> CGImageDestinationRef;
    fn CGImageDestinationAddImage(
        dest: CGImageDestinationRef,
        image: CGImageRef,
        props: *const c_void,
    );
    fn CGImageDestinationFinalize(dest: CGImageDestinationRef) -> bool;
}

unsafe fn cfstring(s: &str) -> CFStringRef {
    CFStringCreateWithBytes(
        ptr::null(),
        s.as_ptr(),
        s.len() as CFIndex,
        kCFStringEncodingUTF8,
        0,
    )
}

unsafe fn save_png(ctx: CGContextRef, path: &Path) {
    let img = CGBitmapContextCreateImage(ctx);
    let bytes = path.as_os_str().as_bytes();
    let url = CFURLCreateFromFileSystemRepresentation(
        ptr::null(),
        bytes.as_ptr(),
        bytes.len() as CFIndex,
        0,
    );
    let uti = cfstring("public.png");
    let dest = CGImageDestinationCreateWithURL(url, uti, 1, ptr::null());
    if !dest.is_null() {
        CGImageDestinationAddImage(dest, img, ptr::null());
        let _ = CGImageDestinationFinalize(dest);
        CFRelease(dest as CFTypeRef);
    }
    CFRelease(uti);
    CFRelease(url);
    CGImageRelease(img);
}

unsafe fn fill_rounded_rect(
    ctx: CGContextRef,
    rect: CGRect,
    radius: f64,
    rgb: (f64, f64, f64, f64),
) {
    CGContextSaveGState(ctx);
    CGContextSetRGBFillColor(ctx, rgb.0, rgb.1, rgb.2, rgb.3);
    let p = CGPathCreateWithRoundedRect(rect, radius, radius, ptr::null());
    CGContextAddPath(ctx, p);
    CGContextFillPath(ctx);
    CGPathRelease(p);
    CGContextRestoreGState(ctx);
}

unsafe fn render_app_icon_png(size: usize, out_path: &Path) {
    let cs = CGColorSpaceCreateDeviceRGB();
    let ctx = CGBitmapContextCreate(
        ptr::null_mut(),
        size,
        size,
        8,
        size * 4,
        cs,
        kCGImageAlphaPremultipliedLast,
    );
    let s = size as f64;
    let pad = s * 0.08;
    let body = s - 2.0 * pad;
    let radius = body * 0.225;

    // Deep slate-indigo macOS squircle background
    fill_rounded_rect(
        ctx,
        CGRect::new(pad, pad, body, body),
        radius,
        (0.07, 0.11, 0.20, 1.0),
    );
    // Subtle inner highlight card
    fill_rounded_rect(
        ctx,
        CGRect::new(pad + s * 0.02, pad + s * 0.02, body - s * 0.04, body - s * 0.04),
        radius * 0.92,
        (0.10, 0.16, 0.29, 1.0),
    );

    // Draw open imposed signature book in the center
    let cx = s * 0.50;
    let cy = s * 0.51;
    let bw = s * 0.56;
    let bh = s * 0.44;

    // Back signature sheet shadow (cyan/sky accent)
    fill_rounded_rect(
        ctx,
        CGRect::new(cx - bw * 0.53, cy - bh * 0.48, bw * 1.06, bh * 0.96),
        s * 0.03,
        (0.06, 0.65, 0.91, 0.95),
    );

    // Left & Right open folio pages (warm ivory white)
    let page_w = bw * 0.48;
    fill_rounded_rect(
        ctx,
        CGRect::new(cx - page_w - s * 0.008, cy - bh * 0.45, page_w, bh * 0.92),
        s * 0.02,
        (0.98, 0.99, 1.0, 1.0),
    );
    fill_rounded_rect(
        ctx,
        CGRect::new(cx + s * 0.008, cy - bh * 0.45, page_w, bh * 0.92),
        s * 0.02,
        (0.95, 0.97, 1.0, 1.0),
    );

    // Central spine & stepped collating marks
    CGContextSetRGBFillColor(ctx, 0.12, 0.18, 0.30, 1.0);
    CGContextFillRect(ctx, CGRect::new(cx - s * 0.012, cy - bh * 0.46, s * 0.024, bh * 0.94));

    // Stepped amber/emerald collating blocks on the spine
    for i in 0..3 {
        let mark_y = cy + bh * 0.22 - (i as f64) * (s * 0.055);
        CGContextSetRGBFillColor(ctx, 0.98, 0.45, 0.15, 1.0);
        CGContextFillRect(ctx, CGRect::new(cx - s * 0.018, mark_y, s * 0.036, s * 0.038));
    }

    // Precision prepress crop marks & registration target at corners
    CGContextSetRGBStrokeColor(ctx, 0.38, 0.82, 0.98, 0.95);
    CGContextSetLineWidth(ctx, (s * 0.014).max(1.0));
    let m_len = s * 0.06;
    let x0 = cx - bw * 0.56;
    let x1 = cx + bw * 0.56;
    let y0 = cy - bh * 0.52;
    let y1 = cy + bh * 0.52;

    CGContextBeginPath(ctx);
    // Top-left crop mark
    CGContextMoveToPoint(ctx, x0 - m_len, y1);
    CGContextAddLineToPoint(ctx, x0, y1);
    CGContextMoveToPoint(ctx, x0, y1);
    CGContextAddLineToPoint(ctx, x0, y1 + m_len);
    // Top-right crop mark
    CGContextMoveToPoint(ctx, x1, y1);
    CGContextAddLineToPoint(ctx, x1 + m_len, y1);
    CGContextMoveToPoint(ctx, x1, y1);
    CGContextAddLineToPoint(ctx, x1, y1 + m_len);
    // Bottom-left crop mark
    CGContextMoveToPoint(ctx, x0 - m_len, y0);
    CGContextAddLineToPoint(ctx, x0, y0);
    CGContextMoveToPoint(ctx, x0, y0 - m_len);
    CGContextAddLineToPoint(ctx, x0, y0);
    // Bottom-right crop mark
    CGContextMoveToPoint(ctx, x1, y0);
    CGContextAddLineToPoint(ctx, x1 + m_len, y0);
    CGContextMoveToPoint(ctx, x1, y0 - m_len);
    CGContextAddLineToPoint(ctx, x1, y0);
    CGContextStrokePath(ctx);

    // Registration target at bottom center
    let reg_y = pad + s * 0.11;
    let reg_r = s * 0.038;
    CGContextStrokeEllipseInRect(
        ctx,
        CGRect::new(cx - reg_r, reg_y - reg_r, reg_r * 2.0, reg_r * 2.0),
    );
    CGContextBeginPath(ctx);
    CGContextMoveToPoint(ctx, cx - reg_r * 1.45, reg_y);
    CGContextAddLineToPoint(ctx, cx + reg_r * 1.45, reg_y);
    CGContextMoveToPoint(ctx, cx, reg_y - reg_r * 1.45);
    CGContextAddLineToPoint(ctx, cx, reg_y + reg_r * 1.45);
    CGContextStrokePath(ctx);

    save_png(ctx, out_path);
    CGContextRelease(ctx);
    CGColorSpaceRelease(cs);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let iconset_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/OpenBookletIcon.iconset"));
    let _ = fs::create_dir_all(&iconset_dir);

    let specs = [
        ("icon_16x16.png", 16usize),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ];

    unsafe {
        for (name, px) in specs {
            render_app_icon_png(px, &iconset_dir.join(name));
        }
        if let Some(assets_dir) = args.get(2).map(PathBuf::from) {
            let _ = fs::create_dir_all(&assets_dir);
            render_app_icon_png(512, &assets_dir.join("app-icon.png"));
        }
    }
}
