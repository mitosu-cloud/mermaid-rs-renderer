use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::ir::Direction;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    pub text: String,
    pub style: SpanStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLine {
    pub spans: Vec<TextSpan>,
}

impl TextLine {
    /// Create a plain (unformatted) line from a string.
    pub fn plain(s: String) -> Self {
        Self {
            spans: vec![TextSpan {
                text: s,
                style: SpanStyle::default(),
            }],
        }
    }

    /// Get the plain text content of this line, zero-alloc for plain lines.
    pub fn text(&self) -> Cow<'_, str> {
        if self.spans.len() == 1 {
            Cow::Borrowed(&self.spans[0].text)
        } else {
            let mut s = String::new();
            for span in &self.spans {
                s.push_str(&span.text);
            }
            Cow::Owned(s)
        }
    }

    /// Check if any span has bold or italic formatting.
    pub fn has_formatting(&self) -> bool {
        self.spans.iter().any(|s| s.style.bold || s.style.italic)
    }
}

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub lines: Vec<TextLine>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub shape: crate::ir::NodeShape,
    pub style: crate::ir::NodeStyle,
    pub link: Option<crate::ir::NodeLink>,
    pub anchor_subgraph: Option<usize>,
    pub hidden: bool,
    pub icon: Option<String>,
    pub img: Option<String>,
    pub img_w: Option<f32>,
    pub img_h: Option<f32>,
    /// Secondary label for treemap leaf nodes (displays the value below the name).
    pub sub_label: Option<TextBlock>,
    /// Whether this treemap node is a leaf (no children) — affects label centering.
    pub is_treemap_leaf: bool,
}

#[derive(Debug, Clone)]
pub struct EdgeLayout {
    pub from: String,
    pub to: String,
    pub label: Option<TextBlock>,
    pub start_label: Option<TextBlock>,
    pub end_label: Option<TextBlock>,
    pub label_anchor: Option<(f32, f32)>,
    pub start_label_anchor: Option<(f32, f32)>,
    pub end_label_anchor: Option<(f32, f32)>,
    pub points: Vec<(f32, f32)>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<crate::ir::EdgeArrowhead>,
    pub arrow_end_kind: Option<crate::ir::EdgeArrowhead>,
    pub start_decoration: Option<crate::ir::EdgeDecoration>,
    pub end_decoration: Option<crate::ir::EdgeDecoration>,
    pub sequence_arrow_end: Option<crate::ir::SequenceArrowHead>,
    pub sequence_arrow_start: Option<crate::ir::SequenceArrowHead>,
    pub style: crate::ir::EdgeStyle,
    pub override_style: crate::ir::EdgeStyleOverride,
    pub curve: Option<crate::ir::CurveType>,
}

#[derive(Debug, Clone)]
pub struct SubgraphLayout {
    pub label: String,
    pub label_block: TextBlock,
    pub nodes: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: crate::ir::NodeStyle,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Lifeline {
    pub id: String,
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
}

#[derive(Debug, Clone)]
pub struct SequenceLabel {
    pub x: f32,
    pub y: f32,
    pub text: TextBlock,
}

#[derive(Debug, Clone)]
pub struct SequenceFrameLayout {
    pub kind: crate::ir::SequenceFrameKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label_box: (f32, f32, f32, f32),
    pub label: SequenceLabel,
    pub section_labels: Vec<SequenceLabel>,
    pub dividers: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct SequenceBoxLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: Option<TextBlock>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SequenceNoteLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub position: crate::ir::SequenceNotePosition,
    pub participants: Vec<String>,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct StateNoteLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: TextBlock,
    pub position: crate::ir::StateNotePosition,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct SequenceActivationLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub participant: String,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct SequenceNumberLayout {
    pub x: f32,
    pub y: f32,
    pub value: usize,
}

#[derive(Debug, Clone)]
pub struct PieSliceLayout {
    pub label: TextBlock,
    pub value: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct PieLegendItem {
    pub x: f32,
    pub y: f32,
    pub label: TextBlock,
    pub color: String,
    pub marker_size: f32,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct PieTitleLayout {
    pub x: f32,
    pub y: f32,
    pub text: TextBlock,
}

#[derive(Debug, Clone)]
pub struct SankeyNodeLayout {
    pub id: String,
    pub label: String,
    pub total: f32,
    pub rank: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct SankeyLinkLayout {
    pub source: String,
    pub target: String,
    pub value: f32,
    pub thickness: f32,
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub color_start: String,
    pub color_end: String,
    pub gradient_id: String,
}

#[derive(Debug, Clone)]
pub struct SankeyLayout {
    pub width: f32,
    pub height: f32,
    pub node_width: f32,
    pub nodes: Vec<SankeyNodeLayout>,
    pub links: Vec<SankeyLinkLayout>,
}

#[derive(Debug, Clone)]
pub struct GitGraphBranchLabelLayout {
    pub bg_x: f32,
    pub bg_y: f32,
    pub bg_width: f32,
    pub bg_height: f32,
    pub text_x: f32,
    pub text_y: f32,
    pub text_width: f32,
    pub text_height: f32,
}

#[derive(Debug, Clone)]
pub struct GitGraphBranchLayout {
    pub name: String,
    pub index: usize,
    pub pos: f32,
    pub label: GitGraphBranchLabelLayout,
}

#[derive(Debug, Clone)]
pub struct GitGraphTransform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub rotate_deg: f32,
    pub rotate_cx: f32,
    pub rotate_cy: f32,
}

#[derive(Debug, Clone)]
pub struct GitGraphCommitLabelLayout {
    pub text: String,
    pub text_x: f32,
    pub text_y: f32,
    pub bg_x: f32,
    pub bg_y: f32,
    pub bg_width: f32,
    pub bg_height: f32,
    pub transform: Option<GitGraphTransform>,
}

#[derive(Debug, Clone)]
pub struct GitGraphTagLayout {
    pub text: String,
    pub text_x: f32,
    pub text_y: f32,
    pub points: Vec<(f32, f32)>,
    pub hole_x: f32,
    pub hole_y: f32,
    pub transform: Option<GitGraphTransform>,
}

#[derive(Debug, Clone)]
pub struct GitGraphCommitLayout {
    pub id: String,
    pub seq: usize,
    pub branch_index: usize,
    pub x: f32,
    pub y: f32,
    pub axis_pos: f32,
    pub commit_type: crate::ir::GitGraphCommitType,
    pub custom_type: Option<crate::ir::GitGraphCommitType>,
    pub tags: Vec<GitGraphTagLayout>,
    pub label: Option<GitGraphCommitLabelLayout>,
}

#[derive(Debug, Clone)]
pub struct GitGraphArrowLayout {
    pub path: String,
    pub color_index: usize,
}

#[derive(Debug, Clone)]
pub struct GitGraphLayout {
    pub branches: Vec<GitGraphBranchLayout>,
    pub commits: Vec<GitGraphCommitLayout>,
    pub arrows: Vec<GitGraphArrowLayout>,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub max_pos: f32,
    pub direction: Direction,
}

#[derive(Debug, Clone)]
pub struct ErrorLayout {
    pub viewbox_width: f32,
    pub viewbox_height: f32,
    pub render_width: f32,
    pub render_height: f32,
    pub message: String,
    pub version: String,
    pub text_x: f32,
    pub text_y: f32,
    pub text_size: f32,
    pub version_x: f32,
    pub version_y: f32,
    pub version_size: f32,
    pub icon_scale: f32,
    pub icon_tx: f32,
    pub icon_ty: f32,
}

#[derive(Debug, Clone)]
pub struct XYChartBarLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub value: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct XYChartLineLayout {
    pub points: Vec<(f32, f32)>,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct XYChartLayout {
    pub title: Option<TextBlock>,
    pub title_y: f32,
    pub x_axis_label: Option<TextBlock>,
    pub x_axis_label_y: f32,
    pub y_axis_label: Option<TextBlock>,
    pub y_axis_label_x: f32,
    pub x_axis_categories: Vec<(String, f32)>,
    pub y_axis_ticks: Vec<(String, f32)>,
    pub bars: Vec<XYChartBarLayout>,
    pub lines: Vec<XYChartLineLayout>,
    pub plot_x: f32,
    pub plot_y: f32,
    pub plot_width: f32,
    pub plot_height: f32,
    pub width: f32,
    pub height: f32,
}

/// A time period card displayed above the timeline axis ("taskWrapper" in JS).
#[derive(Debug, Clone)]
pub struct TimelineTimePeriodLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub section_idx: i32,
}

/// An individual event card displayed below the timeline axis ("eventWrapper" in JS).
#[derive(Debug, Clone)]
pub struct TimelineEventCardLayout {
    pub lines: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub section_idx: i32,
}

/// A section header spanning multiple time periods.
#[derive(Debug, Clone)]
pub struct TimelineSectionLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub section_idx: i32,
}

/// Dashed connector from a time card down to the event area.
#[derive(Debug, Clone)]
pub struct TimelineConnectorLayout {
    pub x: f32,
    pub start_y: f32,
    pub end_y: f32,
}

#[derive(Debug, Clone)]
pub struct TimelineLayout {
    pub title: Option<TextBlock>,
    pub title_x: f32,
    pub title_y: f32,
    pub sections: Vec<TimelineSectionLayout>,
    pub time_periods: Vec<TimelineTimePeriodLayout>,
    pub event_cards: Vec<TimelineEventCardLayout>,
    pub connectors: Vec<TimelineConnectorLayout>,
    pub axis_y: f32,
    pub axis_start_x: f32,
    pub axis_end_x: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct JourneyActorLayout {
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct JourneyTaskLayout {
    pub id: String,
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub score: Option<f32>,
    pub score_color: String,
    pub score_y: f32,
    pub actors: Vec<String>,
    pub actor_y: Option<f32>,
    pub section_idx: usize,
}

#[derive(Debug, Clone)]
pub struct JourneySectionLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct JourneyLayout {
    pub title: Option<TextBlock>,
    pub title_y: f32,
    pub actors: Vec<JourneyActorLayout>,
    pub actor_label_y: f32,
    pub tasks: Vec<JourneyTaskLayout>,
    pub sections: Vec<JourneySectionLayout>,
    pub baseline: Option<(f32, f32, f32)>,
    pub score_radius: f32,
    pub actor_radius: f32,
    pub actor_gap: f32,
    pub card_gap_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct SequenceData {
    pub lifelines: Vec<Lifeline>,
    pub footboxes: Vec<NodeLayout>,
    pub boxes: Vec<SequenceBoxLayout>,
    pub frames: Vec<SequenceFrameLayout>,
    pub notes: Vec<SequenceNoteLayout>,
    pub activations: Vec<SequenceActivationLayout>,
    pub numbers: Vec<SequenceNumberLayout>,
}

#[derive(Debug, Clone)]
pub struct PieData {
    pub slices: Vec<PieSliceLayout>,
    pub legend: Vec<PieLegendItem>,
    pub center: (f32, f32),
    pub radius: f32,
    pub title: Option<PieTitleLayout>,
}

#[derive(Debug, Clone)]
pub struct VennCircleLayout {
    pub id: String,
    pub label: String,
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub color: String,
    pub fill_opacity: f32,
    pub stroke: String,
    pub stroke_width: f32,
    pub text_color: String,
}

#[derive(Debug, Clone)]
pub struct VennIntersectionLayout {
    pub set_ids: Vec<String>,
    pub label: Option<String>,
    pub cx: f32,
    pub cy: f32,
    pub text_color: String,
}

#[derive(Debug, Clone)]
pub struct VennLayout {
    pub width: f32,
    pub height: f32,
    pub title: Option<String>,
    pub circles: Vec<VennCircleLayout>,
    pub intersections: Vec<VennIntersectionLayout>,
}

#[derive(Debug, Clone)]
pub enum DiagramData {
    Graph { state_notes: Vec<StateNoteLayout> },
    Sequence(SequenceData),
    Pie(PieData),
    Quadrant(QuadrantLayout),
    Gantt(GanttLayout),
    Sankey(SankeyLayout),
    GitGraph(GitGraphLayout),
    C4(C4Layout),
    XYChart(XYChartLayout),
    Timeline(TimelineLayout),
    Journey(JourneyLayout),
    Venn(VennLayout),
    TreeView(TreeViewLayout),
    Ishikawa(IshikawaLayout),
    Wardley(WardleyLayout),
    Error(ErrorLayout),
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub kind: crate::ir::DiagramKind,
    pub nodes: BTreeMap<String, NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub subgraphs: Vec<SubgraphLayout>,
    pub width: f32,
    pub height: f32,
    pub diagram: DiagramData,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct C4Layout {
    pub shapes: Vec<C4ShapeLayout>,
    pub boundaries: Vec<C4BoundaryLayout>,
    pub rels: Vec<C4RelLayout>,
    pub viewbox_x: f32,
    pub viewbox_y: f32,
    pub viewbox_width: f32,
    pub viewbox_height: f32,
    pub use_max_width: bool,
}

#[derive(Debug, Clone)]
pub struct C4TextLayout {
    pub text: String,
    pub lines: Vec<String>,
    pub width: f32,
    pub height: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct C4ShapeLayout {
    pub id: String,
    pub kind: crate::ir::C4ShapeKind,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub type_label: C4TextLayout,
    pub label: C4TextLayout,
    pub type_or_techn: Option<C4TextLayout>,
    pub descr: Option<C4TextLayout>,
    pub image_y: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct C4BoundaryLayout {
    pub id: String,
    pub label: C4TextLayout,
    pub boundary_type: Option<C4TextLayout>,
    pub descr: Option<C4TextLayout>,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct C4RelLayout {
    pub kind: crate::ir::C4RelKind,
    pub from: String,
    pub to: String,
    pub label: C4TextLayout,
    pub techn: Option<C4TextLayout>,
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub offset_x: f32,
    pub offset_y: f32,
    pub line_color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QuadrantLayout {
    pub title: Option<TextBlock>,
    pub title_y: f32,
    pub x_axis_left: Option<TextBlock>,
    pub x_axis_right: Option<TextBlock>,
    pub y_axis_bottom: Option<TextBlock>,
    pub y_axis_top: Option<TextBlock>,
    pub quadrant_labels: [Option<TextBlock>; 4],
    pub points: Vec<QuadrantPointLayout>,
    pub grid_x: f32,
    pub grid_y: f32,
    pub grid_width: f32,
    pub grid_height: f32,
}

#[derive(Debug, Clone)]
pub struct QuadrantPointLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct GanttLayout {
    pub title: Option<TextBlock>,
    pub sections: Vec<GanttSectionLayout>,
    pub tasks: Vec<GanttTaskLayout>,
    pub time_start: f32,
    pub time_end: f32,
    pub chart_x: f32,
    pub chart_y: f32,
    pub chart_width: f32,
    pub chart_height: f32,
    pub row_height: f32,
    pub label_x: f32,
    pub label_width: f32,
    pub section_label_x: f32,
    pub section_label_width: f32,
    pub task_label_x: f32,
    pub task_label_width: f32,
    pub title_y: f32,
    pub ticks: Vec<GanttTick>,
    /// X position for the "today" marker line, if applicable.
    pub today_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct GanttSectionLayout {
    pub label: TextBlock,
    pub y: f32,
    pub height: f32,
    pub color: String,
    pub band_color: String,
}

#[derive(Debug, Clone)]
pub struct GanttTaskLayout {
    pub label: TextBlock,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
    pub start: f32,
    pub duration: f32,
    pub status: Option<crate::ir::GanttStatus>,
}

#[derive(Debug, Clone)]
pub struct GanttTick {
    pub x: f32,
    pub label: String,
}

// ── TreeView layout ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TreeViewNodeLayout {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct TreeViewLineLayout {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone)]
pub struct TreeViewLayout {
    pub title: Option<String>,
    pub nodes: Vec<TreeViewNodeLayout>,
    pub lines: Vec<TreeViewLineLayout>,
    pub width: f32,
    pub height: f32,
}

// ── Ishikawa layout ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IshikawaLineLayout {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub stroke_width: f32,
}

#[derive(Debug, Clone)]
pub struct IshikawaLabelLayout {
    pub text: String,
    pub lines: Vec<String>, // pre-wrapped lines (only used for head label)
    pub x: f32,
    pub y: f32,
    pub anchor: String,    // "middle", "end", "start"
    pub font_weight: String,
    pub has_box: bool,
    pub box_x: f32,
    pub box_y: f32,
    pub box_w: f32,
    pub box_h: f32,
}

#[derive(Debug, Clone)]
pub struct IshikawaLayout {
    pub head_path: String,  // SVG path for fish head
    pub head_x: f32,
    pub head_y: f32,
    pub spine: IshikawaLineLayout,
    pub branches: Vec<IshikawaLineLayout>,
    pub labels: Vec<IshikawaLabelLayout>,
    pub width: f32,
    pub height: f32,
}

// ── Wardley layout ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WardleyNodeLayout {
    pub id: String,
    pub label: String,
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub is_anchor: bool,
    pub label_x: f32,
    pub label_y: f32,
    pub strategy: Option<crate::ir::WardleyStrategy>,
    pub inertia: bool,
}

#[derive(Debug, Clone)]
pub struct WardleyLinkLayout {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub dashed: bool,
    pub label: Option<String>,
    pub flow: Option<crate::ir::WardleyFlow>,
}

#[derive(Debug, Clone)]
pub struct WardleyTrendLayout {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone)]
pub struct WardleyStageLayout {
    pub label: String,
    pub divider_x: f32,
    pub label_x: f32,
}

#[derive(Debug, Clone)]
pub struct WardleyLayout {
    pub title: Option<String>,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub padding: f32,
    pub chart_x: f32,
    pub chart_y: f32,
    pub chart_width: f32,
    pub chart_height: f32,
    pub nodes: Vec<WardleyNodeLayout>,
    pub links: Vec<WardleyLinkLayout>,
    pub trends: Vec<WardleyTrendLayout>,
    pub stages: Vec<WardleyStageLayout>,
    pub x_label: String,
    pub y_label: String,
    pub notes: Vec<(String, f32, f32)>,
    pub width: f32,
    pub height: f32,
}
