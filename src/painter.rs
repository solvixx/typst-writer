use gpui::*;
use typst::layout::{Frame, FrameItem};
use typst::syntax::Source;
use std::sync::Arc;

fn is_math_placeholder(linked_node: &typst::syntax::LinkedNode) -> bool {
    use typst::syntax::SyntaxKind;
    if linked_node.text() != "?" {
        return false;
    }
    let mut curr = Some(linked_node.clone());
    while let Some(node) = curr {
        if node.kind() == SyntaxKind::Math || node.kind() == SyntaxKind::MathIdent {
            return true;
        }
        curr = node.parent().cloned();
    }
    false
}

// Recursive helper to traverse and render nested Typst Frame items natively inside GPUI
pub fn paint_frame(
    frame: &Frame,
    origin: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
    source: &Source,
    selection_range: Option<(usize, usize)>,
) {
    for (pos, item) in frame.items() {
        let item_x = origin.x + px(pos.x.to_pt() as f32);
        let item_y = origin.y + px(pos.y.to_pt() as f32);

        match item {
            FrameItem::Text(text_item) => {
                let size_px = px(text_item.size.to_pt() as f32);
                
                let mut family_name = text_item.font.info().family.to_string();
                if family_name == "New Computer Modern Math" {
                    family_name = "NewComputerModernMath".to_string();
                } else if family_name == "New Computer Modern" {
                    family_name = "NewComputerModern10".to_string();
                }
                let mut weight = FontWeight(text_item.font.info().variant.weight.to_number() as f32);
                if family_name == "NewComputerModernMath" || family_name == "NewComputerModern10" {
                    weight = FontWeight::NORMAL;
                }
                let style = match text_item.font.info().variant.style {
                    typst::text::FontStyle::Normal => FontStyle::Normal,
                    typst::text::FontStyle::Italic => FontStyle::Italic,
                    typst::text::FontStyle::Oblique => FontStyle::Oblique,
                };

                // Map Typst text paint color directly to hardware accelerated color
                let text_color = match &text_item.fill {
                    typst::visualize::Paint::Solid(color) => {
                        let rgb = color.to_rgb();
                        gpui::Rgba {
                            r: rgb.red,
                            g: rgb.green,
                            b: rgb.blue,
                            a: rgb.alpha,
                        }
                    }
                    _ => gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                };

                let fallbacks = Some(FontFallbacks(Arc::new(vec![
                    "New Computer Modern Math".to_string(),
                    "New Computer Modern".to_string(),
                    "Libertinus Serif".to_string(),
                    "DejaVu Sans Mono".to_string(),
                ])));

                let font = Font {
                    family: family_name.into(),
                    weight,
                    style,
                    features: Default::default(),
                    fallbacks,
                };

                // Resolve GPUI FontId using the TextSystem
                let font_id = cx.text_system().resolve_font(&font);
                let hsla_color: gpui::Hsla = text_color.into();

                let mut current_x = item_x;
                for glyph in &text_item.glyphs {
                    let x_offset = px((glyph.x_offset.get() as f32) * (text_item.size.to_pt() as f32));
                    let y_offset = px((glyph.y_offset.get() as f32) * (text_item.size.to_pt() as f32));
                    let x_advance = px((glyph.x_advance.get() as f32) * (text_item.size.to_pt() as f32));
                    
                    let span = glyph.span.0;
                    let mut is_placeholder = false;
                    let mut is_glyph_selected = false;
                    
                    if let Some(linked_node) = source.find(span) {
                        is_placeholder = is_math_placeholder(&linked_node);
                        
                        if let Some((sel_start, sel_end)) = selection_range {
                            let offset = linked_node.offset() + (glyph.span.1 as usize);
                            let min = sel_start.min(sel_end);
                            let max = sel_start.max(sel_end);
                            if offset >= min && offset < max {
                                is_glyph_selected = true;
                            }
                        }
                    }

                    if is_placeholder {
                        if is_glyph_selected {
                            // Draw translucent vector selection box behind the placeholder, consistent with text selection
                            window.paint_quad(quad(
                                Bounds {
                                    origin: point(current_x, item_y - size_px),
                                    size: size(x_advance, size_px),
                                },
                                px(0.0),
                                gpui::rgba(0x3b82f633), // 20% opacity Blue-500 selection highlight
                                px(0.0),
                                gpui::transparent_black(),
                                Default::default(),
                            ));
                        }

                        // Design a beautiful rounded box representing the slot
                        let box_height = size_px * 0.70;
                        let box_width = size_px * 0.70;
                        let box_y = item_y - size_px * 0.75;
                        let bounds = Bounds {
                            origin: point(current_x + (x_advance - box_width) / 2.0, box_y),
                            size: size(box_width, box_height),
                        };

                        // Unselected (and selected) look: Soft gray border with a light background representing the placeholder
                        window.paint_quad(quad(
                            bounds,
                            px(2.5),
                            gpui::rgba(0x00000008), // 3% dark fill
                            px(1.0),
                            gpui::rgba(0x9ca3af80), // 50% opacity gray-400 border
                            Default::default(),
                        ));
                    } else {
                        // Vector Selection Box Overlay rendering (if selection exists)
                        if is_glyph_selected {
                            // Draw translucent vector selection box behind the glyph
                            window.paint_quad(quad(
                                Bounds {
                                    origin: point(current_x, item_y - size_px),
                                    size: size(x_advance, size_px),
                                },
                                px(0.0),
                                gpui::rgba(0x3b82f633), // 20% opacity Blue-500 selection highlight
                                px(0.0),
                                gpui::transparent_black(),
                                Default::default(),
                            ));
                        }

                        // Construct GPUI GlyphId from Typst's shaped glyph ID using transmute
                        let gpui_glyph_id: gpui::GlyphId = unsafe { std::mem::transmute(glyph.id as u32) };
                        
                        // Paint glyph directly at Typst's exact coordinate!
                        // Note: Typst's baseline is item_y. Typst's y_offset goes UP, so we subtract y_offset.
                        let glyph_origin = point(current_x + x_offset, item_y - y_offset);
                        
                        let _ = window.paint_glyph(
                            glyph_origin,
                            font_id,
                            gpui_glyph_id,
                            size_px,
                            hsla_color,
                        );
                    }

                    current_x += x_advance;
                }
            }
            FrameItem::Group(group) => {
                let dx = px(group.transform.tx.to_pt() as f32);
                let dy = px(group.transform.ty.to_pt() as f32);
                paint_frame(
                    &group.frame,
                    point(item_x + dx, item_y + dy),
                    window,
                    cx,
                    source,
                    selection_range,
                );
            }
            FrameItem::Shape(shape, _) => {
                let geometry = &shape.geometry;

                // Extract high-performance custom fills and strokes
                let fill_color = match &shape.fill {
                    Some(paint) => {
                        match paint {
                            typst::visualize::Paint::Solid(color) => {
                                let rgb = color.to_rgb();
                                gpui::Rgba {
                                    r: rgb.red,
                                    g: rgb.green,
                                    b: rgb.blue,
                                    a: rgb.alpha,
                                }
                            }
                            _ => gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
                        }
                    }
                    None => gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
                };

                let stroke_color = match &shape.stroke {
                    Some(stroke) => {
                        match &stroke.paint {
                            typst::visualize::Paint::Solid(color) => {
                                let rgb = color.to_rgb();
                                gpui::Rgba {
                                    r: rgb.red,
                                    g: rgb.green,
                                    b: rgb.blue,
                                    a: rgb.alpha,
                                }
                            }
                            _ => gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                        }
                    }
                    None => gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
                };

                let stroke_width = match &shape.stroke {
                    Some(stroke) => px(stroke.thickness.to_pt() as f32),
                    None => px(0.0),
                };

                match geometry {
                    typst::visualize::Geometry::Rect(rect_size) => {
                        let rect_w = px(rect_size.x.to_pt() as f32);
                        let rect_h = px(rect_size.y.to_pt() as f32);
                        let rect_bounds = Bounds {
                            origin: point(item_x, item_y),
                            size: size(rect_w, rect_h),
                        };
                        window.paint_quad(quad(
                            rect_bounds,
                            px(0.0),
                            fill_color,
                            stroke_width,
                            stroke_color,
                            Default::default(),
                        ));
                    }
                    typst::visualize::Geometry::Line(p) => {
                        let end_x = item_x + px(p.x.to_pt() as f32);
                        let end_y = item_y + px(p.y.to_pt() as f32);
                        let mut builder = PathBuilder::stroke(stroke_width);
                        builder.move_to(point(item_x, item_y));
                        builder.line_to(point(end_x, end_y));
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, stroke_color);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
