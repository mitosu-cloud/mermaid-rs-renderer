use super::*;
pub(super) fn shape(
    s: &C4ShapeLayout,
    conf: &crate::config::C4Config,
    default_fill: &str,
    default_stroke: &str,
    font: f32,
    family: &str,
) -> String {
    let fill = escape_xml(s.bg_color.as_deref().unwrap_or(default_fill));
    let stroke = escape_xml(s.border_color.as_deref().unwrap_or(default_stroke));
    let color = escape_xml(s.font_color.as_deref().unwrap_or("#FFFFFF"));
    let mut out = format!("<g id=\"c4-{}\" class=\"c4-shape\">", escape_xml(&s.id));
    if crate::c4_shapes::person(s.kind) {
        let r = (s.width * 0.23).clamp(16.0, 56.0);
        let body_h = s.height - r * 1.73;
        let radius = (s.width * 0.177).min(body_h * 0.45);
        out.push_str(&format!("<rect x=\"{:.4}\" y=\"{:.4}\" width=\"{:.4}\" height=\"{body_h:.4}\" rx=\"{radius:.4}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"2\"/><circle cx=\"{:.4}\" cy=\"{:.4}\" r=\"{r:.4}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",s.x,s.y+r*1.73,s.width,s.x+s.width/2.0,s.y+r));
    } else if crate::c4_shapes::database(s.kind) {
        let rx = s.width / 2.0;
        let ry = rx / (2.5 + s.width / 50.0);
        let h = s.height - 2.0 * ry;
        out.push_str(&format!("<path d=\"M{:.4},{:.4} a{rx:.4},{ry:.4} 0,0,0 {:.4},0 a{rx:.4},{ry:.4} 0,0,0 -{:.4},0 l0,{h:.4} a{rx:.4},{ry:.4} 0,0,0 {:.4},0 l0,-{h:.4}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",s.x,s.y+ry,s.width,s.width,s.width));
    } else if crate::c4_shapes::queue(s.kind) {
        let rx = 8.0;
        let ry = s.height / 2.0;
        out.push_str(&format!("<path d=\"M{:.4},{:.4} h{:.4} a{rx},{ry} 0,0,1 0,{:.4} h-{:.4} a{rx},{ry} 0,0,1 0,-{:.4} M{:.4},{:.4} a{rx},{ry} 0,0,0 0,{:.4}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",s.x+rx,s.y,s.width-2.0*rx,s.height,s.width-2.0*rx,s.height,s.x+s.width-rx,s.y,s.height));
    } else {
        out.push_str(&format!("<rect x=\"{:.4}\" y=\"{:.4}\" width=\"{:.4}\" height=\"{:.4}\" rx=\"12\" ry=\"12\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{}\"/>",s.x,s.y,s.width,s.height,conf.shape_stroke_width));
    }
    for (label, size, bold) in std::iter::once((&s.label, font, true))
        .chain(s.type_or_techn.iter().map(|l| (l, font * 0.75, false)))
        .chain(s.descr.iter().map(|l| (l, font * 0.82, false)))
    {
        let baseline = s.y
            + label.y
            + crate::text_metrics::centered_baseline_offset(size, family).unwrap_or(size * 0.35)
            + crate::text_metrics::svg_text_height(size, family).unwrap_or(size * 1.35) / 2.0;
        for (i, line) in label.lines.iter().enumerate() {
            out.push_str(&format!("<text x=\"{:.4}\" y=\"{:.4}\" text-anchor=\"middle\" fill=\"{color}\" font-family=\"{}\" font-size=\"{size}\" font-weight=\"{}\">{}</text>",s.x+s.width/2.0,baseline+i as f32*size*1.1,normalize_font_family(family),if bold {"bold"}else{"normal"},escape_xml(line)));
        }
    }
    out.push_str("</g>");
    out
}
