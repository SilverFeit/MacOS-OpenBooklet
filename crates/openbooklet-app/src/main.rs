//! `OpenBooklet.app` — Open-Source Native macOS AppKit & PDFKit GUI application written in pure Rust.

mod objc_bridge;

use std::cell::RefCell;
use std::env;
use std::ffi::{c_void, CString};
use std::path::PathBuf;

use objc_bridge::*;
use openbooklet_core::{
    ImpositionMode, JobConfig, PageNumberPosition, PaperFormat, PrepressBoxMode,
};
use openbooklet_pdf::{
    generate_duplex_calibration_pdf, generate_sample_document_pdf, inspect_pdf_file,
    render_imposed_pdf, RenderSummary,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_URL: &str = "https://github.com/SilverFeit/MacOS-OpenBooklet";

struct AppState {
    window: Id,
    pdf_view: Id,
    file_badge_label: Id,
    sheet_nav_label: Id,
    preflight_label: Id,

    // Controls
    mode_popup: Id,
    paper_popup: Id,
    orientation_popup: Id,
    pdf_box_popup: Id,
    folios_popup: Id,
    flyleaves_popup: Id,
    creep_field: Id,
    duplex_x_field: Id,
    duplex_y_field: Id,
    range_field: Id,
    scale_field: Id,
    gutter_field: Id,
    margin_field: Id,
    rotation_popup: Id,
    page_num_popup: Id,

    rtl_check: Id,
    calendar_check: Id,
    auto_scale_check: Id,
    flip_h_check: Id,
    flip_v_check: Id,
    fold_guide_check: Id,
    crop_marks_check: Id,
    reg_marks_check: Id,
    collating_marks_check: Id,
    tech_slug_check: Id,

    // Data
    input_pdf_path: PathBuf,
    preview_pdf_path: PathBuf,
    last_summary: Option<RenderSummary>,
}

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

const MODES: [ImpositionMode; 12] = [
    ImpositionMode::Booklet,
    ImpositionMode::Signatures,
    ImpositionMode::CutAndStack2Up,
    ImpositionMode::CutAndStack4Up,
    ImpositionMode::DeimposeBooklet,
    ImpositionMode::Split2Up,
    ImpositionMode::TwoUp,
    ImpositionMode::FourUpRows,
    ImpositionMode::FourUpCols,
    ImpositionMode::NCopies2,
    ImpositionMode::NCopies4,
    ImpositionMode::OneUp,
];

const PAPERS: [PaperFormat; 7] = [
    PaperFormat::A4,
    PaperFormat::Letter,
    PaperFormat::Legal,
    PaperFormat::A3,
    PaperFormat::Tabloid,
    PaperFormat::Sra3,
    PaperFormat::A5,
];

fn main() {
    unsafe {
        let app: Id = msg0(cls("NSApplication"), sel("sharedApplication"));
        let _: bool = msg1(
            app,
            sel("setActivationPolicy:"),
            NSApplicationActivationPolicyRegular,
        );

        let _ = register_flipped_view_class();
        let delegate_cls = register_delegate_class();
        let delegate_alloc: Id = msg0(delegate_cls, sel("alloc"));
        let delegate: Id = msg0(delegate_alloc, sel("init"));
        let _: () = msg1(app, sel("setDelegate:"), delegate);

        setup_main_menu(app, delegate);
        build_main_window(delegate);

        let _: () = msg1(app, sel("activateIgnoringOtherApps:"), true);
        let _: () = msg0(app, sel("run"));
    }
}

/// Register a custom `NSView` subclass whose `isFlipped` returns `true` so `NSScrollView`
/// uses top-to-bottom coordinates (`y = 0` at top) and starts scrolled to the top.
unsafe fn register_flipped_view_class() -> Class {
    let name = CString::new("OpenBookletFlippedView").unwrap();
    let existing = objc_getClass(name.as_ptr());
    if !existing.is_null() {
        return existing;
    }

    let superclass = cls("NSView");
    let new_cls = objc_allocateClassPair(superclass, name.as_ptr(), 0);
    let bool_types = CString::new("B@:").unwrap();
    class_addMethod(
        new_cls,
        sel("isFlipped"),
        flipped_view_is_flipped as *const c_void,
        bool_types.as_ptr(),
    );
    objc_registerClassPair(new_cls);
    new_cls
}

extern "C" fn flipped_view_is_flipped(_this: Id, _cmd: Sel) -> bool {
    true
}

unsafe fn register_delegate_class() -> Class {
    let name = CString::new("OpenBookletAppDelegate").unwrap();
    let existing = objc_getClass(name.as_ptr());
    if !existing.is_null() {
        return existing;
    }

    let superclass = cls("NSObject");
    let new_cls = objc_allocateClassPair(superclass, name.as_ptr(), 0);
    let void_id_types = CString::new("v@:@").unwrap();
    let bool_id_types = CString::new("B@:@").unwrap();

    class_addMethod(
        new_cls,
        sel("onControlChanged:"),
        on_control_changed as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onOpenPdf:"),
        on_open_pdf as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onLoadSample:"),
        on_load_sample as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onExportPdf:"),
        on_export_pdf as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onExportCalibration:"),
        on_export_calibration as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onPrintPdf:"),
        on_print_pdf as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onResetDefaults:"),
        on_reset_defaults as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onPrevSide:"),
        on_prev_side as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onNextSide:"),
        on_next_side as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onZoomIn:"),
        on_zoom_in as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onZoomOut:"),
        on_zoom_out as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onZoomFit:"),
        on_zoom_fit as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("onOpenGitHub:"),
        on_open_github as *const c_void,
        void_id_types.as_ptr(),
    );
    class_addMethod(
        new_cls,
        sel("applicationShouldTerminateAfterLastWindowClosed:"),
        app_should_terminate as *const c_void,
        bool_id_types.as_ptr(),
    );

    objc_registerClassPair(new_cls);
    new_cls
}

extern "C" fn app_should_terminate(_this: Id, _cmd: Sel, _app: Id) -> bool {
    true
}

extern "C" fn on_control_changed(_this: Id, _cmd: Sel, _sender: Id) {
    refresh_preview();
}

extern "C" fn on_open_pdf(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        let panel: Id = msg0(cls("NSOpenPanel"), sel("openPanel"));
        let _: () = msg1(panel, sel("setCanChooseFiles:"), true);
        let _: () = msg1(panel, sel("setCanChooseDirectories:"), false);
        let _: () = msg1(panel, sel("setAllowsMultipleSelection:"), false);
        let response: NSInteger = msg0(panel, sel("runModal"));
        if response == 1 {
            let url: Id = msg0(panel, sel("URL"));
            if let Some(path) = path_from_nsurl(url) {
                STATE.with(|cell| {
                    if let Some(st) = cell.borrow_mut().as_mut() {
                        st.input_pdf_path = path;
                    }
                });
                refresh_preview();
            }
        }
    }
}

extern "C" fn on_load_sample(_this: Id, _cmd: Sel, _sender: Id) {
    let sample_path = std::env::temp_dir().join("openbooklet_demo_16pp.pdf");
    if generate_sample_document_pdf(&sample_path, 16, PaperFormat::A4).is_ok() {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow_mut().as_mut() {
                st.input_pdf_path = sample_path;
            }
        });
        refresh_preview();
    }
}

extern "C" fn on_export_pdf(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        let panel: Id = msg0(cls("NSSavePanel"), sel("savePanel"));
        let default_name = nsstring("Imposed-OpenBooklet.pdf");
        let _: () = msg1(panel, sel("setNameFieldStringValue:"), default_name);
        let response: NSInteger = msg0(panel, sel("runModal"));
        if response == 1 {
            let url: Id = msg0(panel, sel("URL"));
            if let Some(dest) = path_from_nsurl(url) {
                STATE.with(|cell| {
                    if let Some(st) = cell.borrow().as_ref() {
                        let cfg = read_config_from_ui(st);
                        let _ = render_imposed_pdf(&st.input_pdf_path, &dest, &cfg);
                    }
                });
            }
        }
    }
}

extern "C" fn on_export_calibration(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        let panel: Id = msg0(cls("NSSavePanel"), sel("savePanel"));
        let default_name = nsstring("OpenBooklet-Duplex-Calibration-Sheet.pdf");
        let _: () = msg1(panel, sel("setNameFieldStringValue:"), default_name);
        let response: NSInteger = msg0(panel, sel("runModal"));
        if response == 1 {
            let url: Id = msg0(panel, sel("URL"));
            if let Some(dest) = path_from_nsurl(url) {
                let mut paper = PaperFormat::A4;
                let mut landscape = true;
                STATE.with(|cell| {
                    if let Some(st) = cell.borrow().as_ref() {
                        let cfg = read_config_from_ui(st);
                        paper = cfg.paper;
                        landscape = cfg.is_landscape();
                    }
                });
                if generate_duplex_calibration_pdf(&dest, paper, landscape).is_ok() {
                    let ws: Id = msg0(cls("NSWorkspace"), sel("sharedWorkspace"));
                    let ns_url = nsurl_from_path(&dest);
                    let _: bool = msg1(ws, sel("openURL:"), ns_url);
                }
            }
        }
    }
}

extern "C" fn on_print_pdf(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let doc: Id = msg0(st.pdf_view, sel("document"));
                if !doc.is_null() {
                    let print_info: Id = msg0(cls("NSPrintInfo"), sel("sharedPrintInfo"));
                    let op: Id = msg3(
                        doc,
                        sel("printOperationForPrintInfo:scalingMode:autoRotate:"),
                        print_info,
                        0isize, // kPDFPrintPageScaleNone
                        true,
                    );
                    if !op.is_null() {
                        let _: bool = msg0(op, sel("runOperation"));
                    }
                }
            }
        });
    }
}

extern "C" fn on_reset_defaults(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let _: () = msg1(st.mode_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.paper_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.orientation_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.pdf_box_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.folios_popup, sel("selectItemAtIndex:"), 2isize);
                let _: () = msg1(st.flyleaves_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.creep_field, sel("setStringValue:"), nsstring("0.0"));
                let _: () = msg1(st.duplex_x_field, sel("setStringValue:"), nsstring("0.0"));
                let _: () = msg1(st.duplex_y_field, sel("setStringValue:"), nsstring("0.0"));
                let _: () = msg1(st.range_field, sel("setStringValue:"), nsstring("all"));
                let _: () = msg1(st.scale_field, sel("setStringValue:"), nsstring("100"));
                let _: () = msg1(st.gutter_field, sel("setStringValue:"), nsstring("0.0"));
                let _: () = msg1(st.margin_field, sel("setStringValue:"), nsstring("0.0"));
                let _: () = msg1(st.rotation_popup, sel("selectItemAtIndex:"), 0isize);
                let _: () = msg1(st.page_num_popup, sel("selectItemAtIndex:"), 0isize);

                let _: () = msg1(st.rtl_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.calendar_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.auto_scale_check, sel("setState:"), NSControlStateValueOn);
                let _: () = msg1(st.flip_h_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.flip_v_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.fold_guide_check, sel("setState:"), NSControlStateValueOn);
                let _: () = msg1(st.crop_marks_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.reg_marks_check, sel("setState:"), NSControlStateValueOff);
                let _: () =
                    msg1(st.collating_marks_check, sel("setState:"), NSControlStateValueOff);
                let _: () = msg1(st.tech_slug_check, sel("setState:"), NSControlStateValueOff);
            }
        });
    }
    refresh_preview();
}

extern "C" fn on_prev_side(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let can: bool = msg0(st.pdf_view, sel("canGoToPreviousPage"));
                if can {
                    let _: () = msg1(st.pdf_view, sel("goToPreviousPage:"), NIL);
                    update_navigation_badge(st);
                }
            }
        });
    }
}

extern "C" fn on_next_side(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let can: bool = msg0(st.pdf_view, sel("canGoToNextPage"));
                if can {
                    let _: () = msg1(st.pdf_view, sel("goToNextPage:"), NIL);
                    update_navigation_badge(st);
                }
            }
        });
    }
}

extern "C" fn on_zoom_in(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let _: () = msg1(st.pdf_view, sel("zoomIn:"), NIL);
            }
        });
    }
}

extern "C" fn on_zoom_out(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let _: () = msg1(st.pdf_view, sel("zoomOut:"), NIL);
            }
        });
    }
}

extern "C" fn on_zoom_fit(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        STATE.with(|cell| {
            if let Some(st) = cell.borrow().as_ref() {
                let _: () = msg1(st.pdf_view, sel("setAutoScales:"), true);
            }
        });
    }
}

extern "C" fn on_open_github(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        let ws: Id = msg0(cls("NSWorkspace"), sel("sharedWorkspace"));
        let ns_str = nsstring(GITHUB_URL);
        let url: Id = msg1(cls("NSURL"), sel("URLWithString:"), ns_str);
        if !url.is_null() {
            let _: bool = msg1(ws, sel("openURL:"), url);
        }
    }
}

unsafe fn setup_main_menu(app: Id, target: Id) {
    let menubar_alloc: Id = msg0(cls("NSMenu"), sel("alloc"));
    let menubar: Id = msg0(menubar_alloc, sel("init"));

    let app_menu_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let app_menu_item: Id = msg0(app_menu_item_alloc, sel("init"));
    let _: () = msg1(menubar, sel("addItem:"), app_menu_item);

    let app_menu_alloc: Id = msg0(cls("NSMenu"), sel("alloc"));
    let app_menu: Id = msg1(app_menu_alloc, sel("initWithTitle:"), nsstring("OpenBooklet"));

    let open_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let open_item: Id = msg3(
        open_item_alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        nsstring("Open PDF…"),
        sel("onOpenPdf:"),
        nsstring("o"),
    );
    let _: () = msg1(open_item, sel("setTarget:"), target);
    let _: () = msg1(app_menu, sel("addItem:"), open_item);

    let save_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let save_item: Id = msg3(
        save_item_alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        nsstring("Export Imposed PDF…"),
        sel("onExportPdf:"),
        nsstring("s"),
    );
    let _: () = msg1(save_item, sel("setTarget:"), target);
    let _: () = msg1(app_menu, sel("addItem:"), save_item);

    let cal_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let cal_item: Id = msg3(
        cal_item_alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        nsstring("Export Duplex Calibration Sheet…"),
        sel("onExportCalibration:"),
        nsstring("k"),
    );
    let _: () = msg1(cal_item, sel("setTarget:"), target);
    let _: () = msg1(app_menu, sel("addItem:"), cal_item);

    let print_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let print_item: Id = msg3(
        print_item_alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        nsstring("Print Imposed Sheets…"),
        sel("onPrintPdf:"),
        nsstring("p"),
    );
    let _: () = msg1(print_item, sel("setTarget:"), target);
    let _: () = msg1(app_menu, sel("addItem:"), print_item);

    let quit_item_alloc: Id = msg0(cls("NSMenuItem"), sel("alloc"));
    let quit_item: Id = msg3(
        quit_item_alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        nsstring("Quit OpenBooklet"),
        sel("terminate:"),
        nsstring("q"),
    );
    let _: () = msg1(app_menu, sel("addItem:"), quit_item);

    let _: () = msg1(app_menu_item, sel("setSubmenu:"), app_menu);
    let _: () = msg1(app, sel("setMainMenu:"), menubar);
}

unsafe fn build_main_window(target: Id) {
    let win_w = 1200.0;
    let win_h = 740.0;
    let style = NSWindowStyleMaskTitled
        | NSWindowStyleMaskClosable
        | NSWindowStyleMaskMiniaturizable
        | NSWindowStyleMaskResizable;

    let win_alloc: Id = msg0(cls("NSWindow"), sel("alloc"));
    let window: Id = msg4(
        win_alloc,
        sel("initWithContentRect:styleMask:backing:defer:"),
        CGRect::new(80.0, 60.0, win_w, win_h),
        style,
        NSBackingStoreBuffered,
        false,
    );
    let title = format!(
        "OpenBooklet v{APP_VERSION} — Open-Source Vector PDF Booklet, Bookbinding & Prepress Studio"
    );
    let _: () = msg1(window, sel("setTitle:"), nsstring(&title));
    let _: () = msg1(
        window,
        sel("setMinSize:"),
        CGSize {
            width: 920.0,
            height: 520.0,
        },
    );

    let content_view: Id = msg0(window, sel("contentView"));
    let action_sel = sel("onControlChanged:");

    let sidebar_w = 408.0;
    let footer_h = 86.0;
    let pad = 16.0;
    // Leave 16 pt on the right inside the scroll document so the vertical scrollbar never overlaps controls
    let doc_w = sidebar_w - 16.0;
    let ctrl_w = doc_w - 2.0 * pad;

    // --- LEFT SCROLLABLE SIDEBAR (`NSScrollView` + Flipped `OpenBookletFlippedView`) ---
    let scroll_alloc: Id = msg0(cls("NSScrollView"), sel("alloc"));
    let scroll_view: Id = msg1(
        scroll_alloc,
        sel("initWithFrame:"),
        CGRect::new(0.0, footer_h, sidebar_w, win_h - footer_h),
    );
    let _: () = msg1(scroll_view, sel("setHasVerticalScroller:"), true);
    let _: () = msg1(scroll_view, sel("setHasHorizontalScroller:"), false);
    let _: () = msg1(scroll_view, sel("setAutohidesScrollers:"), false);
    let _: () = msg1(scroll_view, sel("setBorderType:"), 0usize); // NSNoBorder
    let _: () = msg1(scroll_view, sel("setDrawsBackground:"), false);
    let _: () = msg1(
        scroll_view,
        sel("setAutoresizingMask:"),
        NSViewHeightSizable | NSViewMaxXMargin,
    );

    let doc_h = 680.0;
    let flipped_cls = cls("OpenBookletFlippedView");
    let doc_alloc: Id = msg0(flipped_cls, sel("alloc"));
    let doc_view: Id = msg1(
        doc_alloc,
        sel("initWithFrame:"),
        CGRect::new(0.0, 0.0, doc_w, doc_h),
    );

    // Because `doc_view` is flipped (`isFlipped == true`), `y = 0` is at the TOP and increases downward!
    let mut y = 14.0;

    // 1. Source Document
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "1. SOURCE DOCUMENT",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let open_btn = make_button(
        "Open PDF…",
        CGRect::new(pad, y, (ctrl_w - 8.0) * 0.50, 28.0),
        target,
        sel("onOpenPdf:"),
    );
    let sample_btn = make_button(
        "Load 16-Page Demo",
        CGRect::new(
            pad + (ctrl_w - 8.0) * 0.50 + 8.0,
            y,
            (ctrl_w - 8.0) * 0.50,
            28.0,
        ),
        target,
        sel("onLoadSample:"),
    );
    let _: () = msg1(doc_view, sel("addSubview:"), open_btn);
    let _: () = msg1(doc_view, sel("addSubview:"), sample_btn);
    y += 32.0;

    let file_badge_label = make_label(
        "Ready — 16-page demo loaded",
        CGRect::new(pad, y, ctrl_w, 16.0),
        11.0,
        false,
        true,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), file_badge_label);
    y += 22.0;
    let _: () = msg1(
        doc_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, y, ctrl_w, 4.0)),
    );
    y += 12.0;

    // 2. Imposition Mode, Paper & Prepress Box
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "2. IMPOSITION MODE & PRESS SHEET",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let mode_titles = [
        "Single Booklet (Saddle-Stitch)",
        "Multi-Signature Book (Sewn / Glued)",
        "Guillotine Cut & Stack (2-Up)",
        "Guillotine Cut & Stack (4-Up)",
        "De-Impose Booklet -> Linear PDF",
        "Split 2-Up Spreads -> Single Pages",
        "2-Up Sequential Side-by-Side",
        "4-Up Grid (2x2 by Rows)",
        "4-Up Grid (2x2 by Columns)",
        "Step & Repeat (2 Identical Copies)",
        "Step & Repeat (4 Identical Copies)",
        "1-Up Prepress Frame & Marks",
    ];
    let mode_popup = make_popup(
        &mode_titles,
        0,
        CGRect::new(pad, y, ctrl_w, 25.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), mode_popup);
    y += 32.0;

    let paper_titles = [
        "A4 (210 × 297 mm)",
        "US Letter (215.9 × 279.4 mm)",
        "US Legal (215.9 × 355.6 mm)",
        "A3 (297 × 420 mm)",
        "Tabloid / Ledger (11 × 17 in)",
        "SRA3 Digital Press (320 × 450 mm)",
        "A5 (148 × 210 mm)",
    ];
    let paper_popup = make_popup(
        &paper_titles,
        0,
        CGRect::new(pad, y, (ctrl_w - 8.0) * 0.58, 25.0),
        target,
        action_sel,
    );
    let orient_titles = ["Auto Orientation", "Landscape", "Portrait"];
    let orientation_popup = make_popup(
        &orient_titles,
        0,
        CGRect::new(
            pad + (ctrl_w - 8.0) * 0.58 + 8.0,
            y,
            (ctrl_w - 8.0) * 0.42,
            25.0,
        ),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), paper_popup);
    let _: () = msg1(doc_view, sel("addSubview:"), orientation_popup);
    y += 32.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "PDF Box:",
            CGRect::new(pad, y + 4.0, 68.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let box_titles = [
        "Auto (TrimBox → CropBox → MediaBox)",
        "MediaBox (Full Uncropped Sheet)",
        "CropBox",
        "TrimBox (Finished Cut Size)",
        "BleedBox (Include Print Bleed)",
    ];
    let pdf_box_popup = make_popup(
        &box_titles,
        0,
        CGRect::new(pad + 70.0, y, ctrl_w - 70.0, 25.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), pdf_box_popup);
    y += 32.0;
    let _: () = msg1(
        doc_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, y, ctrl_w, 4.0)),
    );
    y += 12.0;

    // 3. Bookbinding Signatures, Creep & Flyleaves
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "3. SIGNATURES, CREEP & FLYLEAVES",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Signature:",
            CGRect::new(pad, y + 4.0, 68.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let folio_titles = [
        "1 Sheet (4 pp / sig)",
        "2 Sheets (8 pp / sig)",
        "4 Sheets (16 pp / sig)",
        "5 Sheets (20 pp / sig)",
        "6 Sheets (24 pp / sig)",
        "8 Sheets (32 pp / sig)",
    ];
    let folios_popup = make_popup(
        &folio_titles,
        2,
        CGRect::new(pad + 70.0, y, 160.0, 25.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), folios_popup);

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Creep (mm):",
            CGRect::new(pad + 236.0, y + 4.0, 74.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let creep_field = make_text_field(
        "0.0",
        CGRect::new(pad + 310.0, y + 1.0, ctrl_w - 310.0, 22.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), creep_field);
    y += 32.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Flyleaves:",
            CGRect::new(pad, y + 4.0, 68.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let flyleaf_titles = [
        "None (0 blank endpapers)",
        "+2 pp Front & Back (1 leaf each)",
        "+4 pp Front & Back (2 leaves each)",
        "+8 pp Front & Back (4 leaves each)",
    ];
    let flyleaves_popup = make_popup(
        &flyleaf_titles,
        0,
        CGRect::new(pad + 70.0, y, ctrl_w - 70.0, 25.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), flyleaves_popup);
    y += 30.0;

    let rtl_check = make_checkbox(
        "Right-to-Left (Manga / Arabic)",
        CGRect::new(pad, y, 195.0, 20.0),
        false,
        target,
        action_sel,
    );
    let calendar_check = make_checkbox(
        "Calendar Flip (Verso 180°)",
        CGRect::new(pad + 195.0, y, ctrl_w - 195.0, 20.0),
        false,
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), rtl_check);
    let _: () = msg1(doc_view, sel("addSubview:"), calendar_check);
    y += 28.0;
    let _: () = msg1(
        doc_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, y, ctrl_w, 4.0)),
    );
    y += 12.0;

    // 4. Duplex Printer Calibration (Verso X/Y mm + Printable Calibration Sheet)
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "4. DUPLEX PRINTER CALIBRATION (VERSO X/Y)",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Verso dX (mm):",
            CGRect::new(pad, y + 3.0, 92.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let duplex_x_field = make_text_field(
        "0.0",
        CGRect::new(pad + 94.0, y + 1.0, 68.0, 22.0),
        target,
        action_sel,
    );
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Verso dY (mm):",
            CGRect::new(pad + 176.0, y + 3.0, 92.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let duplex_y_field = make_text_field(
        "0.0",
        CGRect::new(pad + 270.0, y + 1.0, ctrl_w - 270.0, 22.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), duplex_x_field);
    let _: () = msg1(doc_view, sel("addSubview:"), duplex_y_field);
    y += 30.0;

    let cal_btn = make_button(
        "Export Printable Duplex Calibration Sheet (PDF)…",
        CGRect::new(pad, y, ctrl_w, 26.0),
        target,
        sel("onExportCalibration:"),
    );
    let _: () = msg1(doc_view, sel("addSubview:"), cal_btn);
    y += 34.0;
    let _: () = msg1(
        doc_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, y, ctrl_w, 4.0)),
    );
    y += 12.0;

    // 5. Scaling, Range, Margins & Transforms
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "5. RANGE, SCALING, MARGINS & TRANSFORMS",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Range:",
            CGRect::new(pad, y + 3.0, 48.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let range_field = make_text_field(
        "all",
        CGRect::new(pad + 50.0, y + 1.0, 118.0, 22.0),
        target,
        action_sel,
    );
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Scale (%):",
            CGRect::new(pad + 176.0, y + 3.0, 64.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let scale_field = make_text_field(
        "100",
        CGRect::new(pad + 242.0, y + 1.0, 48.0, 22.0),
        target,
        action_sel,
    );
    let auto_scale_check = make_checkbox(
        "Fit",
        CGRect::new(pad + 296.0, y + 2.0, 60.0, 20.0),
        true,
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), range_field);
    let _: () = msg1(doc_view, sel("addSubview:"), scale_field);
    let _: () = msg1(doc_view, sel("addSubview:"), auto_scale_check);
    y += 30.0;

    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Gutter (mm):",
            CGRect::new(pad, y + 3.0, 78.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let gutter_field = make_text_field(
        "0.0",
        CGRect::new(pad + 80.0, y + 1.0, 62.0, 22.0),
        target,
        action_sel,
    );
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "Margin (mm):",
            CGRect::new(pad + 154.0, y + 3.0, 82.0, 18.0),
            11.5,
            false,
            false,
        ),
    );
    let margin_field = make_text_field(
        "0.0",
        CGRect::new(pad + 238.0, y + 1.0, ctrl_w - 238.0, 22.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), gutter_field);
    let _: () = msg1(doc_view, sel("addSubview:"), margin_field);
    y += 30.0;

    let rot_titles = ["Rotate 0°", "Rotate 90°", "Rotate 180°", "Rotate 270°"];
    let rotation_popup = make_popup(
        &rot_titles,
        0,
        CGRect::new(pad, y, 130.0, 25.0),
        target,
        action_sel,
    );
    let flip_h_check = make_checkbox(
        "Mirror H",
        CGRect::new(pad + 140.0, y + 2.0, 95.0, 20.0),
        false,
        target,
        action_sel,
    );
    let flip_v_check = make_checkbox(
        "Mirror V",
        CGRect::new(pad + 240.0, y + 2.0, 95.0, 20.0),
        false,
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), rotation_popup);
    let _: () = msg1(doc_view, sel("addSubview:"), flip_h_check);
    let _: () = msg1(doc_view, sel("addSubview:"), flip_v_check);
    y += 32.0;
    let _: () = msg1(
        doc_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, y, ctrl_w, 4.0)),
    );
    y += 12.0;

    // 6. Prepress Marks & Page Numbering
    let _ = msg1::<Id, ()>(
        doc_view,
        sel("addSubview:"),
        make_label(
            "6. PREPRESS MARKS, COLLATING & FOLIOS",
            CGRect::new(pad, y, ctrl_w, 16.0),
            11.0,
            true,
            true,
        ),
    );
    y += 22.0;

    let fold_guide_check = make_checkbox(
        "Fold / Guillotine Line",
        CGRect::new(pad, y, 175.0, 20.0),
        true,
        target,
        action_sel,
    );
    let crop_marks_check = make_checkbox(
        "Corner Crop Marks",
        CGRect::new(pad + 180.0, y, 175.0, 20.0),
        false,
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), fold_guide_check);
    let _: () = msg1(doc_view, sel("addSubview:"), crop_marks_check);
    y += 24.0;

    let reg_marks_check = make_checkbox(
        "Registration Targets",
        CGRect::new(pad, y, 175.0, 20.0),
        false,
        target,
        action_sel,
    );
    let collating_marks_check = make_checkbox(
        "Spine Collating Marks",
        CGRect::new(pad + 180.0, y, 175.0, 20.0),
        false,
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), reg_marks_check);
    let _: () = msg1(doc_view, sel("addSubview:"), collating_marks_check);
    y += 26.0;

    let tech_slug_check = make_checkbox(
        "Technical Sheet Slug",
        CGRect::new(pad, y + 2.0, 165.0, 20.0),
        false,
        target,
        action_sel,
    );
    let num_titles = [
        "No Page Numbers",
        "Page # Bottom Outer",
        "Page # Bottom Center",
        "Page # Top Outer",
        "Page # Top Center",
    ];
    let page_num_popup = make_popup(
        &num_titles,
        0,
        CGRect::new(pad + 170.0, y, ctrl_w - 170.0, 25.0),
        target,
        action_sel,
    );
    let _: () = msg1(doc_view, sel("addSubview:"), tech_slug_check);
    let _: () = msg1(doc_view, sel("addSubview:"), page_num_popup);
    y += 38.0;

    // Resize document view to exact content height
    let _: () = msg1(
        doc_view,
        sel("setFrame:"),
        CGRect::new(0.0, 0.0, doc_w, y),
    );
    let _: () = msg1(scroll_view, sel("setDocumentView:"), doc_view);
    let _: () = msg1(content_view, sel("addSubview:"), scroll_view);

    // --- PINNED BOTTOM ACTION BAR (`0..footer_h`) ---
    let footer_alloc: Id = msg0(cls("NSView"), sel("alloc"));
    let footer_view: Id = msg1(
        footer_alloc,
        sel("initWithFrame:"),
        CGRect::new(0.0, 0.0, sidebar_w, footer_h),
    );
    let _: () = msg1(
        footer_view,
        sel("setAutoresizingMask:"),
        NSViewMaxXMargin | NSViewMaxYMargin,
    );
    let _: () = msg1(
        footer_view,
        sel("addSubview:"),
        make_separator(CGRect::new(pad, footer_h - 4.0, sidebar_w - 2.0 * pad, 4.0)),
    );

    let f_ctrl_w = sidebar_w - 2.0 * pad;
    let export_btn = make_button(
        "Export Imposed PDF…",
        CGRect::new(pad, 42.0, 172.0, 32.0),
        target,
        sel("onExportPdf:"),
    );
    let print_btn = make_button(
        "Print…",
        CGRect::new(pad + 178.0, 42.0, 96.0, 32.0),
        target,
        sel("onPrintPdf:"),
    );
    let reset_btn = make_button(
        "↺ Reset",
        CGRect::new(pad + 280.0, 42.0, f_ctrl_w - 280.0, 32.0),
        target,
        sel("onResetDefaults:"),
    );
    let gh_btn = make_button(
        &format!("OpenBooklet v{APP_VERSION} · MIT License · GitHub ↗"),
        CGRect::new(pad, 10.0, f_ctrl_w, 24.0),
        target,
        sel("onOpenGitHub:"),
    );
    let _: () = msg1(footer_view, sel("addSubview:"), export_btn);
    let _: () = msg1(footer_view, sel("addSubview:"), print_btn);
    let _: () = msg1(footer_view, sel("addSubview:"), reset_btn);
    let _: () = msg1(footer_view, sel("addSubview:"), gh_btn);
    let _: () = msg1(content_view, sel("addSubview:"), footer_view);

    // --- RIGHT PREVIEW AREA ---
    let right_x = sidebar_w + 8.0;
    let right_w = win_w - right_x - 14.0;

    let prev_btn = make_button(
        "◀ Prev Side",
        CGRect::new(right_x, win_h - 40.0, 100.0, 28.0),
        target,
        sel("onPrevSide:"),
    );
    let _: () = msg1(prev_btn, sel("setAutoresizingMask:"), NSViewMinYMargin | NSViewMaxXMargin);

    let next_btn = make_button(
        "Next Side ▶",
        CGRect::new(right_x + 106.0, win_h - 40.0, 100.0, 28.0),
        target,
        sel("onNextSide:"),
    );
    let _: () = msg1(next_btn, sel("setAutoresizingMask:"), NSViewMinYMargin | NSViewMaxXMargin);

    let sheet_nav_label = make_label(
        "Sheet 1 — Recto (Front)",
        CGRect::new(right_x + 216.0, win_h - 35.0, 340.0, 20.0),
        12.5,
        true,
        false,
    );
    let _: () = msg1(
        sheet_nav_label,
        sel("setAutoresizingMask:"),
        NSViewMinYMargin | NSViewWidthSizable,
    );

    let zoom_out_btn = make_button(
        "Zoom −",
        CGRect::new(win_w - 246.0, win_h - 40.0, 72.0, 28.0),
        target,
        sel("onZoomOut:"),
    );
    let _: () = msg1(zoom_out_btn, sel("setAutoresizingMask:"), NSViewMinYMargin | NSViewMinXMargin);

    let zoom_fit_btn = make_button(
        "Fit",
        CGRect::new(win_w - 168.0, win_h - 40.0, 68.0, 28.0),
        target,
        sel("onZoomFit:"),
    );
    let _: () = msg1(zoom_fit_btn, sel("setAutoresizingMask:"), NSViewMinYMargin | NSViewMinXMargin);

    let zoom_in_btn = make_button(
        "Zoom +",
        CGRect::new(win_w - 94.0, win_h - 40.0, 76.0, 28.0),
        target,
        sel("onZoomIn:"),
    );
    let _: () = msg1(zoom_in_btn, sel("setAutoresizingMask:"), NSViewMinYMargin | NSViewMinXMargin);

    let _: () = msg1(content_view, sel("addSubview:"), prev_btn);
    let _: () = msg1(content_view, sel("addSubview:"), next_btn);
    let _: () = msg1(content_view, sel("addSubview:"), sheet_nav_label);
    let _: () = msg1(content_view, sel("addSubview:"), zoom_out_btn);
    let _: () = msg1(content_view, sel("addSubview:"), zoom_fit_btn);
    let _: () = msg1(content_view, sel("addSubview:"), zoom_in_btn);

    // Center Live Retina PDFView
    let pdf_alloc: Id = msg0(cls("PDFView"), sel("alloc"));
    let pdf_view: Id = msg1(
        pdf_alloc,
        sel("initWithFrame:"),
        CGRect::new(right_x, 38.0, right_w, win_h - 86.0),
    );
    let _: () = msg1(pdf_view, sel("setAutoScales:"), true);
    let _: () = msg1(pdf_view, sel("setDisplaysPageBreaks:"), true);
    let _: () = msg1(pdf_view, sel("setDisplayMode:"), kPDFDisplaySinglePage);
    let _: () = msg1(
        pdf_view,
        sel("setAutoresizingMask:"),
        NSViewWidthSizable | NSViewHeightSizable,
    );
    let _: () = msg1(content_view, sel("addSubview:"), pdf_view);

    // Bottom Preflight Status Bar
    let preflight_label = make_label(
        "Preflight Ready",
        CGRect::new(right_x, 12.0, right_w, 18.0),
        11.5,
        true,
        false,
    );
    let _: () = msg1(
        preflight_label,
        sel("setAutoresizingMask:"),
        NSViewWidthSizable | NSViewMaxYMargin,
    );
    let _: () = msg1(content_view, sel("addSubview:"), preflight_label);

    // Prepare initial input PDF (either CLI argument or 16-page demo PDF)
    let tmp_dir = std::env::temp_dir();
    let default_demo = tmp_dir.join("openbooklet_demo_16pp.pdf");
    let preview_pdf_path = tmp_dir.join("openbooklet_live_preview.pdf");

    let cli_pdf = env::args()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.exists() && p.extension().and_then(|e| e.to_str()) == Some("pdf"));

    let input_pdf_path = if let Some(p) = cli_pdf {
        p
    } else {
        let _ = generate_sample_document_pdf(&default_demo, 16, PaperFormat::A4);
        default_demo
    };

    STATE.with(|cell| {
        *cell.borrow_mut() = Some(AppState {
            window,
            pdf_view,
            file_badge_label,
            sheet_nav_label,
            preflight_label,
            mode_popup,
            paper_popup,
            orientation_popup,
            pdf_box_popup,
            folios_popup,
            flyleaves_popup,
            creep_field,
            duplex_x_field,
            duplex_y_field,
            range_field,
            scale_field,
            gutter_field,
            margin_field,
            rotation_popup,
            page_num_popup,
            rtl_check,
            calendar_check,
            auto_scale_check,
            flip_h_check,
            flip_v_check,
            fold_guide_check,
            crop_marks_check,
            reg_marks_check,
            collating_marks_check,
            tech_slug_check,
            input_pdf_path,
            preview_pdf_path,
            last_summary: None,
        });
    });

    refresh_preview();

    STATE.with(|cell| {
        if let Some(st) = cell.borrow().as_ref() {
            let _: () = msg0(st.window, sel("center"));
            let _: () = msg1(st.window, sel("makeKeyAndOrderFront:"), NIL);
        }
    });
}

unsafe fn read_field_f64(tf: Id, default_val: f64) -> f64 {
    let ns: Id = msg0(tf, sel("stringValue"));
    let s = from_nsstring(ns);
    s.trim().replace(',', ".").parse::<f64>().unwrap_or(default_val)
}

unsafe fn is_checked(btn: Id) -> bool {
    let st: NSInteger = msg0(btn, sel("state"));
    st == NSControlStateValueOn
}

unsafe fn read_config_from_ui(st: &AppState) -> JobConfig {
    let mut cfg = JobConfig::default();

    let mode_idx: NSInteger = msg0(st.mode_popup, sel("indexOfSelectedItem"));
    cfg.mode = MODES
        .get(mode_idx.max(0) as usize)
        .copied()
        .unwrap_or(ImpositionMode::Booklet);

    let paper_idx: NSInteger = msg0(st.paper_popup, sel("indexOfSelectedItem"));
    cfg.paper = PAPERS
        .get(paper_idx.max(0) as usize)
        .copied()
        .unwrap_or(PaperFormat::A4);

    let orient_idx: NSInteger = msg0(st.orientation_popup, sel("indexOfSelectedItem"));
    cfg.landscape_override = match orient_idx {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    };

    let box_idx: NSInteger = msg0(st.pdf_box_popup, sel("indexOfSelectedItem"));
    cfg.pdf_box_mode = match box_idx {
        1 => PrepressBoxMode::MediaBox,
        2 => PrepressBoxMode::CropBox,
        3 => PrepressBoxMode::TrimBox,
        4 => PrepressBoxMode::BleedBox,
        _ => PrepressBoxMode::Auto,
    };

    let folio_idx: NSInteger = msg0(st.folios_popup, sel("indexOfSelectedItem"));
    cfg.sheets_per_signature = match folio_idx {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 5,
        4 => 6,
        5 => 8,
        _ => 4,
    };

    let fly_idx: NSInteger = msg0(st.flyleaves_popup, sel("indexOfSelectedItem"));
    let fly_pages = match fly_idx {
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 0,
    };
    cfg.flyleaves_front = fly_pages;
    cfg.flyleaves_back = fly_pages;

    cfg.creep_mm = read_field_f64(st.creep_field, 0.0);
    cfg.duplex_offset_x_mm = read_field_f64(st.duplex_x_field, 0.0);
    cfg.duplex_offset_y_mm = read_field_f64(st.duplex_y_field, 0.0);

    let range_ns: Id = msg0(st.range_field, sel("stringValue"));
    let range_str = from_nsstring(range_ns);
    cfg.page_range = if range_str.trim().is_empty() {
        "all".to_string()
    } else {
        range_str
    };

    let scale_pct = read_field_f64(st.scale_field, 100.0);
    cfg.scale_factor = (scale_pct / 100.0).clamp(0.1, 4.0);
    cfg.spine_gutter_mm = read_field_f64(st.gutter_field, 0.0);
    cfg.outer_margin_mm = read_field_f64(st.margin_field, 0.0);

    let rot_idx: NSInteger = msg0(st.rotation_popup, sel("indexOfSelectedItem"));
    cfg.extra_rotation_deg = match rot_idx {
        1 => 90,
        2 => 180,
        3 => 270,
        _ => 0,
    };

    let pnum_idx: NSInteger = msg0(st.page_num_popup, sel("indexOfSelectedItem"));
    cfg.page_number_position = match pnum_idx {
        1 => PageNumberPosition::BottomOuter,
        2 => PageNumberPosition::BottomCenter,
        3 => PageNumberPosition::TopOuter,
        4 => PageNumberPosition::TopCenter,
        _ => PageNumberPosition::None,
    };

    cfg.right_to_left = is_checked(st.rtl_check);
    cfg.calendar_flip_verso = is_checked(st.calendar_check);
    cfg.auto_scale = is_checked(st.auto_scale_check);
    cfg.mirror_horizontal = is_checked(st.flip_h_check);
    cfg.mirror_vertical = is_checked(st.flip_v_check);
    cfg.draw_fold_guides = is_checked(st.fold_guide_check);
    cfg.draw_crop_marks = is_checked(st.crop_marks_check);
    cfg.draw_registration_marks = is_checked(st.reg_marks_check);
    cfg.draw_collating_marks = is_checked(st.collating_marks_check);
    cfg.draw_tech_slug = is_checked(st.tech_slug_check);

    cfg
}

fn refresh_preview() {
    unsafe {
        STATE.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let Some(st) = borrow.as_mut() else { return };

            let cfg = read_config_from_ui(st);
            let file_name = st
                .input_pdf_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("document.pdf")
                .to_string();

            if let Ok(info) = inspect_pdf_file(&st.input_pdf_path) {
                let box_badge = if info.has_any_distinct_trim_box {
                    "TrimBox detected"
                } else {
                    "MediaBox standard"
                };
                let badge = format!("{} — {} pages ({})", file_name, info.page_count, box_badge);
                let _: () = msg1(st.file_badge_label, sel("setStringValue:"), nsstring(&badge));
            }

            match render_imposed_pdf(&st.input_pdf_path, &st.preview_pdf_path, &cfg) {
                Ok(summary) => {
                    let url = nsurl_from_path(&st.preview_pdf_path);
                    let pdf_doc_alloc: Id = msg0(cls("PDFDocument"), sel("alloc"));
                    let pdf_doc: Id = msg1(pdf_doc_alloc, sel("initWithURL:"), url);
                    if !pdf_doc.is_null() {
                        let _: () = msg1(st.pdf_view, sel("setDocument:"), pdf_doc);
                    }
                    let _: () = msg1(
                        st.preflight_label,
                        sel("setStringValue:"),
                        nsstring(&summary.preflight.status_badge),
                    );
                    st.last_summary = Some(summary);
                    update_navigation_badge(st);
                }
                Err(err) => {
                    let msg = format!("Render error: {err}");
                    let _: () = msg1(st.preflight_label, sel("setStringValue:"), nsstring(&msg));
                }
            }
        });
    }
}

unsafe fn update_navigation_badge(st: &AppState) {
    let doc: Id = msg0(st.pdf_view, sel("document"));
    let cur_page: Id = msg0(st.pdf_view, sel("currentPage"));
    if doc.is_null() || cur_page.is_null() {
        return;
    }
    let idx: NSUInteger = msg1(doc, sel("indexForPage:"), cur_page);
    if let Some(summary) = &st.last_summary {
        if let Some(side) = summary.plan.sides.get(idx) {
            let nav_str = format!(
                "Side {} of {}  —  Sheet {} ({})  |  Signature {}/{}",
                idx + 1,
                summary.plan.sides.len(),
                side.physical_sheet_number,
                side.side.label(),
                side.signature_index + 1,
                side.total_signatures
            );
            let _: () = msg1(st.sheet_nav_label, sel("setStringValue:"), nsstring(&nav_str));
        }
    }
}
