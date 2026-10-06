use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;

use super::{DiagramData, Layout, PacketFieldLayout, PacketLayout};

pub(super) fn compute_packet_layout(graph: &Graph, config: &LayoutConfig) -> Layout {
    let cfg = &config.packet;
    let bits_per_row = cfg.bits_per_row.max(1);
    let bit_width = cfg.bit_width.max(1.0);
    let row_height = cfg.row_height.max(1.0);
    // Mermaid reserves ten extra pixels above a row for the bit numbers.
    let padding_y = cfg.padding_y.max(0.0) + if cfg.show_bits { 10.0 } else { 0.0 };
    let row_step = row_height + padding_y;
    let mut fields = Vec::new();
    let mut row_count = 0;
    for field in &graph.packet.fields {
        let mut start = field.start;
        while start <= field.end {
            let row = start / bits_per_row;
            // Match Mermaid's packet row limit, including very large ranges.
            if row >= 10_000 {
                break;
            }
            let bits_left = bits_per_row - start % bits_per_row;
            let end = field.end.min(start.saturating_add(bits_left - 1));
            fields.push(PacketFieldLayout {
                start,
                end,
                label: field.label.clone(),
                x: (start % bits_per_row) as f32 * bit_width + 1.0,
                y: row as f32 * row_step + padding_y,
                width: ((end - start + 1) as f32 * bit_width - cfg.padding_x.max(0.0)).max(0.0),
                height: row_height,
            });
            row_count = row_count.max(row + 1);
            if end == field.end {
                break;
            }
            start = end + 1;
        }
    }
    let title = graph.packet.title.clone().filter(|title| !title.is_empty());
    let width = bit_width * bits_per_row as f32 + 2.0;
    let height = row_step * (row_count + 1) as f32 - if title.is_some() { 0.0 } else { row_height };
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height: height.max(1.0),
        diagram: DiagramData::Packet(PacketLayout {
            fields,
            title,
            title_x: width / 2.0,
            title_y: height - row_step / 2.0,
            show_bits: cfg.show_bits,
        }),
        acc_title: None,
        acc_descr: None,
    }
}
