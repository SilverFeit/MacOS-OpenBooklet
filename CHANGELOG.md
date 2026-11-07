# Changelog

All notable changes to **MacOS OpenBooklet** are documented in this file.

## [1.0.0] — 2026-11-07

### Added
- **Clean-Room 100% Rust Workspace Architecture** (`openbooklet-core`, `openbooklet-pdf`, `openbooklet-cli`, `openbooklet-app`) released under the **MIT License**.
- **Native Quartz 2D (`CoreGraphics` + `CoreText`) Vector PDF Engine:** Preserves 100% vector graphics, embedded fonts, and color spaces with zero rasterization and zero external runtime dependencies.
- **12 Imposition, Cut-and-Stack & De-Imposition Modes:**
  - `booklet` (Saddle-Stitch Single Booklet)
  - `signatures` (Multi-Signature Sewn / Glued Bookbinding)
  - `cut-stack-2up` & `cut-stack-4up` (Guillotine Cut & Stack with Verso horizontal column mirroring)
  - `deimpose` & `split-2up` (Reverse imposed 2-up booklets or spreads back into sequential `1..N` single pages)
  - `2up`, `4up-rows`, `4up-cols`, `2-copies`, `4-copies`, and `1up`.
- **Duplex Printer Calibration (`Verso dX / dY mm`) + Printable 2-Page Vernier Calibration Sheet** (`Cmd+K` / `openbooklet calibrate-sheet`).
- **Stepped Spine Collating Marks** with reverse micro-numbered signature index (`1`, `2`, `3`...) on the outermost fold of each signature.
- **Automatic Front & Back Flyleaves (Endpapers)** injection (`+2`, `+4`, `+8` blank pages).
- **ISO 32000-1 Prepress 5-Box Inspector (`MediaBox`, `CropBox`, `TrimBox`, `BleedBox`, `ArtBox`)** with `Auto` TrimBox detection (`openbooklet inspect`).
- **Native macOS `AppKit` + `PDFKit` GUI (`OpenBooklet.app`)** and bundled **`openbooklet` CLI** with `--dry-run`, `--json`, and Homebrew Cask packaging (`Casks/macos-openbooklet.rb`).
