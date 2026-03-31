use regex::Regex;
use std::sync::LazyLock;

use crate::config::LayoutConfig;
use crate::text_metrics;
use crate::theme::Theme;

use super::markdown::parse_markdown_spans;
use super::{TextBlock, TextLine};

static HTML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"</?[a-zA-Z][a-zA-Z0-9]*[^>]*>").unwrap());

/// Check whether a label string contains HTML formatting tags (not just `<br>`).
pub(super) fn has_html_formatting(text: &str) -> bool {
    // Quick pre-check before hitting the regex.
    if !text.contains('<') {
        return false;
    }
    // Match any HTML tag that is NOT <br>, <br/>, or <br />.
    for m in HTML_TAG_RE.find_iter(text) {
        let tag = m.as_str().to_ascii_lowercase();
        if tag.starts_with("<br") {
            continue;
        }
        return true;
    }
    false
}

/// Convert HTML formatting tags to Markdown equivalents and normalise
/// line-break tags to `\n`.  Returns the normalised string.
pub(super) fn normalize_html_label(text: &str) -> String {
    let mut s = text.to_string();
    // Line-break tags → newline (case-insensitive).
    s = regex::RegexBuilder::new(r"<br\s*/?>")
        .case_insensitive(true)
        .build()
        .unwrap()
        .replace_all(&s, "\n")
        .into_owned();
    // Bold tags → markdown bold.
    for tag in &["<b>", "</b>", "<strong>", "</strong>"] {
        s = replace_tag_ci(&s, tag, "**");
    }
    // Italic tags → markdown italic.
    for tag in &["<i>", "</i>", "<em>", "</em>"] {
        s = replace_tag_ci(&s, tag, "*");
    }
    // Strip any remaining HTML tags.
    s = HTML_TAG_RE.replace_all(&s, "").into_owned();
    s
}

/// Case-insensitive replacement of a specific HTML tag with a substitute string.
fn replace_tag_ci(text: &str, tag: &str, replacement: &str) -> String {
    let re = regex::RegexBuilder::new(&regex::escape(tag))
        .case_insensitive(true)
        .build()
        .unwrap();
    re.replace_all(text, replacement).into_owned()
}

pub(super) fn measure_label(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    // Intercept HTML-formatted labels and route them through the
    // markdown measurement path so <b>, <i>, <br/> etc. are honoured.
    if has_html_formatting(text) {
        let normalized = normalize_html_label(text);
        return measure_markdown_label(&normalized, theme, config);
    }
    // Mermaid's layout sizing appears to use a baseline font size (~16px)
    // even when the configured theme font size is smaller. Using that
    // baseline improves parity with mermaid-cli node sizes.
    let measure_font_size = theme.font_size.max(16.0);
    measure_label_with_font_size(
        text,
        measure_font_size,
        config,
        true,
        theme.font_family.as_str(),
    )
}

pub(super) fn measure_label_with_font_size(
    text: &str,
    font_size: f32,
    config: &LayoutConfig,
    wrap: bool,
    font_family: &str,
) -> TextBlock {
    // Intercept HTML-formatted labels – convert to markdown so styling
    // and line-breaks are preserved regardless of how we were called.
    if has_html_formatting(text) {
        let normalized = normalize_html_label(text);
        let theme_stub = crate::theme::Theme {
            font_size,
            font_family: font_family.to_string(),
            ..crate::theme::Theme::modern()
        };
        return measure_markdown_label(&normalized, &theme_stub, config);
    }
    let raw_lines = split_lines(text);
    let mut lines = Vec::new();
    let fast_metrics = config.fast_text_metrics;
    let max_width_px = max_label_width_px(
        config.max_label_width_chars,
        font_size,
        font_family,
        fast_metrics,
    );
    for line in raw_lines {
        if wrap {
            let wrapped = wrap_line(&line, max_width_px, font_size, font_family, fast_metrics);
            lines.extend(wrapped);
        } else {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
    let max_width = lines
        .iter()
        .map(|line| text_width(line, font_size, font_family, fast_metrics))
        .fold(0.0, f32::max);
    // Guard width: a safety floor based on character count × average width.
    // With fast_text_metrics the per-char fallback is already the primary
    // measurement, so the guard would always dominate and inflate widths.
    // Only apply it when using the font library (which can return 0 for
    // missing glyphs).
    let width = if fast_metrics {
        max_width
    } else {
        let avg_char = average_char_width(font_family, font_size, fast_metrics);
        let guard_width = max_len as f32 * avg_char;
        max_width.max(guard_width)
    };
    let height = lines.len() as f32 * font_size * config.label_line_height;

    TextBlock {
        lines: lines.into_iter().map(TextLine::plain).collect(),
        width,
        height,
    }
}

pub(super) fn char_width_factor(ch: char) -> f32 {
    // Calibrated per-character widths against mermaid-cli output using the
    // default font stack and a 16px measurement baseline.
    match ch {
        ' ' => 0.306,
        '\\' | '.' | ',' | ':' | ';' | '|' | '!' | '(' | ')' | '[' | ']' | '{' | '}' => 0.321,
        'A' => 0.652,
        'B' => 0.648,
        'C' => 0.734,
        'D' => 0.723,
        'E' => 0.594,
        'F' => 0.575,
        'G' | 'H' => 0.742,
        'I' => 0.272,
        'J' => 0.557,
        'K' => 0.648,
        'L' => 0.559,
        'M' => 0.903,
        'N' => 0.763,
        'O' => 0.754,
        'P' => 0.623,
        'Q' => 0.755,
        'R' => 0.637,
        'S' => 0.633,
        'T' => 0.599,
        'U' => 0.746,
        'V' => 0.661,
        'W' => 0.958,
        'X' => 0.655,
        'Y' => 0.646,
        'Z' => 0.621,
        'a' => 0.550,
        'b' => 0.603,
        'c' => 0.547,
        'd' => 0.609,
        'e' => 0.570,
        'f' => 0.340,
        'g' | 'h' => 0.600,
        'i' => 0.235,
        'j' => 0.227,
        'k' => 0.522,
        'l' => 0.239,
        'm' => 0.867,
        'n' => 0.585,
        'o' => 0.574,
        'p' => 0.595,
        'q' => 0.585,
        'r' => 0.364,
        's' => 0.523,
        't' => 0.305,
        'u' => 0.585,
        'v' => 0.545,
        'w' => 0.811,
        'x' => 0.538,
        'y' => 0.556,
        'z' => 0.550,
        '0' => 0.613,
        '1' => 0.396,
        '2' => 0.609,
        '3' => 0.597,
        '4' => 0.614,
        '5' => 0.586,
        '6' => 0.608,
        '7' => 0.559,
        '8' => 0.611,
        '9' => 0.595,
        '@' | '#' | '%' | '&' => 0.946,
        _ => 0.568,
    }
}

pub(super) fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = text.replace("<br/>", "\n").replace("<br>", "\n");
    current = current.replace("\\n", "\n");
    for line in current.split('\n') {
        lines.push(line.trim().to_string());
    }
    lines
}

pub(super) fn wrap_line(
    line: &str,
    max_width: f32,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
) -> Vec<String> {
    if text_width(line, font_size, font_family, fast_metrics) <= max_width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if text_width(&candidate, font_size, font_family, fast_metrics) > max_width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            current.push_str(word);
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(super) fn text_width(text: &str, font_size: f32, font_family: &str, fast_metrics: bool) -> f32 {
    if fast_metrics && text.is_ascii() {
        return fallback_text_width(text, font_size);
    }
    text_metrics::measure_text_width(text, font_size, font_family)
        .unwrap_or_else(|| fallback_text_width(text, font_size))
}

fn fallback_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().map(char_width_factor).sum::<f32>() * font_size
}

fn average_char_width(font_family: &str, font_size: f32, fast_metrics: bool) -> f32 {
    if fast_metrics {
        return font_size * 0.56;
    }
    text_metrics::average_char_width(font_family, font_size).unwrap_or(font_size * 0.56)
}

fn max_label_width_px(
    max_chars: usize,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
) -> f32 {
    let avg_char = average_char_width(font_family, font_size, fast_metrics);
    (max_chars.max(1) as f32) * avg_char
}

const BOLD_WIDTH_MULTIPLIER: f32 = 1.07;

/// Measure a markdown-formatted label. Splits on literal `\n` only (no `<br/>` processing),
/// parses each line via `parse_markdown_spans()`, measures span widths with a 1.07x
/// multiplier for bold text, and returns a `TextBlock` with formatted `TextLine`s.
pub(super) fn measure_markdown_label(
    text: &str,
    theme: &Theme,
    config: &LayoutConfig,
) -> TextBlock {
    let font_size = theme.font_size.max(16.0);
    let font_family = theme.font_family.as_str();
    let fast_metrics = config.fast_text_metrics;

    // Markdown strings split on literal newlines only (no <br/> processing).
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let mut lines: Vec<TextLine> = Vec::new();
    let mut max_width: f32 = 0.0;

    for raw in &raw_lines {
        let spans = parse_markdown_spans(raw);
        let mut line_width: f32 = 0.0;
        for span in &spans {
            let w = text_width(&span.text, font_size, font_family, fast_metrics);
            if span.style.bold {
                line_width += w * BOLD_WIDTH_MULTIPLIER;
            } else {
                line_width += w;
            }
        }
        max_width = max_width.max(line_width);
        lines.push(TextLine { spans });
    }

    if lines.is_empty() {
        lines.push(TextLine::plain(String::new()));
    }

    let height = lines.len() as f32 * font_size * config.label_line_height;

    TextBlock {
        lines,
        width: max_width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lines_handles_br_tags() {
        assert_eq!(split_lines("a<br/>b"), vec!["a", "b"]);
        assert_eq!(split_lines("a<br>b"), vec!["a", "b"]);
        assert_eq!(split_lines("a\\nb"), vec!["a", "b"]);
    }

    #[test]
    fn split_lines_trims_whitespace() {
        assert_eq!(split_lines("  hello  \n  world  "), vec!["hello", "world"]);
    }

    #[test]
    fn char_width_factor_returns_positive_values() {
        for ch in ['a', 'Z', ' ', '0', '@', '\u{4e2d}'] {
            assert!(char_width_factor(ch) > 0.0, "char {:?} has zero width", ch);
        }
    }

    #[test]
    fn fallback_text_width_scales_with_font_size() {
        let w16 = fallback_text_width("Hello", 16.0);
        let w32 = fallback_text_width("Hello", 32.0);
        assert!(
            (w32 - w16 * 2.0).abs() < 0.01,
            "width should double with font size"
        );
    }

    #[test]
    fn wrap_line_does_not_wrap_short_text() {
        let result = wrap_line("short", 1000.0, 16.0, "sans-serif", true);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn wrap_line_splits_long_text() {
        let result = wrap_line(
            "this is a rather long line that should be wrapped",
            100.0,
            16.0,
            "sans-serif",
            true,
        );
        assert!(result.len() > 1, "expected wrapping, got {:?}", result);
    }

    #[test]
    fn measure_label_produces_nonempty_block() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_label("Hello world", &theme, &config);
        assert!(!block.lines.is_empty());
        assert!(block.width > 0.0);
        assert!(block.height > 0.0);
    }

    #[test]
    fn measure_label_empty_string_produces_single_line() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_label("", &theme, &config);
        assert_eq!(block.lines.len(), 1);
    }

    #[test]
    fn measure_markdown_label_bold_wider_than_plain() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let plain = measure_markdown_label("hello", &theme, &config);
        let bold = measure_markdown_label("**hello**", &theme, &config);
        assert!(
            bold.width > plain.width,
            "bold markdown should be wider than plain markdown: {} vs {}",
            bold.width,
            plain.width
        );
    }

    #[test]
    fn measure_markdown_label_has_formatting() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_markdown_label("**bold** and *italic*", &theme, &config);
        assert_eq!(block.lines.len(), 1);
        assert!(block.lines[0].has_formatting());
    }

    #[test]
    fn measure_markdown_label_multiline() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_markdown_label("line1\nline2", &theme, &config);
        assert_eq!(block.lines.len(), 2);
    }

    #[test]
    fn has_html_formatting_detects_bold_tags() {
        assert!(has_html_formatting("<b>bold</b>"));
        assert!(has_html_formatting("<B>Bold</B>"));
        assert!(has_html_formatting("<i>italic</i>"));
        assert!(has_html_formatting("<strong>strong</strong>"));
    }

    #[test]
    fn has_html_formatting_ignores_br_tags() {
        assert!(!has_html_formatting("hello<br/>world"));
        assert!(!has_html_formatting("hello<br>world"));
        assert!(!has_html_formatting("no tags here"));
    }

    #[test]
    fn normalize_html_label_converts_bold() {
        assert_eq!(
            normalize_html_label("<b>Node A</b><br/>10.0.0.1"),
            "**Node A**\n10.0.0.1"
        );
    }

    #[test]
    fn normalize_html_label_converts_italic() {
        assert_eq!(
            normalize_html_label("<i>italic</i> text"),
            "*italic* text"
        );
    }

    #[test]
    fn normalize_html_label_strips_unknown_tags() {
        assert_eq!(
            normalize_html_label("<u>underline</u> <font>text</font>"),
            "underline text"
        );
    }

    #[test]
    fn normalize_html_label_handles_emoji() {
        let result = normalize_html_label("<b>Node A</b><br/>⭐ Leader");
        assert_eq!(result, "**Node A**\n⭐ Leader");
    }
}
