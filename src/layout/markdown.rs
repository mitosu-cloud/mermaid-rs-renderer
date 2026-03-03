use super::types::{SpanStyle, TextSpan};

/// Parse inline markdown formatting from a string.
/// Only supports `**bold**`, `*italic*`, and `***bold+italic***`.
/// Unmatched delimiters are treated as literal `*` characters.
pub fn parse_markdown_spans(input: &str) -> Vec<TextSpan> {
    let mut spans: Vec<TextSpan> = Vec::new();
    let mut current = String::new();
    let current_style = SpanStyle::default();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '*' {
            // Count consecutive asterisks
            let _star_start = i;
            let mut star_count = 0;
            while i < len && chars[i] == '*' {
                star_count += 1;
                i += 1;
            }

            // Try to find matching closing delimiter
            if star_count >= 1 && star_count <= 3 && i < len {
                let delim_len = star_count.min(3);
                if let Some(close_pos) = find_closing_delimiter(&chars, i, delim_len) {
                    // Push any accumulated text
                    if !current.is_empty() {
                        spans.push(TextSpan {
                            text: current.clone(),
                            style: current_style,
                        });
                        current.clear();
                    }

                    let inner: String = chars[i..close_pos].iter().collect();
                    let style = match delim_len {
                        3 => SpanStyle {
                            bold: true,
                            italic: true,
                        },
                        2 => SpanStyle {
                            bold: true,
                            italic: false,
                        },
                        1 => SpanStyle {
                            bold: false,
                            italic: true,
                        },
                        _ => SpanStyle::default(),
                    };

                    // Handle any extra stars from the opening (> 3)
                    let extra_open = star_count.saturating_sub(3);
                    if extra_open > 0 {
                        current.extend(std::iter::repeat('*').take(extra_open));
                    }

                    if !inner.is_empty() {
                        spans.push(TextSpan {
                            text: inner,
                            style,
                        });
                    }
                    i = close_pos + delim_len;
                    continue;
                }
            }

            // No matching closer found - treat as literal
            for _ in 0..star_count {
                current.push('*');
            }
        } else {
            current.push(chars[i]);
            i += 1;
        }
    }

    if !current.is_empty() {
        spans.push(TextSpan {
            text: current,
            style: current_style,
        });
    }

    if spans.is_empty() {
        spans.push(TextSpan {
            text: String::new(),
            style: SpanStyle::default(),
        });
    }

    spans
}

/// Find the position of a closing delimiter (sequence of `delim_len` asterisks)
/// starting from position `start` in the character array.
fn find_closing_delimiter(chars: &[char], start: usize, delim_len: usize) -> Option<usize> {
    let len = chars.len();
    let mut i = start;
    while i <= len.saturating_sub(delim_len) {
        if chars[i] == '*' {
            let mut count = 0;
            let pos = i;
            while i < len && chars[i] == '*' {
                count += 1;
                i += 1;
            }
            if count == delim_len {
                return Some(pos);
            }
            // If we found more or fewer stars, not a match; continue searching
            continue;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let spans = parse_markdown_spans("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello world");
        assert!(!spans[0].style.bold);
        assert!(!spans[0].style.italic);
    }

    #[test]
    fn bold_text() {
        let spans = parse_markdown_spans("**bold**");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "bold");
        assert!(spans[0].style.bold);
        assert!(!spans[0].style.italic);
    }

    #[test]
    fn italic_text() {
        let spans = parse_markdown_spans("*italic*");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "italic");
        assert!(!spans[0].style.bold);
        assert!(spans[0].style.italic);
    }

    #[test]
    fn bold_italic_text() {
        let spans = parse_markdown_spans("***both***");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "both");
        assert!(spans[0].style.bold);
        assert!(spans[0].style.italic);
    }

    #[test]
    fn mixed_formatting() {
        let spans = parse_markdown_spans("**bold** and *italic*");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "bold");
        assert!(spans[0].style.bold);
        assert_eq!(spans[1].text, " and ");
        assert!(!spans[1].style.bold);
        assert_eq!(spans[2].text, "italic");
        assert!(spans[2].style.italic);
    }

    #[test]
    fn unmatched_stars_are_literal() {
        let spans = parse_markdown_spans("hello * world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello * world");
    }

    #[test]
    fn empty_string() {
        let spans = parse_markdown_spans("");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "");
    }

    #[test]
    fn plain_text_with_bold_in_middle() {
        let spans = parse_markdown_spans("start **middle** end");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "start ");
        assert!(!spans[0].style.bold);
        assert_eq!(spans[1].text, "middle");
        assert!(spans[1].style.bold);
        assert_eq!(spans[2].text, " end");
        assert!(!spans[2].style.bold);
    }
}
