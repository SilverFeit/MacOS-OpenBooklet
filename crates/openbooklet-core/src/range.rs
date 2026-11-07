//! Page range parser supporting expressions like `all`, `odd`, `even`, `1-16`, `1,3,5-8`.

/// Parse a page range filter expression against a document with `total_pages` (1-indexed).
pub fn parse_page_range(expr: &str, total_pages: usize) -> Vec<usize> {
    if total_pages == 0 {
        return Vec::new();
    }

    let cleaned = expr.trim().to_ascii_lowercase();
    if cleaned.is_empty() || cleaned == "all" || cleaned == "todas" || cleaned == "*" {
        return (1..=total_pages).collect();
    }
    if cleaned == "odd" || cleaned == "impares" {
        return (1..=total_pages).filter(|p| p % 2 == 1).collect();
    }
    if cleaned == "even" || cleaned == "pares" {
        return (1..=total_pages).filter(|p| p % 2 == 0).collect();
    }

    let mut out = Vec::new();
    for token in cleaned.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let tok = token.trim();
        if tok.is_empty() {
            continue;
        }
        if let Some((start_s, end_s)) = tok.split_once('-') {
            let start = start_s.trim().parse::<usize>().ok();
            let end = end_s.trim().parse::<usize>().ok();
            if let (Some(a), Some(b)) = (start, end) {
                let a_clamped = a.clamp(1, total_pages);
                let b_clamped = b.clamp(1, total_pages);
                if a_clamped <= b_clamped {
                    for p in a_clamped..=b_clamped {
                        out.push(p);
                    }
                } else {
                    for p in (b_clamped..=a_clamped).rev() {
                        out.push(p);
                    }
                }
            }
        } else if let Ok(single) = tok.parse::<usize>() {
            if (1..=total_pages).contains(&single) {
                out.push(single);
            }
        }
    }

    if out.is_empty() {
        (1..=total_pages).collect()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_keywords() {
        assert_eq!(parse_page_range("all", 6), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(parse_page_range("odd", 6), vec![1, 3, 5]);
        assert_eq!(parse_page_range("even", 6), vec![2, 4, 6]);
        assert_eq!(parse_page_range("impares", 5), vec![1, 3, 5]);
        assert_eq!(parse_page_range("pares", 5), vec![2, 4]);
    }

    #[test]
    fn test_range_intervals_and_reverse() {
        assert_eq!(
            parse_page_range("1-3, 6, 8-7", 10),
            vec![1, 2, 3, 6, 8, 7]
        );
    }
}
