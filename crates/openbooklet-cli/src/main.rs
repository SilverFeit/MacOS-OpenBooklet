//! `openbooklet` — Native macOS CLI for vector PDF imposition, bookbinding signatures,
//! guillotine cut-and-stack, booklet de-imposition, and duplex printer calibration.

use std::env;
use std::path::PathBuf;
use std::process;

use openbooklet_core::{
    analyze_preflight, compute_imposition_plan, format_dry_run_table, ImpositionMode, JobConfig,
    PageNumberPosition, PaperFormat, PrepressBoxMode,
};
use openbooklet_pdf::{
    generate_duplex_calibration_pdf, generate_sample_document_pdf, inspect_pdf_file,
    render_imposed_pdf,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "OpenBooklet CLI v{VERSION} — Native Vector PDF Imposition & Bookbinding Engine (MIT)
https://github.com/SilverFeit/MacOS-OpenBooklet

USAGE:
  openbooklet [impose] <input.pdf> <output.pdf> [OPTIONS]
  openbooklet deimpose <input.pdf> <output.pdf> [--split-2up] [--paper <fmt>]
  openbooklet calibrate-sheet <output.pdf> [--paper <fmt>] [--portrait]
  openbooklet inspect <input.pdf> [--json]
  openbooklet sample <output.pdf> [--pages <N>] [--paper <fmt>]
  openbooklet modes

IMPOSITION MODES (--mode / --preset):
  booklet          Single saddle-stitch booklet (4 pages per folded sheet)
  signatures       Multi-signature bookbinding (sewn / glued quires)
  cut-stack-2up    Guillotine 2-Up Cut & Stack (cut center & stack left on right)
  cut-stack-4up    Guillotine 4-Up Cut & Stack (2x2 grid into 4 sequential stacks)
  deimpose         Reverse an imposed 2-Up booklet back into sequential 1..N pages
  split-2up        Split 2-Up spreads horizontally into sequential single pages
  2up              Standard 2-Up sequential side-by-side
  4up-rows         4-Up 2x2 grid ordered by rows
  4up-cols         4-Up 2x2 grid ordered by columns
  2-copies         Step & Repeat: 2 identical copies per sheet side
  4-copies         Step & Repeat: 4 identical copies per sheet side
  1up              1-Up prepress framing, scaling & trim marks

OPTIONS:
  -m, --mode, --preset <mode>     Imposition mode (default: booklet)
  -p, --paper <format>            Paper format: a4, letter, legal, a3, tabloid, sra3, a5
      --custom-size <WxH>         Custom sheet size in millimeters (e.g. 210x297)
      --landscape / --portrait    Force sheet orientation
  -f, --folios <N>                Folded sheets per signature (default: 4 = 16 pages)
  -c, --creep <mm>                Spine creep shingling compensation in mm (default: 0.0)
      --duplex-offset <dx,dy>     Verso duplex printer calibration offset in mm (e.g. 1.2,-0.5)
      --duplex-offset-x <mm>      Verso horizontal calibration shift in mm
      --duplex-offset-y <mm>      Verso vertical calibration shift in mm
      --flyleaves <front,back>    Blank endpaper pages at start,end (e.g. 2,2)
      --flyleaves-front <N>       Blank endpaper pages at front of book
      --flyleaves-back <N>        Blank endpaper pages at back of book
      --pdf-box <mode>            Reference box: auto, media, crop, trim, bleed (default: auto)
  -r, --range <expr>              Page range: all, odd, even, or e.g. \"1-16,20\"
      --rtl                       Right-to-Left binding (Manga / Arabic / Hebrew)
      --calendar                  Rotate Verso sides 180 deg for top-bound calendars
  -s, --scale <factor>            Scale factor (e.g. 0.95 or 95 for 95%)
      --no-auto-scale             Disable automatic fit-to-slot scaling
      --shift-x <mm>              Global horizontal shift in mm
      --shift-y <mm>              Global vertical shift in mm
      --gutter <mm>               Central spine gutter width in mm
      --margin <mm>               Outer sheet margin in mm
      --rotate <0|90|180|270>     Additional page rotation in degrees
      --flip-h / --flip-v         Mirror page content horizontally or vertically
      --crop-marks                Draw perimeter corner crop marks
      --reg-marks                 Draw registration bullseye targets
      --collating-marks           Draw stepped spine collating blocks on signatures
      --slug                      Draw technical prepress metadata slug
      --all-marks                 Enable crop, registration, collating marks & tech slug
      --no-fold-line              Hide dashed fold / guillotine cut guide lines
      --page-numbers <pos>        Stamp page numbers: none, bottom-outer, bottom-center, top-outer, top-center
      --dry-run                   Print the computed sheet/slot imposition plan without writing a PDF
      --json                      Emit structured JSON output
  -v, --version                   Show version
  -h, --help                      Show this help message"
    );
}

fn print_modes() {
    let modes = [
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
    println!("Available OpenBooklet Imposition & Prepress Modes:");
    for m in modes {
        println!("  {:<16} — {}", m.as_slug(), m.display_name());
    }
}

fn parse_scale_value(s: &str) -> Result<f64, String> {
    let cleaned = s.trim().trim_end_matches('%');
    let val: f64 = cleaned
        .parse()
        .map_err(|_| format!("Invalid scale value: {s}"))?;
    if val > 5.0 {
        Ok(val / 100.0)
    } else {
        Ok(val)
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            return;
        }
        "-v" | "--version" | "version" => {
            println!("openbooklet {VERSION}");
            return;
        }
        "modes" | "list-modes" => {
            print_modes();
            return;
        }
        "inspect" => {
            run_inspect(&args[1..]);
            return;
        }
        "calibrate-sheet" | "calibration-sheet" => {
            run_calibrate_sheet(&args[1..]);
            return;
        }
        "sample" => {
            run_sample(&args[1..]);
            return;
        }
        _ => {}
    }

    let mut cfg = JobConfig::default();
    let mut positional: Vec<String> = Vec::new();
    let mut dry_run = false;
    let mut json_out = false;

    let mut idx = 0usize;
    if args[0] == "impose" {
        idx = 1;
    } else if args[0] == "deimpose" {
        cfg.mode = ImpositionMode::DeimposeBooklet;
        cfg.landscape_override = Some(false);
        idx = 1;
    }

    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-m" | "--mode" | "--preset" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --mode"));
                cfg.mode = ImpositionMode::from_slug(val)
                    .unwrap_or_else(|| die(&format!("Unknown mode '{val}'. Run `openbooklet modes`.")));
            }
            "--split-2up" => {
                cfg.mode = ImpositionMode::Split2Up;
                cfg.landscape_override = Some(false);
            }
            "-p" | "--paper" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --paper"));
                cfg.paper = PaperFormat::from_slug(val)
                    .unwrap_or_else(|| die(&format!("Unknown paper format '{val}'.")));
            }
            "--custom-size" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --custom-size"));
                let (w_s, h_s) = val
                    .split_once('x')
                    .or_else(|| val.split_once('X'))
                    .unwrap_or_else(|| die("Expected WxH in mm for --custom-size (e.g. 210x297)"));
                let w: f64 = w_s.parse().unwrap_or_else(|_| die("Invalid custom width"));
                let h: f64 = h_s.parse().unwrap_or_else(|_| die("Invalid custom height"));
                cfg.paper = PaperFormat::Custom {
                    width_mm: w,
                    height_mm: h,
                };
            }
            "--landscape" => cfg.landscape_override = Some(true),
            "--portrait" => cfg.landscape_override = Some(false),
            "-f" | "--folios" | "--sheets-per-sig" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --folios"));
                cfg.sheets_per_signature = val
                    .parse::<usize>()
                    .unwrap_or_else(|_| die("Invalid integer for --folios"))
                    .max(1);
            }
            "-c" | "--creep" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --creep"));
                cfg.creep_mm = val.parse().unwrap_or_else(|_| die("Invalid float for --creep"));
            }
            "--duplex-offset" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --duplex-offset"));
                let (dx_s, dy_s) = val
                    .split_once(',')
                    .unwrap_or_else(|| die("Expected dx,dy in mm for --duplex-offset"));
                cfg.duplex_offset_x_mm = dx_s.trim().parse().unwrap_or_else(|_| die("Invalid dx"));
                cfg.duplex_offset_y_mm = dy_s.trim().parse().unwrap_or_else(|_| die("Invalid dy"));
            }
            "--duplex-offset-x" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --duplex-offset-x"));
                cfg.duplex_offset_x_mm = val.parse().unwrap_or_else(|_| die("Invalid float"));
            }
            "--duplex-offset-y" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --duplex-offset-y"));
                cfg.duplex_offset_y_mm = val.parse().unwrap_or_else(|_| die("Invalid float"));
            }
            "--flyleaves" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --flyleaves"));
                if let Some((f_s, b_s)) = val.split_once(',') {
                    cfg.flyleaves_front = f_s.trim().parse().unwrap_or(0);
                    cfg.flyleaves_back = b_s.trim().parse().unwrap_or(0);
                } else {
                    let n: usize = val.parse().unwrap_or(0);
                    cfg.flyleaves_front = n;
                    cfg.flyleaves_back = n;
                }
            }
            "--flyleaves-front" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --flyleaves-front"));
                cfg.flyleaves_front = val.parse().unwrap_or(0);
            }
            "--flyleaves-back" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --flyleaves-back"));
                cfg.flyleaves_back = val.parse().unwrap_or(0);
            }
            "--pdf-box" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --pdf-box"));
                cfg.pdf_box_mode = PrepressBoxMode::from_slug(val)
                    .unwrap_or_else(|| die("Expected auto, media, crop, trim, or bleed"));
            }
            "-r" | "--range" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --range"));
                cfg.page_range = val.clone();
            }
            "--rtl" => cfg.right_to_left = true,
            "--calendar" => cfg.calendar_flip_verso = true,
            "-s" | "--scale" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --scale"));
                cfg.scale_factor = parse_scale_value(val).unwrap_or_else(|e| die(&e));
            }
            "--no-auto-scale" => cfg.auto_scale = false,
            "--shift-x" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --shift-x"));
                cfg.shift_x_mm = val.parse().unwrap_or(0.0);
            }
            "--shift-y" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --shift-y"));
                cfg.shift_y_mm = val.parse().unwrap_or(0.0);
            }
            "--gutter" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --gutter"));
                cfg.spine_gutter_mm = val.parse().unwrap_or(0.0);
            }
            "--margin" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --margin"));
                cfg.outer_margin_mm = val.parse().unwrap_or(0.0);
            }
            "--rotate" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --rotate"));
                cfg.extra_rotation_deg = val.parse().unwrap_or(0);
            }
            "--flip-h" => cfg.mirror_horizontal = true,
            "--flip-v" => cfg.mirror_vertical = true,
            "--crop-marks" => cfg.draw_crop_marks = true,
            "--reg-marks" => cfg.draw_registration_marks = true,
            "--collating-marks" => cfg.draw_collating_marks = true,
            "--slug" => cfg.draw_tech_slug = true,
            "--all-marks" => {
                cfg.draw_crop_marks = true;
                cfg.draw_registration_marks = true;
                cfg.draw_collating_marks = true;
                cfg.draw_tech_slug = true;
            }
            "--no-fold-line" => cfg.draw_fold_guides = false,
            "--page-numbers" => {
                idx += 1;
                let val = args.get(idx).unwrap_or_else(|| die("Missing value for --page-numbers"));
                cfg.page_number_position = PageNumberPosition::from_slug(val)
                    .unwrap_or_else(|| die("Expected none, bottom-outer, bottom-center, top-outer, top-center"));
            }
            "--dry-run" => dry_run = true,
            "--json" => json_out = true,
            other if other.starts_with('-') => {
                die(&format!("Unrecognized flag '{other}'. Run `openbooklet --help`."));
            }
            path_arg => positional.push(path_arg.to_string()),
        }
        idx += 1;
    }

    if positional.is_empty() {
        die("Missing <input.pdf>. Run `openbooklet --help` for usage.");
    }

    let input_path = PathBuf::from(&positional[0]);
    let inspection = inspect_pdf_file(&input_path).unwrap_or_else(|e| die(&e));

    if dry_run {
        let plan = compute_imposition_plan(&cfg, inspection.page_count);
        let preflight = analyze_preflight(&cfg, &plan, inspection.has_any_distinct_trim_box);
        if json_out {
            println!(
                "{{\n  \"status\": \"dry_run\",\n  \"preflight\": \"{}\",\n  \"config\": {}\n}}",
                preflight.status_badge,
                cfg.to_json_pretty()
            );
        } else {
            print!("{}", format_dry_run_table(&cfg, &plan));
            println!("{}", preflight.status_badge);
        }
        return;
    }

    if positional.len() < 2 {
        die("Missing <output.pdf>. Specify an output PDF path or pass `--dry-run`.");
    }

    let output_path = PathBuf::from(&positional[1]);
    let summary = render_imposed_pdf(&input_path, &output_path, &cfg).unwrap_or_else(|e| die(&e));

    if json_out {
        println!(
            "{{\n  \"status\": \"ok\",\n  \"input\": \"{}\",\n  \"output\": \"{}\",\n  \"mode\": \"{}\",\n  \"source_pages\": {},\n  \"logical_pages\": {},\n  \"physical_sheets\": {},\n  \"signatures\": {},\n  \"preflight\": \"{}\"\n}}",
            input_path.display(),
            summary.output_path,
            cfg.mode.as_slug(),
            summary.plan.source_page_count,
            summary.plan.total_logical_pages,
            summary.plan.total_physical_sheets,
            summary.plan.total_signatures,
            summary.preflight.status_badge
        );
    } else {
        println!("==> OpenBooklet v{VERSION}: Vector PDF Imposition Complete");
        println!("    Input    : {} ({} pp)", input_path.display(), summary.plan.source_page_count);
        println!("    Output   : {}", summary.output_path);
        println!("    Mode     : {}", cfg.mode.display_name());
        println!(
            "    Sheets   : {} physical sheets ({} sides, {} signature(s))",
            summary.plan.total_physical_sheets,
            summary.plan.sides.len(),
            summary.plan.total_signatures
        );
        println!("    {}", summary.preflight.status_badge);
    }
}

fn run_inspect(args: &[String]) {
    if args.is_empty() {
        die("Usage: openbooklet inspect <input.pdf> [--json]");
    }
    let path = PathBuf::from(&args[0]);
    let json = args.iter().any(|a| a == "--json");
    let info = inspect_pdf_file(&path).unwrap_or_else(|e| die(&e));
    if json {
        println!("{}", info.to_json_pretty());
    } else {
        println!("File                : {}", info.path_display);
        println!("Pages               : {}", info.page_count);
        println!("Distinct TrimBox    : {}", info.has_any_distinct_trim_box);
        println!("Distinct BleedBox   : {}", info.has_any_distinct_bleed_box);
        println!("Distinct CropBox    : {}", info.has_any_distinct_crop_box);
        if let Some(first) = info.pages.first() {
            println!(
                "Page 1 MediaBox     : {:.1} x {:.1} pt ({:.1} x {:.1} mm)",
                first.media_box.width,
                first.media_box.height,
                openbooklet_core::pt_to_mm(first.media_box.width),
                openbooklet_core::pt_to_mm(first.media_box.height),
            );
            println!(
                "Page 1 TrimBox      : {:.1} x {:.1} pt ({:.1} x {:.1} mm)",
                first.trim_box.width,
                first.trim_box.height,
                openbooklet_core::pt_to_mm(first.trim_box.width),
                openbooklet_core::pt_to_mm(first.trim_box.height),
            );
        }
    }
}

fn run_calibrate_sheet(args: &[String]) {
    if args.is_empty() {
        die("Usage: openbooklet calibrate-sheet <output.pdf> [--paper a4] [--portrait]");
    }
    let out = PathBuf::from(&args[0]);
    let mut paper = PaperFormat::A4;
    let mut landscape = true;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--paper" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    paper = PaperFormat::from_slug(v).unwrap_or(PaperFormat::A4);
                }
            }
            "--portrait" => landscape = false,
            "--landscape" => landscape = true,
            _ => {}
        }
        i += 1;
    }
    generate_duplex_calibration_pdf(&out, paper, landscape).unwrap_or_else(|e| die(&e));
    println!(
        "==> Generated 2-page Duplex Vernier Calibration Sheet at {}",
        out.display()
    );
}

fn run_sample(args: &[String]) {
    if args.is_empty() {
        die("Usage: openbooklet sample <output.pdf> [--pages 16] [--paper a4]");
    }
    let out = PathBuf::from(&args[0]);
    let mut pages = 8usize;
    let mut paper = PaperFormat::A4;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pages" | "-n" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    pages = v.parse().unwrap_or(8);
                }
            }
            "-p" | "--paper" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    paper = PaperFormat::from_slug(v).unwrap_or(PaperFormat::A4);
                }
            }
            _ => {}
        }
        i += 1;
    }
    generate_sample_document_pdf(&out, pages, paper).unwrap_or_else(|e| die(&e));
    println!(
        "==> Generated {pages}-page vector sample PDF at {}",
        out.display()
    );
}

fn die(msg: &str) -> ! {
    eprintln!("openbooklet error: {msg}");
    process::exit(1);
}
