use serde::{Deserialize, Serialize};

const MERMAID_GIT_COLORS: [&str; 8] = [
    "hsl(240, 100%, 46.2745098039%)",
    "hsl(60, 100%, 43.5294117647%)",
    "hsl(80, 100%, 46.2745098039%)",
    "hsl(210, 100%, 46.2745098039%)",
    "hsl(180, 100%, 46.2745098039%)",
    "hsl(150, 100%, 46.2745098039%)",
    "hsl(300, 100%, 46.2745098039%)",
    "hsl(0, 100%, 46.2745098039%)",
];

const MERMAID_GIT_INV_COLORS: [&str; 8] = [
    "hsl(60, 100%, 3.7254901961%)",
    "rgb(0, 0, 160.5)",
    "rgb(48.8333333334, 0, 146.5000000001)",
    "rgb(146.5000000001, 73.2500000001, 0)",
    "rgb(146.5000000001, 0, 0)",
    "rgb(146.5000000001, 0, 73.2500000001)",
    "rgb(0, 146.5000000001, 0)",
    "rgb(0, 146.5000000001, 146.5000000001)",
];

const MERMAID_GIT_BRANCH_LABEL_COLORS: [&str; 8] = [
    "#ffffff", "black", "black", "#ffffff", "black", "black", "black", "black",
];

const MERMAID_GIT_COMMIT_LABEL_COLOR: &str = "#000021";
const MERMAID_GIT_COMMIT_LABEL_BG: &str = "#ffffde";
const MERMAID_GIT_TAG_LABEL_COLOR: &str = "#131300";
const MERMAID_GIT_TAG_LABEL_BG: &str = "#ECECFF";
const MERMAID_GIT_TAG_LABEL_BORDER: &str = "hsl(240, 60%, 86.2745098039%)";
const MERMAID_TEXT_COLOR: &str = "#333";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub font_family: String,
    pub font_size: f32,
    pub primary_color: String,
    pub primary_text_color: String,
    pub primary_border_color: String,
    pub line_color: String,
    pub secondary_color: String,
    pub tertiary_color: String,
    pub edge_label_background: String,
    pub cluster_background: String,
    pub cluster_border: String,
    pub background: String,
    pub sequence_actor_fill: String,
    pub sequence_actor_border: String,
    pub sequence_actor_line: String,
    pub sequence_note_fill: String,
    pub sequence_note_border: String,
    pub sequence_activation_fill: String,
    pub sequence_activation_border: String,
    pub text_color: String,
    pub git_colors: [String; 8],
    pub git_inv_colors: [String; 8],
    pub git_branch_label_colors: [String; 8],
    pub git_commit_label_color: String,
    pub git_commit_label_background: String,
    pub git_tag_label_color: String,
    pub git_tag_label_background: String,
    pub git_tag_label_border: String,
    pub pie_colors: [String; 12],
    pub pie_title_text_size: f32,
    pub pie_title_text_color: String,
    pub pie_section_text_size: f32,
    pub pie_section_text_color: String,
    pub pie_legend_text_size: f32,
    pub pie_legend_text_color: String,
    pub pie_stroke_color: String,
    pub pie_stroke_width: f32,
    pub pie_outer_stroke_width: f32,
    pub pie_outer_stroke_color: String,
    pub pie_opacity: f32,
    /// Custom cScale colors from themeVariables (cScale0, cScale1, ...).
    /// Used by timeline diagrams. Empty means use the default HSL palette.
    #[serde(default)]
    pub cscale_colors: Vec<String>,
}

impl Theme {
    /// Select a theme by name. Returns `None` if the name is unrecognized.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "default" | "mermaid" => Some(Self::mermaid_default()),
            "modern" => Some(Self::modern()),
            "dark" => Some(Self::dark()),
            "neutral" => Some(Self::neutral()),
            "forest" => Some(Self::forest()),
            "base" => Some(Self::base()),
            _ => None,
        }
    }

    pub fn mermaid_default() -> Self {
        let primary_color = "#ECECFF".to_string();
        let secondary_color = "#FFFFDE".to_string();
        let tertiary_color = "#ECECFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#7B88A8".to_string(),
            line_color: "#2F3B4D".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "rgba(248,250,252, 0.92)".to_string(),
            cluster_background: "#FFFFDE".to_string(),
            cluster_border: "#AAAA33".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#ECECFF".to_string(),
            // Light lavender, matching mermaid.js's CSS-resolved
            // `hsl(259.6, 59.78%, 87.9%)` for `.actor` / `.actor-line`.
            // Previously #9370DB which is the title color, much darker
            // than the browser-rendered actor border.
            sequence_actor_border: "#D2C7E4".to_string(),
            sequence_actor_line: "#999999".to_string(),
            sequence_note_fill: "#FFF5AD".to_string(),
            sequence_note_border: "#AAAA33".to_string(),
            sequence_activation_fill: "#EDF2AE".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: MERMAID_TEXT_COLOR.to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS.map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: MERMAID_TEXT_COLOR.to_string(),
            pie_stroke_color: "#000000".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#000000".to_string(),
            pie_opacity: 0.7,
            cscale_colors: Vec::new(),
        }
    }

    pub fn modern() -> Self {
        let primary_color = "#F8FAFC".to_string();
        let secondary_color = "#E2E8F0".to_string();
        let tertiary_color = "#FFFFFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "Inter, ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif"
                .to_string(),
            font_size: 14.0,
            primary_color,
            primary_text_color: "#0F172A".to_string(),
            primary_border_color: "#94A3B8".to_string(),
            line_color: "#64748B".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#FFFFFF".to_string(),
            cluster_background: "#F1F5F9".to_string(),
            cluster_border: "#CBD5E1".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#F8FAFC".to_string(),
            sequence_actor_border: "#94A3B8".to_string(),
            sequence_actor_line: "#64748B".to_string(),
            sequence_note_fill: "#FFF7ED".to_string(),
            sequence_note_border: "#FDBA74".to_string(),
            sequence_activation_fill: "#E2E8F0".to_string(),
            sequence_activation_border: "#94A3B8".to_string(),
            text_color: "#0F172A".to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS.map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#0F172A".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#0F172A".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#0F172A".to_string(),
            pie_stroke_color: "#334155".to_string(),
            pie_stroke_width: 1.6,
            pie_outer_stroke_width: 1.6,
            pie_outer_stroke_color: "#CBD5E1".to_string(),
            pie_opacity: 0.85,
            cscale_colors: Vec::new(),
        }
    }
    /// Official Mermaid "dark" theme — dark background with bright accents.
    pub fn dark() -> Self {
        let primary_color = "#1f2020".to_string();
        let secondary_color = "#FFFFDE".to_string();
        let tertiary_color = "#2B2F3A".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#E0E0E0".to_string(),
            primary_border_color: "#81B1DB".to_string(),
            line_color: "#A0AEC0".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "rgba(30,35,45, 0.9)".to_string(),
            cluster_background: "#2B2F3A".to_string(),
            cluster_border: "#3E4452".to_string(),
            background: "#1A1D23".to_string(),
            sequence_actor_fill: "#2B2F3A".to_string(),
            sequence_actor_border: "#81B1DB".to_string(),
            sequence_actor_line: "#A0AEC0".to_string(),
            sequence_note_fill: "#3E4452".to_string(),
            sequence_note_border: "#81B1DB".to_string(),
            sequence_activation_fill: "#2B2F3A".to_string(),
            sequence_activation_border: "#81B1DB".to_string(),
            text_color: "#CCCCCC".to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS
                .map(|value| value.to_string()),
            git_commit_label_color: "#E0E0E0".to_string(),
            git_commit_label_background: "#2B2F3A".to_string(),
            git_tag_label_color: "#E0E0E0".to_string(),
            git_tag_label_background: "#3E4452".to_string(),
            git_tag_label_border: "#81B1DB".to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#E0E0E0".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#E0E0E0".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#E0E0E0".to_string(),
            pie_stroke_color: "#A0AEC0".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#3E4452".to_string(),
            pie_opacity: 0.85,
            cscale_colors: Vec::new(),
        }
    }

    /// Official Mermaid "neutral" theme — black and white, no color.
    pub fn neutral() -> Self {
        let primary_color = "#EAEAEA".to_string();
        let secondary_color = "#FFFFDE".to_string();
        let tertiary_color = "#F5F5F5".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#999999".to_string(),
            line_color: "#666666".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "rgba(255,255,255, 0.9)".to_string(),
            cluster_background: "#F5F5F5".to_string(),
            cluster_border: "#999999".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#ECECFF".to_string(),
            sequence_actor_border: "#9370DB".to_string(),
            sequence_actor_line: "#9370DB".to_string(),
            sequence_note_fill: "#FFF5AD".to_string(),
            sequence_note_border: "#999999".to_string(),
            sequence_activation_fill: "#F4F4F4".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: "#333333".to_string(),
            git_colors: [
                "#000000", "#666666", "#999999", "#333333", "#AAAAAA", "#CCCCCC", "#444444",
                "#777777",
            ]
            .map(|v| v.to_string()),
            git_inv_colors: [
                "#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF", "#000000", "#000000", "#FFFFFF",
                "#FFFFFF",
            ]
            .map(|v| v.to_string()),
            git_branch_label_colors: [
                "#ffffff", "#ffffff", "#ffffff", "#ffffff", "black", "black", "#ffffff", "#ffffff",
            ]
            .map(|v| v.to_string()),
            git_commit_label_color: "#333333".to_string(),
            git_commit_label_background: "#EAEAEA".to_string(),
            git_tag_label_color: "#333333".to_string(),
            git_tag_label_background: "#EAEAEA".to_string(),
            git_tag_label_border: "#999999".to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#333333".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#333333".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#333333".to_string(),
            pie_stroke_color: "#666666".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#999999".to_string(),
            pie_opacity: 0.7,
            cscale_colors: Vec::new(),
        }
    }

    /// Official Mermaid "forest" theme — green tones inspired by nature.
    pub fn forest() -> Self {
        let primary_color = "#cde498".to_string();
        let secondary_color = "#cdffb2".to_string();
        let tertiary_color = "#EEE8D5".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#6EAA49".to_string(),
            line_color: "#2C5F2D".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "rgba(255,255,255, 0.92)".to_string(),
            cluster_background: "#EBF3E1".to_string(),
            cluster_border: "#6EAA49".to_string(),
            background: "#FFFFFF".to_string(),
            sequence_actor_fill: "#C6E5B1".to_string(),
            sequence_actor_border: "#2C5F2D".to_string(),
            sequence_actor_line: "#6EAA49".to_string(),
            sequence_note_fill: "#FFF5AD".to_string(),
            sequence_note_border: "#AAAA33".to_string(),
            sequence_activation_fill: "#E8F5E1".to_string(),
            sequence_activation_border: "#6EAA49".to_string(),
            text_color: "#333333".to_string(),
            git_colors: [
                "#2C5F2D", "#6EAA49", "#97D077", "#B5D99C", "#4A7C3F", "#357235", "#228B22",
                "#8FBC8F",
            ]
            .map(|v| v.to_string()),
            git_inv_colors: [
                "#FFFFFF", "#000000", "#000000", "#000000", "#FFFFFF", "#FFFFFF", "#FFFFFF",
                "#000000",
            ]
            .map(|v| v.to_string()),
            git_branch_label_colors: [
                "#ffffff", "black", "black", "black", "#ffffff", "#ffffff", "#ffffff", "black",
            ]
            .map(|v| v.to_string()),
            git_commit_label_color: "#333333".to_string(),
            git_commit_label_background: "#cde498".to_string(),
            git_tag_label_color: "#333333".to_string(),
            git_tag_label_background: "#cde498".to_string(),
            git_tag_label_border: "#6EAA49".to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#333333".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#333333".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#333333".to_string(),
            pie_stroke_color: "#2C5F2D".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "#6EAA49".to_string(),
            pie_opacity: 0.7,
            cscale_colors: Vec::new(),
        }
    }

    /// Official Mermaid "base" theme — a starting point for customization via
    /// `themeVariables`. Colors can be overridden by the user; derived colors
    /// are automatically computed from primary/secondary/tertiary.
    pub fn base() -> Self {
        let primary_color = "#ECECFF".to_string();
        let secondary_color = "#FFFFDE".to_string();
        let tertiary_color = "#ECECFF".to_string();
        let pie_colors = default_pie_colors(&primary_color, &secondary_color, &tertiary_color);
        Self {
            font_family: "'trebuchet ms', verdana, arial, sans-serif".to_string(),
            font_size: 16.0,
            primary_color,
            primary_text_color: "#333333".to_string(),
            primary_border_color: "#9370DB".to_string(),
            line_color: "#333333".to_string(),
            secondary_color,
            tertiary_color,
            edge_label_background: "#e8e8e8".to_string(),
            cluster_background: "#FFFFDE".to_string(),
            cluster_border: "#AAAA33".to_string(),
            background: "white".to_string(),
            sequence_actor_fill: "#ECECFF".to_string(),
            sequence_actor_border: "#9370DB".to_string(),
            sequence_actor_line: "#333333".to_string(),
            sequence_note_fill: "#fff5ad".to_string(),
            sequence_note_border: "#AAAA33".to_string(),
            sequence_activation_fill: "#f4f4f4".to_string(),
            sequence_activation_border: "#666666".to_string(),
            text_color: "#333".to_string(),
            git_colors: MERMAID_GIT_COLORS.map(|value| value.to_string()),
            git_inv_colors: MERMAID_GIT_INV_COLORS.map(|value| value.to_string()),
            git_branch_label_colors: MERMAID_GIT_BRANCH_LABEL_COLORS
                .map(|value| value.to_string()),
            git_commit_label_color: MERMAID_GIT_COMMIT_LABEL_COLOR.to_string(),
            git_commit_label_background: MERMAID_GIT_COMMIT_LABEL_BG.to_string(),
            git_tag_label_color: MERMAID_GIT_TAG_LABEL_COLOR.to_string(),
            git_tag_label_background: MERMAID_GIT_TAG_LABEL_BG.to_string(),
            git_tag_label_border: MERMAID_GIT_TAG_LABEL_BORDER.to_string(),
            pie_colors,
            pie_title_text_size: 25.0,
            pie_title_text_color: "#333".to_string(),
            pie_section_text_size: 17.0,
            pie_section_text_color: "#333".to_string(),
            pie_legend_text_size: 17.0,
            pie_legend_text_color: "#333".to_string(),
            pie_stroke_color: "black".to_string(),
            pie_stroke_width: 2.0,
            pie_outer_stroke_width: 2.0,
            pie_outer_stroke_color: "black".to_string(),
            pie_opacity: 0.7,
            cscale_colors: Vec::new(),
        }
    }
}

impl Theme {
    /// Derive secondary/tertiary colors from primaryColor, following the
    /// official Mermaid `theme-base.js` derivation rules.
    /// Call this after setting `primary_color` when secondary/tertiary haven't
    /// been explicitly overridden.
    pub fn derive_base_colors(&mut self) {
        let pc = &self.primary_color;
        // secondaryColor = adjust(primaryColor, h: -120)
        self.secondary_color = adjust_color(pc, -120.0, 0.0, 0.0);
        // tertiaryColor = adjust(primaryColor, h: 180, s: -15, l: 5)
        self.tertiary_color = adjust_color(pc, 180.0, -15.0, 5.0);
        // primaryBorderColor = darken primaryColor by 20%
        self.primary_border_color = adjust_color(pc, 0.0, 0.0, -20.0);
        // lineColor derived from primaryTextColor
        // clusterBkg = secondaryColor
        self.cluster_background = self.secondary_color.clone();
        // clusterBorder = adjust(secondaryColor, l: -10)
        self.cluster_border = adjust_color(&self.secondary_color, 0.0, 0.0, -10.0);
        // Update pie colors from derived colors
        self.pie_colors = default_pie_colors(
            &self.primary_color,
            &self.secondary_color,
            &self.tertiary_color,
        );
    }
}

fn default_pie_colors(primary: &str, secondary: &str, tertiary: &str) -> [String; 12] {
    [
        primary.to_string(),
        secondary.to_string(),
        tertiary.to_string(),
        adjust_color(primary, 0.0, 0.0, -10.0),
        adjust_color(secondary, 0.0, 0.0, -10.0),
        adjust_color(tertiary, 0.0, 0.0, -10.0),
        adjust_color(primary, 60.0, 0.0, -10.0),
        adjust_color(primary, -60.0, 0.0, -10.0),
        adjust_color(primary, 120.0, 0.0, 0.0),
        adjust_color(primary, 60.0, 0.0, -20.0),
        adjust_color(primary, -60.0, 0.0, -20.0),
        adjust_color(primary, 120.0, 0.0, -10.0),
    ]
}

pub(crate) fn adjust_color(color: &str, delta_h: f32, delta_s: f32, delta_l: f32) -> String {
    let Some((h, s, l)) = parse_color_to_hsl(color) else {
        return color.to_string();
    };
    let mut h = h + delta_h;
    if h < 0.0 {
        h = (h % 360.0) + 360.0;
    } else if h >= 360.0 {
        h %= 360.0;
    }
    let s = (s + delta_s).clamp(0.0, 100.0);
    let l = (l + delta_l).clamp(0.0, 100.0);
    format!("hsl({:.10}, {:.10}%, {:.10}%)", h, s, l)
}

pub(crate) fn parse_color_to_hsl(color: &str) -> Option<(f32, f32, f32)> {
    let color = color.trim();
    if let Some(hsl) = parse_hsl(color) {
        return Some(hsl);
    }
    let rgb = parse_hex(color)?;
    Some(rgb_to_hsl(rgb.0, rgb.1, rgb.2))
}

fn parse_hsl(value: &str) -> Option<(f32, f32, f32)> {
    let value = value.trim();
    let open = value.find('(')?;
    let close = value.rfind(')')?;
    let prefix = value[..open].trim().to_ascii_lowercase();
    if prefix != "hsl" && prefix != "hsla" {
        return None;
    }
    let inner = &value[open + 1..close];
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let h = parts[0].trim().parse::<f32>().ok()?;
    let s = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let l = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()?;
    Some((h, s, l))
}

fn parse_hex(value: &str) -> Option<(f32, f32, f32)> {
    let hex = value.strip_prefix('#')?;
    let digits = match hex.len() {
        3 => {
            let mut expanded = String::new();
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            expanded
        }
        6 => hex.to_string(),
        8 => hex[..6].to_string(),
        _ => return None,
    };
    let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
    let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
    let b = u8::from_str_radix(&digits[4..6], 16).ok()?;
    Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let mut h = 0.0;
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    if d != 0.0 {
        if max == r {
            h = ((g - b) / d) % 6.0;
        } else if max == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }
    (h, s * 100.0, l * 100.0)
}
