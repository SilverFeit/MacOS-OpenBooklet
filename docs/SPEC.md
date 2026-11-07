# OpenBooklet Clean-Room Mathematical & Prepress Specification

This document defines the clean-room mathematical models, page-ordering algorithms, coordinate transformations, and prepress mark specifications implemented in `openbooklet-core` and `openbooklet-pdf`. All algorithms are derived from first-principles geometry and public domain graphic-arts print standards (ISO 12647 / PDF 1.7 ISO 32000-1).

---

## 1. Units & Coordinate System

- **Base PDF Unit:** PostScript point (`1 pt = 1 / 72 inch`).
- **Metric Conversion:** `1 mm = 72.0 / 25.4 pt ≈ 2.83464567 pt`.
- **Origin:** Standard PDF bottom-left origin `(0, 0)` with $+X$ rightward and $+Y$ upward.

---

## 2. Logical Page Pipeline & Flyleaves (Endpapers)

Given an input document with $P_{\text{raw}}$ pages (`1..=P_raw`):

1. **Page Selection Filter:** A range expression (`all`, `odd`, `even`, or comma-separated tokens `a-b`, `k`) filters and orders the active page stream $S_0$.
2. **Flyleaves / Endpapers Injection:**
   - $F_{\text{front}}$ blank pages are prepended before $S_0$.
   - $F_{\text{back}}$ blank pages are appended after $S_0$.
   - Resulting stream: $S_1 = [\text{Blank} \times F_{\text{front}}] \mathbin{\Vert} S_0 \mathbin{\Vert} [\text{Blank} \times F_{\text{back}}]$.

---

## 3. Imposition Modes & Page-Ordering Formulas

### 3.1 Single Booklet (Saddle-Stitch / 4 Pages per Folio Sheet)

Let $N = |S_1|$, padded with trailing `Blank` entries until $N \equiv 0 \pmod 4$ and $N \ge 4$.
The number of physical sheets (folios) is $M = N / 4$, indexed $s \in \{0, 1, \dots, M-1\}$ from outermost ($s = 0$) to innermost ($s = M-1$).

For each sheet $s$:
- **Recto (Front Side):**
  $$\text{Left} = S_1[N - 1 - 2s], \quad \text{Right} = S_1[2s]$$
- **Verso (Back Side):**
  $$\text{Left} = S_1[2s + 1], \quad \text{Right} = S_1[N - 2 - 2s]$$
- **Right-to-Left (RTL) Binding:** Swaps `Left` and `Right` on every side so spine numbering progresses from right to left.

### 3.2 Multi-Signature Bookbinding (`Signatures`)

Given $k$ sheets per signature ($4k$ pages per signature, default $k = 4 \implies 16$ pages/signature):
- Partition $S_1$ into contiguous chunks of size $C = 4k$ (padding the final chunk to a multiple of 4).
- Apply the Saddle-Stitch ordering independently to each signature $q \in \{0, 1, \dots, Q-1\}$.
- Record `signature_index = q` and `is_outermost_sheet = (s == 0)` on each generated sheet for spine collating mark placement.

### 3.3 Guillotine Cut & Stack (`CutAndStack2Up` & `CutAndStack4Up`)

Unlike folded booklets, **Cut & Stack** produces duplex sheets that are cut straight through with a guillotine and stacked without folding:

- **2-Up Cut & Stack:**
  - Pad $S_1$ with blanks so total pages $N$ is a multiple of 4; total duplex sheets $M = N / 4$.
  - Left pile covers pages $0 \dots 2M - 1$; Right pile covers pages $2M \dots 4M - 1$.
  - For sheet $s \in \{0, \dots, M-1\}$:
    - **Recto:** $\text{Left} = S_1[2s], \quad \text{Right} = S_1[2M + 2s]$
    - **Verso:** $\text{Left} = S_1[2s + 1], \quad \text{Right} = S_1[2M + 2s + 1]$
    - *(Note: On the Verso of a duplex sheet flipped along the long edge, the physical back of Recto-Left lies on Verso-Right! Therefore Verso swaps horizontal slots: $\text{Verso-Left} = S_1[2M + 2s + 1]$ and $\text{Verso-Right} = S_1[2s + 1]$ so that cutting down the middle places $S_1[2s+1]$ directly on the back of $S_1[2s]$).*
  - After cutting down the center line and placing the Left stack on top of the Right stack, all pages are in sequential order $1 \dots N$.

- **4-Up Cut & Stack:**
  - Divides $S_1$ into 4 vertical stacks ($N$ padded to a multiple of 8, $M = N / 8$ sheets) with Verso horizontal column mirroring so back-to-back registration holds across all 4 quadrants.

### 3.4 De-Impose / Linearize (`DeimposeBooklet` & `Split2Up`)

Reverses a 2-up spread or booklet PDF of $K$ spread pages into $2K$ single pages:
- Each input spread page $p \in \{1, \dots, K\}$ is split along $x = W_{\text{src}} / 2$ into `LeftHalf(p)` and `RightHalf(p)`.
- **`Split2Up` (Sequential Spread Split):** Emits `[LeftHalf(1), RightHalf(1), LeftHalf(2), RightHalf(2), ...]`.
- **`DeimposeBooklet` (Saddle-Stitch Reversal):**
  - Given $M = K / 2$ sheets (pairs of Recto $p_{2s}$ and Verso $p_{2s+1}$):
  - Reconstructs logical pages:
    - Page $2s + 1 \leftarrow \text{RightHalf}(\text{Recto}_s)$
    - Page $2s + 2 \leftarrow \text{LeftHalf}(\text{Verso}_s)$
    - Page $N - 2s \leftarrow \text{LeftHalf}(\text{Recto}_s)$
    - Page $N - 1 - 2s \leftarrow \text{RightHalf}(\text{Verso}_s)$

### 3.5 Standard N-Up & Step-and-Repeat Layouts

- **`TwoUp`:** Consecutive pairs `[2i, 2i+1]` per sheet side.
- **`FourUpRows`:** 2×2 grid filled row-major `[TopLeft, TopRight, BottomLeft, BottomRight]`.
- **`FourUpCols`:** 2×2 grid filled column-major `[TopLeft, BottomLeft, TopRight, BottomRight]`.
- **`NCopies(2 | 4)`:** Repeats the same logical page across all slots on the sheet side.
- **`OneUp`:** 1 page per sheet side with full prepress box framing, scaling, and marks.

---

## 4. Geometry, Creep, Duplex Calibration & Prepress Boxes

### 4.1 PDF Box Hierarchy (`PrepressBox`)
Each source PDF page exposes up to five boxes: `MediaBox`, `CropBox`, `BleedBox`, `TrimBox`, and `ArtBox`.
- **`Auto` Box Resolution:** Uses `TrimBox` if present and smaller than `MediaBox`; falls back to `CropBox`, then `MediaBox`.
- When `CropMarks` are enabled, marks align to the resolved `TrimBox` corners while allowing content up to `BleedBox` (`bleed_pt`) to extend outward toward the cut lines.

### 4.2 Creep (Shingling) Compensation
For a signature of $M$ sheets ($s \in \{0, \dots, M-1\}$ from outside to inside) and total creep $C_{\text{mm}}$:
$$\Delta_{\text{creep}}(s) = C_{\text{pt}} \cdot \frac{s}{\max(1, M - 1)}$$
- On the left slot, page content shifts rightward ($+X$, toward the central spine) by $\Delta_{\text{creep}}(s)$.
- On the right slot, page content shifts leftward ($-X$, toward the central spine) by $\Delta_{\text{creep}}(s)$.

### 4.3 Duplex Printer Calibration (`duplex_offset_x_mm`, `duplex_offset_y_mm`)
Consumer and office duplex units often exhibit a mechanical front-to-back registration offset $(\delta_x, \delta_y)$.
- Applied **exclusively to Verso (Back)** sheet sides:
  $$T_{\text{side}} = \begin{cases} (0, 0) & \text{if Recto} \\ (\delta_x, \delta_y) & \text{if Verso} \end{cases}$$

### 4.4 Spine Collating Marks
For multi-signature jobs (`Signatures`) with $Q$ signatures, on the outermost sheet ($s = 0$, Recto) of signature $q \in \{0, \dots, Q-1\}$:
- A solid black collating block of width $4\text{ pt}$ and height $h_{\text{mark}} = 14\text{ pt}$ is drawn squarely on the central spine fold line $x = W_{\text{sheet}} / 2$.
- Its vertical position steps sequentially from top to bottom along the spine:
  $$y_{\text{mark}}(q) = H_{\text{sheet}} - 54\text{ pt} - (q \cdot h_{\text{mark}}) \pmod{H_{\text{sheet}} - 108\text{ pt}}$$
- Inside the block (or immediately adjacent), the signature number $q + 1$ is stamped in reverse white micro-typography (`5 pt`).
