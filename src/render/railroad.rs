use crate::ir::{RailroadData, RailroadExpr};
use crate::layout::railroad::measure;
use crate::theme::Theme;

use super::escape_xml;

pub(super) fn render_railroad(data: &RailroadData, theme: &Theme) -> String {
    let mut svg = String::new();
    svg.push_str(&format!(
        "<g class=\"railroad-diagram\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\">",
        escape_xml(&theme.line_color)
    ));
    let mut y = 28.0;
    if let Some(title) = &data.title {
        svg.push_str(&format!(
            "<text x=\"28\" y=\"32\" fill=\"{}\" stroke=\"none\" font-family=\"{}\" font-size=\"20\" font-weight=\"600\">{}</text>",
            escape_xml(&theme.primary_text_color),
            escape_xml(&theme.font_family),
            escape_xml(title)
        ));
        y += 38.0;
    }
    for rule in &data.rules {
        let size = measure(&rule.expression);
        svg.push_str(&format!(
            "<text class=\"railroad-rule\" x=\"28\" y=\"{:.2}\" fill=\"{}\" stroke=\"none\" font-family=\"{}\" font-size=\"16\" font-weight=\"600\">{}</text>",
            y + 15.0,
            escape_xml(&theme.primary_text_color),
            escape_xml(&theme.font_family),
            escape_xml(&rule.name)
        ));
        let drawing_y = y + 34.0;
        let baseline = drawing_y + size.baseline;
        svg.push_str(&format!(
            "<circle class=\"railroad-start\" cx=\"27\" cy=\"{baseline:.2}\" r=\"5\" fill=\"{}\"/><path d=\"M 32 {baseline:.2} H 50\"/>",
            escape_xml(&theme.line_color)
        ));
        draw_expr(&rule.expression, 50.0, drawing_y, theme, &mut svg);
        let end_x = 50.0 + size.width;
        svg.push_str(&format!(
            "<path d=\"M {end_x:.2} {baseline:.2} H {:.2}\"/><circle class=\"railroad-end\" cx=\"{:.2}\" cy=\"{baseline:.2}\" r=\"6\"/><circle cx=\"{:.2}\" cy=\"{baseline:.2}\" r=\"2.5\" fill=\"{}\" stroke=\"none\"/>",
            end_x + 16.0,
            end_x + 22.0,
            end_x + 22.0,
            escape_xml(&theme.line_color)
        ));
        y += size.height + 70.0;
    }
    svg.push_str("</g>");
    svg
}

fn draw_expr(expression: &RailroadExpr, x: f32, y: f32, theme: &Theme, svg: &mut String) {
    let size = measure(expression);
    match expression {
        RailroadExpr::Terminal(text) | RailroadExpr::Nonterminal(text) => {
            let terminal = matches!(expression, RailroadExpr::Terminal(_));
            let class = if terminal {
                "railroad-terminal"
            } else {
                "railroad-nonterminal"
            };
            let radius = if terminal { 13 } else { 3 };
            let fill = if terminal {
                &theme.primary_color
            } else {
                &theme.secondary_color
            };
            svg.push_str(&format!(
                "<g class=\"{class}\"><rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{:.2}\" height=\"34\" rx=\"{radius}\" fill=\"{}\" stroke=\"{}\"/><text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"14\" fill=\"{}\" stroke=\"none\">{}</text></g>",
                size.width,
                escape_xml(fill),
                escape_xml(&theme.primary_border_color),
                x + size.width / 2.0,
                y + 17.0,
                escape_xml(&theme.font_family),
                escape_xml(&theme.primary_text_color),
                escape_xml(text)
            ));
        }
        RailroadExpr::Sequence(items) => {
            let mut child_x = x;
            for (index, item) in items.iter().enumerate() {
                let child = measure(item);
                draw_expr(
                    item,
                    child_x,
                    y + size.baseline - child.baseline,
                    theme,
                    svg,
                );
                child_x += child.width;
                if index + 1 < items.len() {
                    svg.push_str(&format!(
                        "<path d=\"M {child_x:.2} {:.2} H {:.2}\"/>",
                        y + size.baseline,
                        child_x + 24.0
                    ));
                    child_x += 24.0;
                }
            }
        }
        RailroadExpr::Choice(options) => {
            let mut child_y = y + 10.0;
            let main_y = y + size.baseline;
            for option in options {
                let child = measure(option);
                let branch_y = child_y + child.baseline;
                let exit_x = x + size.width - 30.0;
                let child_exit = x + 30.0 + child.width;
                svg.push_str(&format!(
                    "<path class=\"railroad-choice\" d=\"M {x:.2} {main_y:.2} H {:.2} V {branch_y:.2} H {:.2} M {child_exit:.2} {branch_y:.2} H {exit_x:.2} V {main_y:.2} H {:.2}\"/>",
                    x + 15.0, x + 30.0, x + size.width
                ));
                draw_expr(option, x + 30.0, child_y, theme, svg);
                child_y += child.height + 22.0;
            }
        }
        RailroadExpr::Optional(inner) => {
            let child = measure(inner);
            let baseline = y + size.baseline;
            let child_y = y + 38.0;
            let child_baseline = child_y + child.baseline;
            svg.push_str(&format!(
                "<path class=\"railroad-optional\" d=\"M {x:.2} {baseline:.2} H {:.2} M {:.2} {baseline:.2} V {child_baseline:.2} H {:.2} M {:.2} {child_baseline:.2} H {:.2} V {baseline:.2}\"/>",
                x + size.width, x + 15.0, x + 30.0,
                x + 30.0 + child.width, x + size.width - 15.0
            ));
            draw_expr(inner, x + 30.0, child_y, theme, svg);
        }
        RailroadExpr::ZeroOrMore(inner) | RailroadExpr::OneOrMore(inner) => {
            let zero = matches!(expression, RailroadExpr::ZeroOrMore(_));
            let child = measure(inner);
            let child_y = y + if zero { 28.0 } else { 10.0 };
            let baseline = child_y + child.baseline;
            let left = x + 30.0;
            let right = left + child.width;
            svg.push_str(&format!(
                "<path d=\"M {x:.2} {baseline:.2} H {left:.2} M {right:.2} {baseline:.2} H {:.2}\"/>",
                x + size.width
            ));
            draw_expr(inner, left, child_y, theme, svg);
            if zero {
                let bypass_y = y + 12.0;
                svg.push_str(&format!(
                    "<path class=\"railroad-bypass\" d=\"M {:.2} {baseline:.2} V {bypass_y:.2} H {:.2} V {baseline:.2}\"/>",
                    x + 15.0,
                    x + size.width - 15.0
                ));
            }
            let loop_y = y + size.height - 15.0;
            svg.push_str(&format!(
                "<path class=\"railroad-loop\" d=\"M {:.2} {baseline:.2} V {loop_y:.2} H {:.2} V {baseline:.2}\"/>",
                x + size.width - 15.0,
                x + 15.0
            ));
        }
    }
}
