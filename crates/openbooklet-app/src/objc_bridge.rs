//! Pure Rust Objective-C runtime (`libobjc.A.dylib`), `Foundation`, `AppKit`, and `PDFKit`
//! bridge for `aarch64-apple-darwin`.

#![allow(non_camel_case_types, non_upper_case_globals, dead_code)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

pub use openbooklet_pdf::ffi::{CGRect, CGSize};

pub type Id = *mut c_void;
pub type Class = *mut c_void;
pub type Sel = *mut c_void;
pub type Imp = *const c_void;
pub type NSUInteger = usize;
pub type NSInteger = isize;

pub const NIL: Id = ptr::null_mut();

// NSWindowStyleMask flags
pub const NSWindowStyleMaskTitled: NSUInteger = 1 << 0;
pub const NSWindowStyleMaskClosable: NSUInteger = 1 << 1;
pub const NSWindowStyleMaskMiniaturizable: NSUInteger = 1 << 2;
pub const NSWindowStyleMaskResizable: NSUInteger = 1 << 3;
pub const NSBackingStoreBuffered: NSUInteger = 2;
pub const NSApplicationActivationPolicyRegular: NSInteger = 0;

// NSButton / Control constants
pub const NSBezelStyleRounded: NSUInteger = 1;
pub const NSButtonTypeMomentaryPushIn: NSUInteger = 7;
pub const NSButtonTypeSwitch: NSUInteger = 3;
pub const NSControlStateValueOn: NSInteger = 1;
pub const NSControlStateValueOff: NSInteger = 0;

// NSView autoresizing mask
pub const NSViewWidthSizable: NSUInteger = 2;
pub const NSViewHeightSizable: NSUInteger = 16;
pub const NSViewMinXMargin: NSUInteger = 1;
pub const NSViewMaxXMargin: NSUInteger = 4;
pub const NSViewMinYMargin: NSUInteger = 8;
pub const NSViewMaxYMargin: NSUInteger = 32;

// PDFView display mode
pub const kPDFDisplaySinglePage: NSInteger = 0;
pub const kPDFDisplaySinglePageContinuous: NSInteger = 1;
pub const kPDFDisplayTwoUp: NSInteger = 2;
pub const kPDFDisplayTwoUpContinuous: NSInteger = 3;

#[link(name = "objc", kind = "dylib")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Class;
    pub fn sel_registerName(name: *const c_char) -> Sel;
    pub fn objc_allocateClassPair(
        superclass: Class,
        name: *const c_char,
        extra_bytes: usize,
    ) -> Class;
    pub fn objc_registerClassPair(cls: Class);
    pub fn class_addMethod(cls: Class, name: Sel, imp: Imp, types: *const c_char) -> bool;
    pub fn objc_msgSend();
}

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "PDFKit", kind = "framework")]
extern "C" {}

#[inline]
pub fn cls(name: &str) -> Class {
    let c = CString::new(name).unwrap();
    unsafe { objc_getClass(c.as_ptr()) }
}

#[inline]
pub fn sel(name: &str) -> Sel {
    let c = CString::new(name).unwrap();
    unsafe { sel_registerName(c.as_ptr()) }
}

// Typed `objc_msgSend` trampolines for Apple Silicon (`aarch64-apple-darwin`)
#[inline]
pub unsafe fn msg0<R>(recv: Id, s: Sel) -> R {
    let f: unsafe extern "C" fn(Id, Sel) -> R = std::mem::transmute(objc_msgSend as *const c_void);
    f(recv, s)
}

#[inline]
pub unsafe fn msg1<A, R>(recv: Id, s: Sel, a: A) -> R {
    let f: unsafe extern "C" fn(Id, Sel, A) -> R =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(recv, s, a)
}

#[inline]
pub unsafe fn msg2<A, B, R>(recv: Id, s: Sel, a: A, b: B) -> R {
    let f: unsafe extern "C" fn(Id, Sel, A, B) -> R =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(recv, s, a, b)
}

#[inline]
pub unsafe fn msg3<A, B, C, R>(recv: Id, s: Sel, a: A, b: B, c: C) -> R {
    let f: unsafe extern "C" fn(Id, Sel, A, B, C) -> R =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(recv, s, a, b, c)
}

#[inline]
pub unsafe fn msg4<A, B, C, D, R>(recv: Id, s: Sel, a: A, b: B, c: C, d: D) -> R {
    let f: unsafe extern "C" fn(Id, Sel, A, B, C, D) -> R =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(recv, s, a, b, c, d)
}

/// Convert a Rust `&str` to an autoreleased `NSString *`.
pub unsafe fn nsstring(s: &str) -> Id {
    let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    msg1(cls("NSString"), sel("stringWithUTF8String:"), c.as_ptr())
}

/// Convert an `NSString *` to a Rust `String`.
pub unsafe fn from_nsstring(ns: Id) -> String {
    if ns.is_null() {
        return String::new();
    }
    let ptr: *const c_char = msg0(ns, sel("UTF8String"));
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Create an `NSURL *` from a filesystem `Path`.
pub unsafe fn nsurl_from_path(p: &Path) -> Id {
    let s = p.to_string_lossy();
    let ns = nsstring(&s);
    msg1(cls("NSURL"), sel("fileURLWithPath:"), ns)
}

/// Convert an `NSURL *` to a `PathBuf`.
pub unsafe fn path_from_nsurl(url: Id) -> Option<PathBuf> {
    if url.is_null() {
        return None;
    }
    let path_ns: Id = msg0(url, sel("path"));
    let s = from_nsstring(path_ns);
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

/// Create a static `NSTextField` label.
pub unsafe fn make_label(text: &str, frame: CGRect, font_size: f64, bold: bool, secondary: bool) -> Id {
    let ns_txt = nsstring(text);
    let lbl: Id = msg1(cls("NSTextField"), sel("labelWithString:"), ns_txt);
    let _: () = msg1(lbl, sel("setFrame:"), frame);
    let font: Id = if bold {
        msg1(cls("NSFont"), sel("boldSystemFontOfSize:"), font_size)
    } else {
        msg1(cls("NSFont"), sel("systemFontOfSize:"), font_size)
    };
    let _: () = msg1(lbl, sel("setFont:"), font);
    if secondary {
        let color: Id = msg0(cls("NSColor"), sel("secondaryLabelColor"));
        let _: () = msg1(lbl, sel("setTextColor:"), color);
    }
    lbl
}

/// Create an editable `NSTextField` input box.
pub unsafe fn make_text_field(initial: &str, frame: CGRect, target: Id, action: Sel) -> Id {
    let tf_alloc: Id = msg0(cls("NSTextField"), sel("alloc"));
    let tf: Id = msg1(tf_alloc, sel("initWithFrame:"), frame);
    let ns_txt = nsstring(initial);
    let _: () = msg1(tf, sel("setStringValue:"), ns_txt);
    let font: Id = msg2(
        cls("NSFont"),
        sel("monospacedDigitSystemFontOfSize:weight:"),
        12.0f64,
        0.0f64,
    );
    if !font.is_null() {
        let _: () = msg1(tf, sel("setFont:"), font);
    }
    if !target.is_null() && !action.is_null() {
        let _: () = msg1(tf, sel("setTarget:"), target);
        let _: () = msg1(tf, sel("setAction:"), action);
    }
    tf
}

/// Create a standard rounded `NSButton`.
pub unsafe fn make_button(title: &str, frame: CGRect, target: Id, action: Sel) -> Id {
    let btn_alloc: Id = msg0(cls("NSButton"), sel("alloc"));
    let btn: Id = msg1(btn_alloc, sel("initWithFrame:"), frame);
    let ns_title = nsstring(title);
    let _: () = msg1(btn, sel("setTitle:"), ns_title);
    let _: () = msg1(btn, sel("setBezelStyle:"), NSBezelStyleRounded);
    let _: () = msg1(btn, sel("setButtonType:"), NSButtonTypeMomentaryPushIn);
    let _: () = msg1(btn, sel("setTarget:"), target);
    let _: () = msg1(btn, sel("setAction:"), action);
    btn
}

/// Create a checkbox `NSButton`.
pub unsafe fn make_checkbox(title: &str, frame: CGRect, checked: bool, target: Id, action: Sel) -> Id {
    let btn_alloc: Id = msg0(cls("NSButton"), sel("alloc"));
    let btn: Id = msg1(btn_alloc, sel("initWithFrame:"), frame);
    let ns_title = nsstring(title);
    let _: () = msg1(btn, sel("setTitle:"), ns_title);
    let _: () = msg1(btn, sel("setButtonType:"), NSButtonTypeSwitch);
    let state = if checked {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    };
    let _: () = msg1(btn, sel("setState:"), state);
    let _: () = msg1(btn, sel("setTarget:"), target);
    let _: () = msg1(btn, sel("setAction:"), action);
    btn
}

/// Create an `NSPopUpButton` dropdown menu with the given items.
pub unsafe fn make_popup(items: &[&str], selected_idx: usize, frame: CGRect, target: Id, action: Sel) -> Id {
    let pop_alloc: Id = msg0(cls("NSPopUpButton"), sel("alloc"));
    let pop: Id = msg2(pop_alloc, sel("initWithFrame:pullsDown:"), frame, false);
    for &item in items {
        let ns_item = nsstring(item);
        let _: () = msg1(pop, sel("addItemWithTitle:"), ns_item);
    }
    let _: () = msg1(pop, sel("selectItemAtIndex:"), selected_idx as NSInteger);
    let _: () = msg1(pop, sel("setTarget:"), target);
    let _: () = msg1(pop, sel("setAction:"), action);
    pop
}

/// Create a horizontal separator line (`NSBox`).
pub unsafe fn make_separator(frame: CGRect) -> Id {
    let box_alloc: Id = msg0(cls("NSBox"), sel("alloc"));
    let b: Id = msg1(box_alloc, sel("initWithFrame:"), frame);
    let _: () = msg1(b, sel("setBoxType:"), 2usize); // NSBoxSeparator = 2
    b
}
