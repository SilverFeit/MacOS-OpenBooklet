//! Clean-room imposition and de-imposition layout generator.

use crate::model::{
    mm_to_pt, ImpositionMode, JobConfig, PageCropRegion, PageSource, SheetSide,
};
use crate::range::parse_page_range;

/// A positioned page slot on a single output sheet side.
#[derive(Debug, Clone)]
pub struct SlotPlacement {
    /// Column index (0..cols).
    pub col: usize,
    /// Row index from top to bottom (0..rows).
    pub row: usize,
    /// Logical source page or Blank.
    pub source: PageSource,
    /// 1-based sequential reading index in the imposed book (used for optional page numbering).
    pub logical_reading_number: Option<usize>,
    /// Horizontal creep displacement in points (+X rightward, -X leftward toward spine).
    pub creep_shift_x_pt: f64,
}

/// One physical side (Recto or Verso) of an output press sheet.
#[derive(Debug, Clone)]
pub struct ImposedSheetSide {
    /// 1-based physical sheet number (`1..=total_physical_sheets`).
    pub physical_sheet_number: usize,
    /// Recto (Front) or Verso (Back).
    pub side: SheetSide,
    /// 0-based signature index when in Booklet or Signatures mode.
    pub signature_index: usize,
    /// Total number of signatures in the job.
    pub total_signatures: usize,
    /// 0-based sheet index inside its signature (0 = outermost sheet, M-1 = innermost centerfold).
    pub sheet_index_in_signature: usize,
    /// Total sheets in this signature.
    pub sheets_in_signature: usize,
    /// Grid dimensions `(cols, rows)`.
    pub grid: (usize, usize),
    /// Placed page slots on this sheet side.
    pub slots: Vec<SlotPlacement>,
}

/// Complete result of an imposition layout calculation.
#[derive(Debug, Clone)]
pub struct ImpositionPlan {
    pub mode: ImpositionMode,
    pub source_page_count: usize,
    pub selected_page_count: usize,
    pub flyleaves_front: usize,
    pub flyleaves_back: usize,
    pub padded_blank_pages: usize,
    pub total_logical_pages: usize,
    pub total_physical_sheets: usize,
    pub total_signatures: usize,
    pub sides: Vec<ImposedSheetSide>,
}

/// Build the complete imposition or de-imposition plan for `source_page_count` pages.
pub fn compute_imposition_plan(config: &JobConfig, source_page_count: usize) -> ImpositionPlan {
    let selected_pages = parse_page_range(&config.page_range, source_page_count);
    let selected_page_count = selected_pages.len();

    if config.mode.is_deimpose_or_split() {
        return compute_deimpose_plan(config, source_page_count, &selected_pages);
    }

    // Build logical stream with front/back flyleaves
    let mut stream: Vec<(PageSource, Option<usize>)> = Vec::new();
    for _ in 0..config.flyleaves_front {
        stream.push((PageSource::Blank, None));
    }
    for (idx, &page_num) in selected_pages.iter().enumerate() {
        stream.push((
            PageSource::DocumentPage {
                page_number: page_num,
                crop: PageCropRegion::FullPage,
            },
            Some(idx + 1),
        ));
    }
    for _ in 0..config.flyleaves_back {
        stream.push((PageSource::Blank, None));
    }

    match config.mode {
        ImpositionMode::Booklet => {
            build_signatures_plan(config, source_page_count, selected_page_count, stream, None)
        }
        ImpositionMode::Signatures => {
            let sheets_per_sig = config.sheets_per_signature.max(1);
            build_signatures_plan(
                config,
                source_page_count,
                selected_page_count,
                stream,
                Some(sheets_per_sig),
            )
        }
        ImpositionMode::CutAndStack2Up => {
            build_cut_and_stack_2up(config, source_page_count, selected_page_count, stream)
        }
        ImpositionMode::CutAndStack4Up => {
            build_cut_and_stack_4up(config, source_page_count, selected_page_count, stream)
        }
        ImpositionMode::TwoUp => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (2, 1), false, false)
        }
        ImpositionMode::FourUpRows => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (2, 2), false, false)
        }
        ImpositionMode::FourUpCols => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (2, 2), true, false)
        }
        ImpositionMode::NCopies2 => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (2, 1), false, true)
        }
        ImpositionMode::NCopies4 => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (2, 2), false, true)
        }
        ImpositionMode::OneUp => {
            build_nup_plan(config, source_page_count, selected_page_count, stream, (1, 1), false, false)
        }
        ImpositionMode::DeimposeBooklet | ImpositionMode::Split2Up => unreachable!(),
    }
}

fn build_signatures_plan(
    config: &JobConfig,
    source_page_count: usize,
    selected_page_count: usize,
    mut stream: Vec<(PageSource, Option<usize>)>,
    sheets_per_sig_opt: Option<usize>,
) -> ImpositionPlan {
    let initial_len = stream.len();
    let padded_total = if initial_len == 0 {
        4
    } else {
        ((initial_len + 3) / 4) * 4
    };
    let padded_blank_pages = padded_total - initial_len;
    while stream.len() < padded_total {
        stream.push((PageSource::Blank, None));
    }

    let pages_per_sig = match sheets_per_sig_opt {
        Some(k) => (k.max(1) * 4).min(padded_total).max(4),
        None => padded_total,
    };

    let chunks: Vec<&[(PageSource, Option<usize>)]> = stream.chunks(pages_per_sig).collect();
    let total_signatures = chunks.len();
    let mut sides = Vec::new();
    let mut physical_sheet_counter = 0usize;
    let max_creep_pt = mm_to_pt(config.creep_mm);

    for (sig_idx, chunk) in chunks.iter().enumerate() {
        let mut sig_pages: Vec<(PageSource, Option<usize>)> = chunk.to_vec();
        let sig_padded_len = ((sig_pages.len() + 3) / 4).max(1) * 4;
        while sig_pages.len() < sig_padded_len {
            sig_pages.push((PageSource::Blank, None));
        }

        let sheets_in_sig = sig_padded_len / 4;
        for s in 0..sheets_in_sig {
            physical_sheet_counter += 1;
            let creep_pt = if sheets_in_sig <= 1 {
                0.0
            } else {
                max_creep_pt * (s as f64) / ((sheets_in_sig - 1) as f64)
            };

            // Standard LTR saddle-stitch indices:
            // Recto: Left = N - 1 - 2s, Right = 2s
            // Verso: Left = 2s + 1,     Right = N - 2 - 2s
            let recto_outer_left = sig_pages[sig_padded_len - 1 - 2 * s];
            let recto_outer_right = sig_pages[2 * s];
            let verso_inner_left = sig_pages[2 * s + 1];
            let verso_inner_right = sig_pages[sig_padded_len - 2 - 2 * s];

            let (r_left, r_right) = if config.right_to_left {
                (recto_outer_right, recto_outer_left)
            } else {
                (recto_outer_left, recto_outer_right)
            };

            let (v_left, v_right) = if config.right_to_left {
                (verso_inner_right, verso_inner_left)
            } else {
                (verso_inner_left, verso_inner_right)
            };

            // Left slot shifts +creep_pt (rightward toward spine at center).
            // Right slot shifts -creep_pt (leftward toward spine at center).
            sides.push(ImposedSheetSide {
                physical_sheet_number: physical_sheet_counter,
                side: SheetSide::Recto,
                signature_index: sig_idx,
                total_signatures,
                sheet_index_in_signature: s,
                sheets_in_signature: sheets_in_sig,
                grid: (2, 1),
                slots: vec![
                    SlotPlacement {
                        col: 0,
                        row: 0,
                        source: r_left.0,
                        logical_reading_number: r_left.1,
                        creep_shift_x_pt: creep_pt,
                    },
                    SlotPlacement {
                        col: 1,
                        row: 0,
                        source: r_right.0,
                        logical_reading_number: r_right.1,
                        creep_shift_x_pt: -creep_pt,
                    },
                ],
            });

            sides.push(ImposedSheetSide {
                physical_sheet_number: physical_sheet_counter,
                side: SheetSide::Verso,
                signature_index: sig_idx,
                total_signatures,
                sheet_index_in_signature: s,
                sheets_in_signature: sheets_in_sig,
                grid: (2, 1),
                slots: vec![
                    SlotPlacement {
                        col: 0,
                        row: 0,
                        source: v_left.0,
                        logical_reading_number: v_left.1,
                        creep_shift_x_pt: creep_pt,
                    },
                    SlotPlacement {
                        col: 1,
                        row: 0,
                        source: v_right.0,
                        logical_reading_number: v_right.1,
                        creep_shift_x_pt: -creep_pt,
                    },
                ],
            });
        }
    }

    ImpositionPlan {
        mode: config.mode,
        source_page_count,
        selected_page_count,
        flyleaves_front: config.flyleaves_front,
        flyleaves_back: config.flyleaves_back,
        padded_blank_pages,
        total_logical_pages: stream.len(),
        total_physical_sheets: physical_sheet_counter,
        total_signatures,
        sides,
    }
}

/// 2-Up Guillotine Cut & Stack:
/// Stack A (Left on Recto, Right on Verso) holds pages `0 .. 2M`.
/// Stack B (Right on Recto, Left on Verso) holds pages `2M .. 4M`.
/// When printed duplex (long-edge flip), cut down the vertical center, and Stack A is placed on top of Stack B,
/// every leaf has `(2k+1, 2k+2)` front-and-back in consecutive order `1..=4M`.
fn build_cut_and_stack_2up(
    config: &JobConfig,
    source_page_count: usize,
    selected_page_count: usize,
    mut stream: Vec<(PageSource, Option<usize>)>,
) -> ImpositionPlan {
    let initial_len = stream.len();
    let padded_total = if initial_len == 0 {
        4
    } else {
        ((initial_len + 3) / 4) * 4
    };
    let padded_blank_pages = padded_total - initial_len;
    while stream.len() < padded_total {
        stream.push((PageSource::Blank, None));
    }

    let total_sheets = padded_total / 4;
    let half_pages = padded_total / 2; // 2 * total_sheets
    let mut sides = Vec::with_capacity(total_sheets * 2);

    for s in 0..total_sheets {
        let stack_a_front = stream[2 * s];
        let stack_a_back = stream[2 * s + 1];
        let stack_b_front = stream[half_pages + 2 * s];
        let stack_b_back = stream[half_pages + 2 * s + 1];

        // Recto: Col 0 = Stack A front, Col 1 = Stack B front
        sides.push(ImposedSheetSide {
            physical_sheet_number: s + 1,
            side: SheetSide::Recto,
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: s,
            sheets_in_signature: total_sheets,
            grid: (2, 1),
            slots: vec![
                SlotPlacement {
                    col: 0,
                    row: 0,
                    source: stack_a_front.0,
                    logical_reading_number: stack_a_front.1,
                    creep_shift_x_pt: 0.0,
                },
                SlotPlacement {
                    col: 1,
                    row: 0,
                    source: stack_b_front.0,
                    logical_reading_number: stack_b_front.1,
                    creep_shift_x_pt: 0.0,
                },
            ],
        });

        // Verso: Because duplex flips horizontally across the vertical spine,
        // the physical back of Recto Col 0 (Left) is Verso Col 1 (Right)!
        sides.push(ImposedSheetSide {
            physical_sheet_number: s + 1,
            side: SheetSide::Verso,
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: s,
            sheets_in_signature: total_sheets,
            grid: (2, 1),
            slots: vec![
                SlotPlacement {
                    col: 0,
                    row: 0,
                    source: stack_b_back.0,
                    logical_reading_number: stack_b_back.1,
                    creep_shift_x_pt: 0.0,
                },
                SlotPlacement {
                    col: 1,
                    row: 0,
                    source: stack_a_back.0,
                    logical_reading_number: stack_a_back.1,
                    creep_shift_x_pt: 0.0,
                },
            ],
        });
    }

    ImpositionPlan {
        mode: config.mode,
        source_page_count,
        selected_page_count,
        flyleaves_front: config.flyleaves_front,
        flyleaves_back: config.flyleaves_back,
        padded_blank_pages,
        total_logical_pages: stream.len(),
        total_physical_sheets: total_sheets,
        total_signatures: 1,
        sides,
    }
}

/// 4-Up Guillotine Cut & Stack (4 stacks: Top-Left, Top-Right, Bottom-Left, Bottom-Right).
fn build_cut_and_stack_4up(
    config: &JobConfig,
    source_page_count: usize,
    selected_page_count: usize,
    mut stream: Vec<(PageSource, Option<usize>)>,
) -> ImpositionPlan {
    let initial_len = stream.len();
    let padded_total = if initial_len == 0 {
        8
    } else {
        ((initial_len + 7) / 8) * 8
    };
    let padded_blank_pages = padded_total - initial_len;
    while stream.len() < padded_total {
        stream.push((PageSource::Blank, None));
    }

    let total_sheets = padded_total / 8;
    let stack_span = padded_total / 4; // 2 * total_sheets
    let mut sides = Vec::with_capacity(total_sheets * 2);

    for s in 0..total_sheets {
        let s0_f = stream[2 * s];
        let s0_b = stream[2 * s + 1];
        let s1_f = stream[stack_span + 2 * s];
        let s1_b = stream[stack_span + 2 * s + 1];
        let s2_f = stream[2 * stack_span + 2 * s];
        let s2_b = stream[2 * stack_span + 2 * s + 1];
        let s3_f = stream[3 * stack_span + 2 * s];
        let s3_b = stream[3 * stack_span + 2 * s + 1];

        // Recto: Top-Left=S0, Top-Right=S1, Bottom-Left=S2, Bottom-Right=S3
        sides.push(ImposedSheetSide {
            physical_sheet_number: s + 1,
            side: SheetSide::Recto,
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: s,
            sheets_in_signature: total_sheets,
            grid: (2, 2),
            slots: vec![
                SlotPlacement { col: 0, row: 0, source: s0_f.0, logical_reading_number: s0_f.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 1, row: 0, source: s1_f.0, logical_reading_number: s1_f.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 0, row: 1, source: s2_f.0, logical_reading_number: s2_f.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 1, row: 1, source: s3_f.0, logical_reading_number: s3_f.1, creep_shift_x_pt: 0.0 },
            ],
        });

        // Verso swaps horizontal columns (Col 0 <-> Col 1) so backs align with fronts
        sides.push(ImposedSheetSide {
            physical_sheet_number: s + 1,
            side: SheetSide::Verso,
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: s,
            sheets_in_signature: total_sheets,
            grid: (2, 2),
            slots: vec![
                SlotPlacement { col: 0, row: 0, source: s1_b.0, logical_reading_number: s1_b.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 1, row: 0, source: s0_b.0, logical_reading_number: s0_b.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 0, row: 1, source: s3_b.0, logical_reading_number: s3_b.1, creep_shift_x_pt: 0.0 },
                SlotPlacement { col: 1, row: 1, source: s2_b.0, logical_reading_number: s2_b.1, creep_shift_x_pt: 0.0 },
            ],
        });
    }

    ImpositionPlan {
        mode: config.mode,
        source_page_count,
        selected_page_count,
        flyleaves_front: config.flyleaves_front,
        flyleaves_back: config.flyleaves_back,
        padded_blank_pages,
        total_logical_pages: stream.len(),
        total_physical_sheets: total_sheets,
        total_signatures: 1,
        sides,
    }
}

/// De-Impose (`DeimposeBooklet`) or Split 2-Up spreads (`Split2Up`) into sequential single pages.
fn compute_deimpose_plan(
    config: &JobConfig,
    source_page_count: usize,
    selected_pages: &[usize],
) -> ImpositionPlan {
    let mut extracted_pages: Vec<PageSource> = Vec::new();

    if config.mode == ImpositionMode::Split2Up {
        for &p in selected_pages {
            if config.right_to_left {
                extracted_pages.push(PageSource::DocumentPage {
                    page_number: p,
                    crop: PageCropRegion::RightHalf,
                });
                extracted_pages.push(PageSource::DocumentPage {
                    page_number: p,
                    crop: PageCropRegion::LeftHalf,
                });
            } else {
                extracted_pages.push(PageSource::DocumentPage {
                    page_number: p,
                    crop: PageCropRegion::LeftHalf,
                });
                extracted_pages.push(PageSource::DocumentPage {
                    page_number: p,
                    crop: PageCropRegion::RightHalf,
                });
            }
        }
    } else {
        // DeimposeBooklet: reverses saddle-stitch booklet pairs of spread pages (Recto, Verso).
        // Each pair of input pages (2s, 2s+1) represents 1 folded sheet = 4 output pages:
        // Front (2s): Left = N - 2s, Right = 2s + 1
        // Back (2s+1): Left = 2s + 2, Right = N - 1 - 2s
        let m_sheets = (selected_pages.len() + 1) / 2;
        let total_reconstructed = m_sheets * 4;
        let mut reconstructed = vec![PageSource::Blank; total_reconstructed];

        for s in 0..m_sheets {
            let recto_page = selected_pages.get(2 * s).copied();
            let verso_page = selected_pages.get(2 * s + 1).copied();

            if let Some(rp) = recto_page {
                let (left_crop, right_crop) = if config.right_to_left {
                    (PageCropRegion::RightHalf, PageCropRegion::LeftHalf)
                } else {
                    (PageCropRegion::LeftHalf, PageCropRegion::RightHalf)
                };
                // Recto Right -> page index 2*s
                reconstructed[2 * s] = PageSource::DocumentPage {
                    page_number: rp,
                    crop: right_crop,
                };
                // Recto Left -> page index total_reconstructed - 1 - 2*s
                reconstructed[total_reconstructed - 1 - 2 * s] = PageSource::DocumentPage {
                    page_number: rp,
                    crop: left_crop,
                };
            }

            if let Some(vp) = verso_page {
                let (left_crop, right_crop) = if config.right_to_left {
                    (PageCropRegion::RightHalf, PageCropRegion::LeftHalf)
                } else {
                    (PageCropRegion::LeftHalf, PageCropRegion::RightHalf)
                };
                // Verso Left -> page index 2*s + 1
                reconstructed[2 * s + 1] = PageSource::DocumentPage {
                    page_number: vp,
                    crop: left_crop,
                };
                // Verso Right -> page index total_reconstructed - 2 - 2*s
                reconstructed[total_reconstructed - 2 - 2 * s] = PageSource::DocumentPage {
                    page_number: vp,
                    crop: right_crop,
                };
            }
        }
        extracted_pages = reconstructed;
    }

    let mut sides = Vec::with_capacity(extracted_pages.len());
    for (idx, src) in extracted_pages.iter().copied().enumerate() {
        sides.push(ImposedSheetSide {
            physical_sheet_number: (idx / 2) + 1,
            side: if idx % 2 == 0 {
                SheetSide::Recto
            } else {
                SheetSide::Verso
            },
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: idx / 2,
            sheets_in_signature: (extracted_pages.len() + 1) / 2,
            grid: (1, 1),
            slots: vec![SlotPlacement {
                col: 0,
                row: 0,
                source: src,
                logical_reading_number: Some(idx + 1),
                creep_shift_x_pt: 0.0,
            }],
        });
    }

    ImpositionPlan {
        mode: config.mode,
        source_page_count,
        selected_page_count: selected_pages.len(),
        flyleaves_front: 0,
        flyleaves_back: 0,
        padded_blank_pages: 0,
        total_logical_pages: extracted_pages.len(),
        total_physical_sheets: (sides.len() + 1) / 2,
        total_signatures: 1,
        sides,
    }
}

fn build_nup_plan(
    config: &JobConfig,
    source_page_count: usize,
    selected_page_count: usize,
    stream: Vec<(PageSource, Option<usize>)>,
    grid: (usize, usize),
    column_major: bool,
    repeat_each_page: bool,
) -> ImpositionPlan {
    let (cols, rows) = grid;
    let slots_per_side = cols * rows;
    let mut expanded: Vec<(PageSource, Option<usize>)> = Vec::new();

    if repeat_each_page {
        for item in &stream {
            for _ in 0..slots_per_side {
                expanded.push(*item);
            }
        }
    } else {
        expanded = stream.clone();
    }

    if expanded.is_empty() {
        expanded.push((PageSource::Blank, None));
    }

    let padded_len = ((expanded.len() + slots_per_side - 1) / slots_per_side) * slots_per_side;
    let padded_blank_pages = padded_len.saturating_sub(expanded.len());
    while expanded.len() < padded_len {
        expanded.push((PageSource::Blank, None));
    }

    let total_sides = padded_len / slots_per_side;
    let total_sheets = (total_sides + 1) / 2;
    let mut sides = Vec::with_capacity(total_sides);

    for (side_idx, chunk) in expanded.chunks(slots_per_side).enumerate() {
        let mut slots = Vec::with_capacity(slots_per_side);
        for (k, &(source, reading_num)) in chunk.iter().enumerate() {
            let (col, row) = if column_major {
                (k / rows, k % rows)
            } else {
                (k % cols, k / cols)
            };
            slots.push(SlotPlacement {
                col,
                row,
                source,
                logical_reading_number: reading_num,
                creep_shift_x_pt: 0.0,
            });
        }

        sides.push(ImposedSheetSide {
            physical_sheet_number: (side_idx / 2) + 1,
            side: if side_idx % 2 == 0 {
                SheetSide::Recto
            } else {
                SheetSide::Verso
            },
            signature_index: 0,
            total_signatures: 1,
            sheet_index_in_signature: side_idx / 2,
            sheets_in_signature: total_sheets,
            grid,
            slots,
        });
    }

    ImpositionPlan {
        mode: config.mode,
        source_page_count,
        selected_page_count,
        flyleaves_front: config.flyleaves_front,
        flyleaves_back: config.flyleaves_back,
        padded_blank_pages,
        total_logical_pages: stream.len() + padded_blank_pages,
        total_physical_sheets: total_sheets,
        total_signatures: 1,
        sides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_8_page_saddle_stitch_booklet() {
        let cfg = JobConfig::default();
        let plan = compute_imposition_plan(&cfg, 8);
        assert_eq!(plan.total_physical_sheets, 2);
        assert_eq!(plan.sides.len(), 4);

        // Sheet 1 Recto: [8, 1]
        assert_eq!(
            plan.sides[0].slots[0].source,
            PageSource::DocumentPage {
                page_number: 8,
                crop: PageCropRegion::FullPage
            }
        );
        assert_eq!(
            plan.sides[0].slots[1].source,
            PageSource::DocumentPage {
                page_number: 1,
                crop: PageCropRegion::FullPage
            }
        );
        // Sheet 1 Verso: [2, 7]
        assert_eq!(
            plan.sides[1].slots[0].source,
            PageSource::DocumentPage {
                page_number: 2,
                crop: PageCropRegion::FullPage
            }
        );
        assert_eq!(
            plan.sides[1].slots[1].source,
            PageSource::DocumentPage {
                page_number: 7,
                crop: PageCropRegion::FullPage
            }
        );
        // Sheet 2 Recto: [6, 3]
        assert_eq!(
            plan.sides[2].slots[0].source,
            PageSource::DocumentPage {
                page_number: 6,
                crop: PageCropRegion::FullPage
            }
        );
        assert_eq!(
            plan.sides[2].slots[1].source,
            PageSource::DocumentPage {
                page_number: 3,
                crop: PageCropRegion::FullPage
            }
        );
        // Sheet 2 Verso: [4, 5]
        assert_eq!(
            plan.sides[3].slots[0].source,
            PageSource::DocumentPage {
                page_number: 4,
                crop: PageCropRegion::FullPage
            }
        );
        assert_eq!(
            plan.sides[3].slots[1].source,
            PageSource::DocumentPage {
                page_number: 5,
                crop: PageCropRegion::FullPage
            }
        );
    }

    #[test]
    fn test_cut_and_stack_2up_8_pages() {
        let mut cfg = JobConfig::default();
        cfg.mode = ImpositionMode::CutAndStack2Up;
        let plan = compute_imposition_plan(&cfg, 8);
        assert_eq!(plan.total_physical_sheets, 2);

        // Sheet 1 Recto: Left=1, Right=5
        assert_eq!(plan.sides[0].slots[0].logical_reading_number, Some(1));
        assert_eq!(plan.sides[0].slots[1].logical_reading_number, Some(5));
        // Sheet 1 Verso: Left=6 (back of 5), Right=2 (back of 1)
        assert_eq!(plan.sides[1].slots[0].logical_reading_number, Some(6));
        assert_eq!(plan.sides[1].slots[1].logical_reading_number, Some(2));
    }

    #[test]
    fn test_deimpose_booklet_roundtrip() {
        let mut cfg = JobConfig::default();
        cfg.mode = ImpositionMode::DeimposeBooklet;
        // 4 spread sides (2 sheets = 8 logical pages)
        let plan = compute_imposition_plan(&cfg, 4);
        assert_eq!(plan.sides.len(), 8);
        // Page 1 comes from RightHalf of spread 1 (Recto sheet 1)
        assert_eq!(
            plan.sides[0].slots[0].source,
            PageSource::DocumentPage {
                page_number: 1,
                crop: PageCropRegion::RightHalf
            }
        );
        // Page 8 comes from LeftHalf of spread 1 (Recto sheet 1)
        assert_eq!(
            plan.sides[7].slots[0].source,
            PageSource::DocumentPage {
                page_number: 1,
                crop: PageCropRegion::LeftHalf
            }
        );
    }

    #[test]
    fn test_flyleaves_and_signatures() {
        let mut cfg = JobConfig::default();
        cfg.mode = ImpositionMode::Signatures;
        cfg.sheets_per_signature = 2; // 8 pages per signature
        cfg.flyleaves_front = 2;
        cfg.flyleaves_back = 2;
        let plan = compute_imposition_plan(&cfg, 12);
        // 12 + 2 + 2 = 16 pages -> 2 signatures of 8 pages (4 physical sheets)
        assert_eq!(plan.total_signatures, 2);
        assert_eq!(plan.total_physical_sheets, 4);
        assert_eq!(plan.padded_blank_pages, 0);
    }
}
