# MacOS OpenBooklet — User & Prepress Guide

## 1. Getting Started

Launch **`OpenBooklet.app`** from `/Applications` or run `openbooklet` from the terminal. Upon startup, **OpenBooklet** loads a built-in 16-page vector demonstration manuscript so you can immediately explore every imposition schema, signature setting, and prepress mark in the live Retina `PDFView` canvas.

To load your own PDF:
- Click **`Open PDF…`** (`Cmd+O`) in the top-left of the sidebar, or
- Pass the PDF path directly on the command line: `openbooklet input.pdf output.pdf`.

---

## 2. Choosing the Right Imposition Mode

| Mode | CLI Slug | When to Use |
| :--- | :--- | :--- |
| **Single Booklet (Saddle-Stitch)** | `booklet` | Zines, pamphlets, and magazines up to ~64 pages that fold in a single stack and are stapled or sewn through the center fold. |
| **Multi-Signature Book** | `signatures` | Books of any length bound by sewing or gluing multiple folded sections (*quires / cuadernillos*, typically 16 or 24 pages per signature). |
| **Guillotine Cut & Stack (2-Up)** | `cut-stack-2up` | Ring-bound manuals, notepads, or perfect-bound blocks cut in half with a guillotine: print duplex, cut down the center line, and place the **Left pile on top of the Right pile** for instant `1..N` ordering without folding. |
| **Guillotine Cut & Stack (4-Up)** | `cut-stack-4up` | Postcards, A6 booklets, or tickets printed 4-up: cut into 4 quadrants and stack `Top-Left → Top-Right → Bottom-Left → Bottom-Right`. |
| **De-Impose Booklet -> Linear PDF** | `deimpose` | Converts an already-imposed 2-up saddle-stitch booklet PDF back into sequential `1, 2, 3, ..., N` single reading pages. |
| **Split 2-Up Spreads -> Single Pages** | `split-2up` | Splits double-page spreads or scanned books down the middle into individual sequential pages. |
| **2-Up / 4-Up / Step & Repeat / 1-Up** | `2up`, `4up-rows`, `4up-cols`, `2-copies`, `4-copies`, `1up` | Handouts, multi-slide sheets, duplicate flyers, or 1-up prepress crop-mark framing. |

---

## 3. Calibrating Your Printer's Duplex Unit (`Verso dX / dY mm`)

Almost all home and office duplex printers exhibit a small mechanical front-to-back shift (`0.5 mm` to `2.5 mm`).

1. In **OpenBooklet**, click **`Export Printable Duplex Calibration Sheet (PDF)…`** (`Cmd+K`) or run:
   ```bash
   openbooklet calibrate-sheet calibration.pdf --paper a4
   ```
2. Print `calibration.pdf` **Duplex (Two-Sided, Long-Edge Flip)** at **100% Actual Size** (disable "Fit to Printable Area").
3. Hold the printed sheet up to a window or desk lamp:
   - **Side 1 (Recto)** has a graduated millimeter vernier ruler (`-15 mm` to `+15 mm` on both X and Y axes).
   - **Side 2 (Verso)** has a bold crimson crosshair at the exact center.
4. Read the millimeter coordinate where the crimson Verso crosshair intersects the Front ruler, and enter the compensating `Verso dX (mm)` and `Verso dY (mm)` values into Section 4 of the sidebar (or pass `--duplex-offset dx,dy` on the CLI).

---

## 4. Bookbinding Signatures, Creep & Spine Collating Marks

- **Sheets per Signature (Folios):** Each physical folded sheet contains `4` pages (`2` on Recto, `2` on Verso). A standard 16-page signature uses **`4 Sheets (16 pp / sig)`**.
- **Creep (Shingling) Compensation (`mm`):** As sheets nest inside a signature, paper thickness pushes inner leaves outward. Setting `Creep (mm)` (e.g. `1.0 mm` for 4 sheets of 80 gsm paper) progressively shifts inner pages toward the spine fold so the fore-edge trims flush without clipping outer page margins.
- **Spine Collating Marks:** Enable **`Spine Collating Marks`** to print a stepped black block with the signature number (`1`, `2`, `3`...) on the outer fold of each signature's outermost sheet. When all folded signatures are gathered in sequence, the black marks form a continuous diagonal staircase down the spine.
- **Flyleaves (Endpapers):** Select **`+2 pp`**, **`+4 pp`**, or **`+8 pp Front & Back`** to automatically prepend and append blank protective leaves at the start and end of the book block.
