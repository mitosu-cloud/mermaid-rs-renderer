use std::fmt::Write;

use crate::layout::PacketLayout;
use crate::theme::Theme;

use super::escape_xml;

pub(super) fn render_packet(packet: &PacketLayout, theme: &Theme) -> String {
    let mut svg = format!(
        "<g font-family=\"{}\" fill=\"black\">",
        escape_xml(&theme.font_family)
    );
    for field in &packet.fields {
        let x = field.x;
        let y = field.y;
        let width = field.width;
        let height = field.height;
        let center_x = x + width / 2.0;
        let center_y = y + height / 2.0;
        let _ = write!(
            svg,
            "<rect class=\"packetBlock\" x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"#efefef\" stroke=\"black\" stroke-width=\"1\"/><text class=\"packetLabel\" x=\"{center_x}\" y=\"{center_y}\" font-size=\"12\" dominant-baseline=\"middle\" text-anchor=\"middle\">{}</text>",
            escape_xml(&field.label)
        );
        if packet.show_bits {
            let single_bit = field.start == field.end;
            let start_x = if single_bit { center_x } else { x };
            let anchor = if single_bit { "middle" } else { "start" };
            let bit_y = y - 2.0;
            let _ = write!(
                svg,
                "<text class=\"packetByte start\" x=\"{start_x}\" y=\"{bit_y}\" font-size=\"10\" text-anchor=\"{anchor}\">{}</text>",
                field.start
            );
            if !single_bit {
                let end_x = x + width;
                let _ = write!(
                    svg,
                    "<text class=\"packetByte end\" x=\"{end_x}\" y=\"{bit_y}\" font-size=\"10\" text-anchor=\"end\">{}</text>",
                    field.end
                );
            }
        }
    }
    if let Some(title) = &packet.title {
        let _ = write!(
            svg,
            "<text class=\"packetTitle\" x=\"{}\" y=\"{}\" font-size=\"14\" dominant-baseline=\"middle\" text-anchor=\"middle\">{}</text>",
            packet.title_x,
            packet.title_y,
            escape_xml(title)
        );
    }
    svg.push_str("</g>");
    svg
}
