use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::{DiagramData, Layout, PacketBlockLayout, PacketLayout};

const ROW_HEIGHT: f32 = 32.0;
const BIT_WIDTH: f32 = 32.0;
const BITS_PER_ROW: u32 = 32;
const PADDING_X: f32 = 5.0;
const PADDING_Y: f32 = 15.0;
const TOTAL_ROW_HEIGHT: f32 = ROW_HEIGHT + PADDING_Y;
const SHOW_BITS: bool = true;

pub(super) fn compute_packet_layout(
    graph: &Graph,
    _theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let mut blocks = Vec::new();
    let mut max_row = 0_u32;

    for block in &graph.packet.blocks {
        let mut start = block.start;
        while start <= block.end {
            let row = start / BITS_PER_ROW;
            let row_end = ((row + 1) * BITS_PER_ROW - 1).min(block.end);
            max_row = max_row.max(row);

            let x = (start % BITS_PER_ROW) as f32 * BIT_WIDTH + 1.0;
            let y = row as f32 * TOTAL_ROW_HEIGHT + PADDING_Y;
            let width = (row_end - start + 1) as f32 * BIT_WIDTH - PADDING_X;

            blocks.push(PacketBlockLayout {
                start,
                end: row_end,
                label: block.label.clone(),
                x,
                y,
                width,
                height: ROW_HEIGHT,
            });

            if row_end == u32::MAX {
                break;
            }
            start = row_end + 1;
        }
    }

    let word_count = if graph.packet.blocks.is_empty() {
        0
    } else {
        max_row + 1
    };
    let width = BIT_WIDTH * BITS_PER_ROW as f32 + 2.0;
    let mut height = TOTAL_ROW_HEIGHT * (word_count as f32 + 1.0);
    if graph.packet.title.is_none() {
        height -= ROW_HEIGHT;
    }

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Packet(PacketLayout {
            width,
            height,
            title: graph.packet.title.clone(),
            title_x: width / 2.0,
            title_y: height - TOTAL_ROW_HEIGHT / 2.0,
            show_bits: SHOW_BITS,
            blocks,
        }),
        acc_title: None,
        acc_descr: None,
    }
}
