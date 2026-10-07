//! Image node sizing without network access.
use crate::{ir::Node, layout::TextBlock};

pub(crate) fn dimensions(node: &Node, label: &TextBlock) -> (f32, f32) {
    let natural = node.img.as_deref().and_then(intrinsic_size);
    let positive = |value: Option<f32>| value.filter(|value| value.is_finite() && *value > 0.0);
    let asset_width = positive(node.img_w);
    let asset_height = positive(node.img_h);
    let natural_width = natural
        .map(|value| value.0)
        .unwrap_or(asset_width.unwrap_or(60.0));
    let natural_height = natural
        .map(|value| value.1)
        .unwrap_or(asset_height.unwrap_or(60.0));
    let has_label = label.lines.iter().any(|line| !line.text().is_empty());
    let width = asset_width
        .unwrap_or(natural_width)
        .max(if has_label { 120.0 } else { 0.0 });
    if node.constraint.as_deref() == Some("on") {
        let ratio = natural_width / natural_height;
        let width = asset_height.map(|height| height * ratio).unwrap_or(width);
        (width, width / ratio)
    } else {
        (width, asset_height.unwrap_or(natural_height))
    }
}

#[cfg(feature = "png")]
fn intrinsic_size(url: &str) -> Option<(f32, f32)> {
    // The existing SVG backend decodes embedded PNG/JPEG/GIF/WebP/SVG data.
    // Remote images retain authored dimensions and are not fetched at layout time.
    if !url.starts_with("data:image/") {
        return None;
    }
    let escaped = url
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;");
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><image width=\"1\" height=\"1\" href=\"{escaped}\"/></svg>"
    );
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).ok()?;
    fn find(group: &usvg::Group) -> Option<(f32, f32)> {
        for node in group.children() {
            match node {
                usvg::Node::Image(image) => {
                    return Some((image.size().width(), image.size().height()));
                }
                usvg::Node::Group(group) => {
                    if let Some(size) = find(group) {
                        return Some(size);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find(tree.root())
}

#[cfg(not(feature = "png"))]
fn intrinsic_size(_url: &str) -> Option<(f32, f32)> {
    None
}
