use gpui::{px, point, Point, Pixels, size, Bounds};
use typst::layout::{Frame, FrameItem};
use typst::syntax::Source;

/// Global scaling factor from Typst points (1/72 inch) to logical pixels (typically 1/96 inch).
/// 1 pt = 1.3333334 px
pub const PT_TO_PX: f32 = 96.0 / 72.0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlyphBox {
    pub offset: usize,
    pub bounds: Bounds<Pixels>,
    pub height: f32,
    pub baseline: Pixels,
    pub is_text: bool,
    #[serde(default)]
    pub is_radical: bool,
}

pub fn collect_glyph_boxes_with_source(
    frame: &Frame,
    origin: Point<Pixels>,
    source: &Source,
    boxes: &mut Vec<GlyphBox>,
    zoom: f32,
) {
    collect_glyph_boxes_with_source_rec(frame, origin, origin.y, source, boxes, zoom);
}

fn collect_glyph_boxes_with_source_rec(
    frame: &Frame,
    origin: Point<Pixels>,
    parent_baseline: Pixels,
    source: &Source,
    boxes: &mut Vec<GlyphBox>,
    zoom: f32,
) {
    let mut last_span = None;
    let mut last_node = None;

    // Discover the local baseline and text height by finding the max of text items in this frame
    let mut local_baseline = parent_baseline;
    let mut local_text_height = px(14.666667);
    for (pos, item) in frame.items() {
        if let FrameItem::Text(text_item) = item {
            let item_y = origin.y + px(pos.y.to_pt() as f32 * PT_TO_PX) * zoom;
            local_baseline = local_baseline.max(item_y);
            let size_px = (text_item.size.to_pt() as f32 * PT_TO_PX) * zoom;
            local_text_height = px(size_px / zoom);
        }
    }

    for (pos, item) in frame.items() {
        let item_x = origin.x + px(pos.x.to_pt() as f32 * PT_TO_PX) * zoom;
        let item_y = origin.y + px(pos.y.to_pt() as f32 * PT_TO_PX) * zoom;

        match item {
            FrameItem::Text(text_item) => {
                let size_px = (text_item.size.to_pt() as f32 * PT_TO_PX) * zoom;
                let mut current_x = item_x;
                // Total height = 1.5 * size_px. This safely covers tall math symbols and radicals.
                let box_top = item_y - px(size_px);

                for glyph in &text_item.glyphs {
                    let glyph_width = px((glyph.x_advance.get() as f32) * size_px);
                    let x_offset = px((glyph.x_offset.get() as f32) * size_px);
                    let y_offset = px((glyph.y_offset.get() as f32) * size_px);
                    
                    let span = glyph.span.0;
                    
                    if Some(span) != last_span {
                        last_span = Some(span);
                        last_node = source.find(span);
                    }

                    if let Some(ref linked_node) = last_node {
                        let offset_start = linked_node.offset() + (glyph.span.1 as usize);
                        let node_range = linked_node.range();
                        let source_text = source.text();
                        
                        // Safety: ensure range is within text bounds
                        if node_range.end <= source_text.len() {
                            let _node_text = &source_text[node_range];
                        }
                        
                        // Handle math root/radical specially by using the internal placement of the bounding box
                        let node_range = linked_node.range();
                        let source_text = source.text();
                        if node_range.end > source_text.len() {
                             continue;
                        }
                        let node_text = &source_text[node_range];
                        let has_integral_name = node_text.contains("integral")
                            || node_text.contains("int")
                            || node_text.contains('∫')
                            || node_text.contains('∬')
                            || node_text.contains('∭')
                            || node_text.contains('∮');
                        
                        let has_radical_name = node_text.contains("sqrt")
                            || node_text.contains("root")
                            || node_text.contains('√')
                            || node_text.contains('∛')
                            || node_text.contains('∜');

                        // To be the actual radical symbol itself, the offset must fall within the prefix name length of the node
                        let is_radical_node = if has_radical_name {
                            let prefix_len = if node_text.starts_with("sqrt") || node_text.starts_with("root") {
                                4
                            } else if node_text.starts_with('√') || node_text.starts_with('∛') || node_text.starts_with('∜') {
                                3
                            } else {
                                1
                            };
                            offset_start < linked_node.offset() + prefix_len
                        } else {
                            false
                        };

                        let is_integral = if has_integral_name {
                            let prefix_len = if node_text.starts_with("integral") {
                                8
                            } else if node_text.starts_with("int") || node_text.starts_with('∫') || node_text.starts_with('∬') || node_text.starts_with('∭') || node_text.starts_with('∮') {
                                3
                            } else {
                                1
                            };
                            offset_start < linked_node.offset() + prefix_len
                        } else {
                            false
                        };

                        // C1: Only √/root family uses TTF bbox sizing — NOT integrals.
                        // Integral descender arms distort the bounding box below the main baseline.
                        let is_radical = is_radical_node; // integral handled separately

                        // C2: Compute glyph box geometry.
                        // - Radicals (√): use TTF bbox for accurate stroke extent.
                        // - Integrals (∫): use main-line metrics; the descender arm is decorative.
                        // - Everything else: default box_top / box_height.
                        let mut adjusted_box_top = box_top - y_offset;
                        let mut box_height = px(size_px * 1.5);
                        // Baseline for caret drawing — integrals stay on the main text line.
                        let glyph_baseline = if is_radical_node {
                            local_baseline
                        } else {
                            item_y  // item_y IS the baseline for all non-radical text
                        };

                        if is_radical_node {
                            // √ family: derive true visual extent from TTF bounding box.
                            let face = text_item.font.ttf();
                            let units_per_em = face.units_per_em();
                            let scale = size_px / f32::from(units_per_em);
                            if let Some(rect) = face.glyph_bounding_box(ttf_parser::GlyphId(glyph.id)) {
                                let y_max = f32::from(rect.y_max) * scale;
                                let y_min = f32::from(rect.y_min) * scale;
                                adjusted_box_top = item_y - y_offset - px(y_max);
                                box_height = px(y_max - y_min).max(px(size_px * 0.5));
                            }
                        }
                        // For integrals (is_integral == true): adjusted_box_top and box_height
                        // remain at their default values (main-line metrics), which is correct.

                        let bounds = Bounds {
                            origin: point(current_x + x_offset, adjusted_box_top),
                            size: size(glyph_width, box_height),
                        };

                        boxes.push(GlyphBox {
                            offset: offset_start,
                            bounds,
                            height: size_px / zoom,
                            baseline: glyph_baseline,
                            is_text: true,
                            is_radical,
                        });

                        // Edge candidate: Ensure the offset after the glyph is also reachable.
                        // Use the combined is_radical_node || is_integral flag only for byte-length
                        // calculation (both are multi-byte keyword tokens).
                        let is_keyword_token = is_radical_node || is_integral;
                        let mut glyph_byte_len = if is_keyword_token {
                            let prefix_len = node_text.chars()
                                .take_while(|c| c.is_alphabetic())
                                .map(|c| c.len_utf8())
                                .sum::<usize>();
                            if prefix_len > 0 { prefix_len } else { 1 }
                        } else {
                            if let Some(ch) = source.text().get(offset_start..).and_then(|s| s.chars().next()) {
                                ch.len_utf8()
                            } else {
                                1
                            }
                        };

                        // For √/root: if next char is '(', skip it so caret lands inside args.
                        // Do NOT do this for integrals — their sub/superscript groups are separate
                        // Typst layout items and do not start with a literal '('.
                        if is_radical_node && source.text().get(offset_start + glyph_byte_len..).and_then(|s| s.chars().next()).is_some_and(|ch| ch == '(') {
                            glyph_byte_len += 1;
                        }

                        // C3: Exit marker — always anchored to the main baseline, never the TTF
                        // distorted baseline.  This is critical for integrals so the caret placed
                        // "after the ∫ token" sits on the main text line, not at the subscript level.
                        if glyph_byte_len > 0 {
                            boxes.push(GlyphBox {
                                offset: offset_start + glyph_byte_len,
                                bounds: Bounds {
                                    origin: point(current_x + glyph_width, adjusted_box_top),
                                    size: size(px(0.0), box_height),
                                },
                                height: size_px / zoom,
                                baseline: glyph_baseline,
                                is_text: true,
                                is_radical,
                            });
                        }
                    }
                    current_x += glyph_width;
                }
            }
            FrameItem::Group(group) => {
                let dx = px(group.transform.tx.to_pt() as f32 * PT_TO_PX) * zoom;
                let dy = px(group.transform.ty.to_pt() as f32 * PT_TO_PX) * zoom;
                collect_glyph_boxes_with_source_rec(
                    &group.frame,
                    point(item_x + dx, item_y + dy),
                    item_y,
                    source,
                    boxes,
                    zoom,
                );
            }
            FrameItem::Shape(shape, span) if !span.is_detached() => {
                if let Some(node) = source.find(*span) {
                    let mut shape_bounds = Bounds {
                        origin: point(item_x, item_y),
                        size: size(px(0.0), px(0.0)),
                    };
                    match &shape.geometry {
                        typst::visualize::Geometry::Rect(s) => {
                            shape_bounds.size = size(
                                px(s.x.to_pt() as f32 * PT_TO_PX) * zoom,
                                px(s.y.to_pt() as f32 * PT_TO_PX) * zoom,
                            );
                        }
                        typst::visualize::Geometry::Line(p) => {
                            let dx = px(p.x.to_pt() as f32 * PT_TO_PX) * zoom;
                            let dy = px(p.y.to_pt() as f32 * PT_TO_PX) * zoom;
                            shape_bounds.size = size(dx.abs(), dy.abs());
                            if dx < px(0.0) { shape_bounds.origin.x += dx; }
                            if dy < px(0.0) { shape_bounds.origin.y += dy; }
                        }
                        typst::visualize::Geometry::Curve(curve) => {
                            let mut min_x = f32::MAX;
                            let mut min_y = f32::MAX;
                            let mut max_x = f32::MIN;
                            let mut max_y = f32::MIN;
                            
                            for item in curve.0.iter() {
                                match item {
                                    typst::visualize::CurveItem::Move(p) | typst::visualize::CurveItem::Line(p) => {
                                        let px_x = p.x.to_pt() as f32 * PT_TO_PX;
                                        let px_y = p.y.to_pt() as f32 * PT_TO_PX;
                                        min_x = min_x.min(px_x);
                                        min_y = min_y.min(px_y);
                                        max_x = max_x.max(px_x);
                                        max_y = max_y.max(px_y);
                                    }
                                    typst::visualize::CurveItem::Cubic(ctrl1, ctrl2, to) => {
                                        for pt in &[ctrl1, ctrl2, to] {
                                            let px_x = pt.x.to_pt() as f32 * PT_TO_PX;
                                            let px_y = pt.y.to_pt() as f32 * PT_TO_PX;
                                            min_x = min_x.min(px_x);
                                            min_y = min_y.min(px_y);
                                            max_x = max_x.max(px_x);
                                            max_y = max_y.max(px_y);
                                        }
                                    }
                                    typst::visualize::CurveItem::Close => {}
                                }
                            }
                            
                            if min_x <= max_x && min_y <= max_y {
                                let x = item_x + px(min_x) * zoom;
                                let y = item_y + px(min_y) * zoom;
                                let w = px(max_x - min_x) * zoom;
                                let h = px(max_y - min_y) * zoom;
                                shape_bounds.origin = point(x, y);
                                shape_bounds.size = size(w, h);
                            }
                        }
                    }
                    
                    // Ensure a minimum height for hit-testing
                    if shape_bounds.size.height < px(10.0) {
                        shape_bounds.origin.y -= px(5.0);
                        shape_bounds.size.height = px(10.0);
                    }
                    
                    boxes.push(GlyphBox {
                        offset: node.offset(),
                        bounds: shape_bounds,                        height: local_text_height.into(),
                        baseline: local_baseline,
                        is_text: false,
                        is_radical: false,
                    });
                    boxes.push(GlyphBox {
                        offset: node.offset() + node.len(),
                        bounds: Bounds {
                            origin: point(shape_bounds.origin.x + shape_bounds.size.width, shape_bounds.origin.y),
                            size: size(px(0.0), shape_bounds.size.height),
                        },
                        height: local_text_height.into(),
                        baseline: local_baseline,
                        is_text: false,
                        is_radical: false,
                    });
                }
            }
            FrameItem::Image(_image, s, span) if !span.is_detached() => {
                if let Some(node) = source.find(*span) {
                    let w = px(s.x.to_pt() as f32 * PT_TO_PX) * zoom;
                    let h = px(s.y.to_pt() as f32 * PT_TO_PX) * zoom;
                    let mut bounds = Bounds {
                        origin: point(item_x, item_y),
                        size: size(w, h),
                    };
                    
                    if bounds.size.height < px(10.0) {
                        bounds.origin.y -= px(5.0);
                        bounds.size.height = px(10.0);
                    }

                    boxes.push(GlyphBox {
                        offset: node.offset(),
                        bounds,                        height: local_text_height.into(),
                        baseline: local_baseline,
                        is_text: false,
                        is_radical: false,
                    });
                    boxes.push(GlyphBox {
                        offset: node.offset() + node.len(),
                        bounds: Bounds {
                            origin: point(item_x + w, item_y),
                            size: size(px(0.0), bounds.size.height),
                        },
                        height: local_text_height.into(),
                        baseline: local_baseline,
                        is_text: false,
                        is_radical: false,
                    });
                }
            }
            _ => {}
        }
    }
}

pub fn find_closest_offset(
    boxes: &[GlyphBox],
    click_pos: Point<Pixels>,
    _source: &Source,
) -> Option<(usize, Point<Pixels>, f32)> {
    boxes.iter()
        .min_by(|a, b| {
            let dx_a = f32::from(a.bounds.left() - click_pos.x).abs();
            let dy_a = if click_pos.y < a.bounds.top() {
                f32::from(a.bounds.top() - click_pos.y)
            } else if click_pos.y > a.bounds.bottom() {
                f32::from(click_pos.y - a.bounds.bottom())
            } else {
                0.0
            };
            let dist_a = dx_a * dx_a + dy_a * dy_a * 1000.0;

            let dx_b = f32::from(b.bounds.left() - click_pos.x).abs();
            let dy_b = if click_pos.y < b.bounds.top() {
                f32::from(b.bounds.top() - click_pos.y)
            } else if click_pos.y > b.bounds.bottom() {
                f32::from(click_pos.y - b.bounds.bottom())
            } else {
                0.0
            };
            let dist_b = dx_b * dx_b + dy_b * dy_b * 1000.0;

            // Tie-break: Prefer text items if distances are equal
            let res = dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal);
            if res == std::cmp::Ordering::Equal {
                match (a.is_text, b.is_text) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            } else {
                res
            }
        })
        .map(|b| {
            (b.offset, point(b.bounds.origin.x, b.baseline), b.height)
        })
}

pub fn find_cursor_position(
    frame: &Frame,
    _origin: Point<Pixels>,
    target_offset: usize,
    source: &Source,
    zoom: f32,
) -> Option<(Point<Pixels>, f32, usize)> {
    let mut boxes = Vec::new();
    collect_glyph_boxes_with_source(frame, Point::default(), source, &mut boxes, zoom);
    find_cursor_position_in_boxes(&boxes, target_offset, source)
}

pub fn find_cursor_position_in_boxes(
    boxes: &[GlyphBox],
    target_offset: usize,
    source: &Source,
) -> Option<(Point<Pixels>, f32, usize)> {
    if boxes.is_empty() {
        return None;
    }

    let lines = source.lines();
    let line_target = lines.byte_to_line(target_offset).unwrap_or(0);

    // Find box A (largest offset <= target_offset)
    let mut opt_a: Option<&GlyphBox> = None;
    for b in boxes {
        if b.offset <= target_offset {
            if opt_a.is_none() {
                opt_a = Some(b);
            } else {
                let current_a = opt_a.unwrap();
                let choose_new = if b.offset > current_a.offset {
                    true
                } else if b.offset == current_a.offset {
                    let line_b = lines.byte_to_line(b.offset).unwrap_or(0);
                    let line_cur = lines.byte_to_line(current_a.offset).unwrap_or(0);
                    
                    let b_on_line = line_b == line_target;
                    let cur_on_line = line_cur == line_target;
                    
                    if b_on_line && !cur_on_line {
                        true
                    } else if !b_on_line && cur_on_line {
                        false
                    } else {
                        // Prefer visually rightmost to anchor right edge of characters/exit markers at this offset.
                        b.bounds.origin.x > current_a.bounds.origin.x
                    }
                } else {
                    false
                };
                if choose_new {
                    opt_a = Some(b);
                }
            }
        }
    }

    // Find box B (smallest offset >= target_offset)
    let mut opt_b: Option<&GlyphBox> = None;
    for b in boxes {
        if b.offset >= target_offset {
            if opt_b.is_none() {
                opt_b = Some(b);
            } else {
                let current_b = opt_b.unwrap();
                let choose_new = if b.offset < current_b.offset {
                    true
                } else if b.offset == current_b.offset {
                    let line_b = lines.byte_to_line(b.offset).unwrap_or(0);
                    let line_cur = lines.byte_to_line(current_b.offset).unwrap_or(0);
                    
                    let b_on_line = line_b == line_target;
                    let cur_on_line = line_cur == line_target;
                    
                    if b_on_line && !cur_on_line {
                        true
                    } else if !b_on_line && cur_on_line {
                        false
                    } else {
                        // Prefer visually leftmost to anchor leading edge of characters at this offset.
                        b.bounds.origin.x < current_b.bounds.origin.x
                    }
                } else {
                    false
                };
                if choose_new {
                    opt_b = Some(b);
                }
            }
        }
    }

    match (opt_a, opt_b) {
        (Some(a), Some(b)) => {
            let dist_a = target_offset.saturating_sub(a.offset);
            let dist_b = b.offset.saturating_sub(target_offset);
            let closest_dist = dist_a.min(dist_b);
            
            if a.offset == b.offset {
                // Exact offset hit: return leftmost glyph's own leading edge directly.
                Some((point(b.bounds.origin.x, b.baseline), b.height, closest_dist))
            } else {
                let same_line = (f32::from(a.baseline) - f32::from(b.baseline)).abs() < 5.0;
                if same_line {
                    // Both boxes on the same visual line: interpolate x (smooth caret sliding).
                    let ratio = (target_offset - a.offset) as f32 / (b.offset - a.offset) as f32;
                    let interp_x = a.bounds.origin.x + ratio * (b.bounds.origin.x - a.bounds.origin.x);
                    let interp_baseline = a.baseline + ratio * (b.baseline - a.baseline);
                    let interp_height = a.height + ratio * (b.height - a.height);
                    Some((point(interp_x, interp_baseline), interp_height, closest_dist))
                } else {
                    // C4: Baselines diverge (e.g. box B is an integral/radical on a different line, or subscript/superscript).
                    let line_a = lines.byte_to_line(a.offset).unwrap_or(0);
                    let line_b = lines.byte_to_line(b.offset).unwrap_or(0);
                    
                    let choose_b = if line_target == line_a && line_target != line_b {
                        false
                    } else if line_target == line_b && line_target != line_a {
                        true
                    } else {
                        dist_b < dist_a
                    };

                    if choose_b {
                        // Target offset is closer to box B (e.g. we entered a new structural token or content block).
                        // Anchor to box B's left edge, baseline, and height.
                        Some((point(b.bounds.origin.x, b.baseline), b.height, closest_dist))
                    } else {
                        // Target offset is closer to box A. Anchor to box A's right edge.
                        let a_right = a.bounds.origin.x + a.bounds.size.width;
                        Some((point(a_right, a.baseline), a.height, closest_dist))
                    }
                }
            }
        }
        (Some(a), None) => {
            // Beyond the last offset on the page, place at right edge of last glyph.
            let last_x = a.bounds.origin.x + a.bounds.size.width;
            let closest_dist = target_offset.saturating_sub(a.offset);
            Some((point(last_x, a.baseline), a.height, closest_dist))
        }
        (None, Some(b)) => {
            // Before the first offset on the page.
            let closest_dist = b.offset.saturating_sub(target_offset);
            Some((point(b.bounds.origin.x, b.baseline), b.height, closest_dist))
        }
        (None, None) => None,
    }
}

pub fn move_in_direction(
    boxes: &[GlyphBox],
    current_pos: Point<Pixels>,
    direction: &str,
    current_offset: usize,
    _source: &Source,
) -> Option<(usize, Point<Pixels>, f32)> {
    if boxes.is_empty() {
        return None;
    }

    // For Up/Down, we use pure spatial search
    if direction == "up" || direction == "down" {
        let cy = current_pos.y;
        
        let candidates: Vec<&GlyphBox> = boxes.iter()
            .filter(|b| {
                if b.offset == current_offset {
                    return false;
                }
                
                let by = b.baseline;
                match direction {
                    "down" => by > cy + px(4.0),
                    "up" => by < cy - px(4.0),
                    _ => false,
                }
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let best = candidates.into_iter()
            .min_by(|a, b| {
                let ax = f32::from(a.bounds.origin.x);
                let ay = f32::from(a.baseline);
                let bx = f32::from(b.bounds.origin.x);
                let by = f32::from(b.baseline);
                let cx = f32::from(current_pos.x);
                let cy = f32::from(current_pos.y);

                let score_a = match direction {
                    "down" => (ay - cy) + 3.0 * (ax - cx).abs(),
                    "up" => (cy - ay) + 3.0 * (ax - cx).abs(),
                    _ => 0.0,
                };

                let score_b = match direction {
                    "down" => (by - cy) + 3.0 * (bx - cx).abs(),
                    "up" => (cy - by) + 3.0 * (bx - cx).abs(),
                    _ => 0.0,
                };

                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        return best.map(|b| (b.offset, point(b.bounds.origin.x, b.baseline), b.height));
    }

    // For Left/Right, we use line-aware logical/spatial constraints
    let curr_off = current_offset;
    let cx = current_pos.x;
    let cy = current_pos.y;

    // 1. Gather all logically valid candidates based on document flow direction
    let offset_valid_candidates: Vec<&GlyphBox> = boxes.iter()
        .filter(|b| {
            match direction {
                "right" => b.offset > curr_off,
                "left" => b.offset < curr_off,
                _ => false,
            }
        })
        .collect();

    if offset_valid_candidates.is_empty() {
        return None;
    }

    // 2. Identify candidates on the same line
    let same_line_threshold = px(25.0);
    let same_line_candidates: Vec<&GlyphBox> = offset_valid_candidates.iter()
        .filter(|b| {
            if (b.baseline - cy).abs() >= same_line_threshold {
                return false;
            }
            let bx = b.bounds.origin.x;
            match direction {
                "right" => bx > cx - px(1.0),
                "left" => bx < cx + px(1.0),
                _ => false,
            }
        })
        .cloned()
        .collect();

    let best_candidate = if !same_line_candidates.is_empty() {
        // Spatial 2D search on the same line
        same_line_candidates.into_iter()
            .min_by(|a, b| {
                let ax = f32::from(a.bounds.origin.x);
                let ay = f32::from(a.baseline);
                let bx = f32::from(b.bounds.origin.x);
                let by = f32::from(b.baseline);
                let cx = f32::from(current_pos.x);
                let cy = f32::from(current_pos.y);

                let score_a = match direction {
                    "right" => (ax - cx) + 3.0 * (ay - cy).abs(),
                    "left" => (cx - ax) + 3.0 * (ay - cy).abs(),
                    _ => 0.0,
                };

                let score_b = match direction {
                    "right" => (bx - cx) + 3.0 * (by - cy).abs(),
                    "left" => (cx - bx) + 3.0 * (by - cy).abs(),
                    _ => 0.0,
                };

                let cmp = score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal);
                if cmp == std::cmp::Ordering::Equal || (score_a - score_b).abs() < 0.01 {
                    match direction {
                        "right" => a.offset.cmp(&b.offset),
                        "left" => b.offset.cmp(&a.offset),
                        _ => std::cmp::Ordering::Equal,
                    }
                } else {
                    cmp
                }
            })
    } else {
        // Cross-line navigation
        let other_line_candidates: Vec<&GlyphBox> = offset_valid_candidates.iter()
            .filter(|b| {
                match direction {
                    "left" => b.baseline < cy - px(15.0),  // Preceding lines
                    "right" => b.baseline > cy + px(15.0), // Succeeding lines
                    _ => false,
                }
            })
            .cloned()
            .collect();

        if !other_line_candidates.is_empty() {
            other_line_candidates.into_iter()
                .min_by(|a, b| {
                    match direction {
                        "left" => b.offset.cmp(&a.offset),  // Prefer largest offset (end of previous line)
                        "right" => a.offset.cmp(&b.offset), // Prefer smallest offset (start of next line)
                        _ => std::cmp::Ordering::Equal,
                    }
                })
        } else {
            // Absolute fall-back: just choose the logically closest offset candidate in the allowed flow direction
            offset_valid_candidates.into_iter()
                .min_by(|a, b| {
                    match direction {
                        "left" => b.offset.cmp(&a.offset),  // Prefer closest preceding
                        "right" => a.offset.cmp(&b.offset), // Prefer closest succeeding
                        _ => std::cmp::Ordering::Equal,
                    }
                })
        }
    };

    best_candidate.map(|b| (b.offset, point(b.bounds.origin.x, b.baseline), b.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::syntax::Source;
    use crate::core::compiler::SimpleWorld;

    #[test]
    fn test_sqrt_glyph_box_baseline_alignment() {
        let source = Source::detached("$sqrt(2)$");
        let world = SimpleWorld::new(source.clone());
        let doc = typst::compile::<typst::layout::PagedDocument>(&world).output.unwrap();
        
        let mut boxes = Vec::new();
        let frame = &doc.pages[0].frame;
        collect_glyph_boxes_with_source(frame, gpui::Point::default(), &source, &mut boxes, 1.0);
        
        // Find the radical glyph box ('s' of sqrt starts at offset 1)
        let radical_box = boxes.iter().find(|b| b.offset == 1).expect("Could not find radical glyph box");
        
        println!("COLLECTED GLYPH BOXES:");
        for (idx, b) in boxes.iter().enumerate() {
            let symbol = source.text().get(b.offset..).and_then(|s| s.chars().next()).unwrap_or('?');
            println!("  [{}] Symbol: '{}' (offset={}), bounds={:?}, height={}, baseline={:?}, is_text={}", 
                idx, symbol, b.offset, b.bounds, b.height, b.baseline, b.is_text);
        }

        // Find the inner radicand glyph box ('2' starts at offset 6)
        let inner_box = boxes.iter().find(|b| b.offset == 6).expect("Could not find inner radicand glyph box");
        
        // Assert that the baseline of the radical glyph box is EXACTLY aligned with the baseline of the inner radicand box
        assert_eq!(radical_box.baseline, inner_box.baseline, "Radical baseline must align with inner radicand baseline");
        
        // Assert that the caret height for the radical is standard font size (same as inner text)
        assert_eq!(radical_box.height, inner_box.height, "Radical caret height must match inner radicand caret height");
    }

    #[test]
    fn test_user_exact_formula_glyph_boxes() {
        let source = Source::detached("$sqrt(2 x   + 1)$");
        let world = SimpleWorld::new(source.clone());
        let doc = typst::compile::<typst::layout::PagedDocument>(&world).output.unwrap();
        
        let mut boxes = Vec::new();
        let frame = &doc.pages[0].frame;
        collect_glyph_boxes_with_source(frame, gpui::Point::default(), &source, &mut boxes, 1.0);
        
        println!("COLLECTED USER GLYPH BOXES:");
        for (idx, b) in boxes.iter().enumerate() {
            let symbol = source.text().get(b.offset..).and_then(|s| s.chars().next()).unwrap_or('?');
            println!("  [{}] Symbol: '{}' (offset={}), bounds={:?}, height={}, baseline={:?}, is_text={}", 
                idx, symbol, b.offset, b.bounds, b.height, b.baseline, b.is_text);
        }

        // Test visual caret coordinate interpolation for target offsets in the whitespace gap (9 to 12)
        let pos_9 = find_cursor_position(frame, gpui::Point::default(), 9, &source, 1.0).unwrap().0;
        let pos_10 = find_cursor_position(frame, gpui::Point::default(), 10, &source, 1.0).unwrap().0;
        let pos_11 = find_cursor_position(frame, gpui::Point::default(), 11, &source, 1.0).unwrap().0;
        let pos_12 = find_cursor_position(frame, gpui::Point::default(), 12, &source, 1.0).unwrap().0;

        println!("INTERPOLATED CARET X COORDINATES:");
        println!("  Offset 9:  {:?}", pos_9.x);
        println!("  Offset 10: {:?}", pos_10.x);
        println!("  Offset 11: {:?}", pos_11.x);
        println!("  Offset 12: {:?}", pos_12.x);

        // Prove that the positions increase linearly in equal steps!
        let diff_9_10 = f32::from(pos_10.x - pos_9.x);
        let diff_10_11 = f32::from(pos_11.x - pos_10.x);
        let diff_11_12 = f32::from(pos_12.x - pos_11.x);

        assert!((diff_9_10 - diff_10_11).abs() < 0.001, "Linear step size must be equal");
        assert!((diff_10_11 - diff_11_12).abs() < 0.001, "Linear step size must be equal");
        assert!(diff_9_10 > 0.0, "Caret must move to the right as offset increases");
    }

    #[test]
    fn test_caret_position_consistency() {
        let source = Source::detached("ab");
        let boxes = vec![
            GlyphBox {
                offset: 0,
                bounds: Bounds {
                    origin: point(px(0.0), px(10.0)),
                    size: size(px(10.0), px(10.0)),
                },
                height: 10.0,
                baseline: px(10.0),
                is_text: true,
                is_radical: false,
            },
            GlyphBox {
                offset: 1,
                bounds: Bounds {
                    origin: point(px(10.0), px(10.0)),
                    size: size(px(0.0), px(10.0)),
                },
                height: 10.0,
                baseline: px(10.0),
                is_text: true,
                is_radical: false,
            },
            GlyphBox {
                offset: 1,
                bounds: Bounds {
                    origin: point(px(15.0), px(10.0)),
                    size: size(px(10.0), px(10.0)),
                },
                height: 10.0,
                baseline: px(10.0),
                is_text: true,
                is_radical: false,
            },
            GlyphBox {
                offset: 2,
                bounds: Bounds {
                    origin: point(px(25.0), px(10.0)),
                    size: size(px(0.0), px(10.0)),
                },
                height: 10.0,
                baseline: px(10.0),
                is_text: true,
                is_radical: false,
            },
        ];

        let res = find_cursor_position_in_boxes(&boxes, 1, &source).unwrap();
        // Since both have offset 1, exact hit: must return b's origin (leftmost) which is 10.0
        assert_eq!(res.0.x, px(10.0));
    }

    #[test]
    fn test_caret_at_end_of_line_with_space_and_t() {
        let text = "= Page 1: Native WYSIWYG Layout\n\nThis is a true hardware-accelerated editor running on GPUI. t\n\nTry clicking and dragging text here to see dynamic translucent highlights!".to_string();
        let source = Source::detached(text);
        let world = SimpleWorld::new(source.clone());
        let doc = typst::compile::<typst::layout::PagedDocument>(&world).output.unwrap();
        
        let mut boxes = Vec::new();
        let frame = &doc.pages[0].frame;
        collect_glyph_boxes_with_source(frame, gpui::Point::default(), &source, &mut boxes, 1.0);
        
        println!("COLLECTED GLYPH BOXES FOR ' t' DOCUMENT (offsets 85-100):");
        for (idx, b) in boxes.iter().enumerate() {
            if b.offset >= 85 && b.offset <= 100 {
                let symbol = source.text().get(b.offset..).and_then(|s| s.chars().next()).unwrap_or('?');
                println!("  [{}] Symbol: '{}' (offset={}), bounds={:?}, height={}, baseline={:?}, is_text={}", 
                    idx, symbol, b.offset, b.bounds, b.height, b.baseline, b.is_text);
            }
        }

        // The cursor position right after 't' (which is at offset 94)
        let res_93 = find_cursor_position(frame, gpui::Point::default(), 93, &source, 1.0).unwrap();
        let res_94 = find_cursor_position(frame, gpui::Point::default(), 94, &source, 1.0).unwrap();
        println!("CARET POSITION AT OFFSET 93: {:?}", res_93);
        println!("CARET POSITION AT OFFSET 94: {:?}", res_94);
    }

    #[test]
    fn test_caret_at_end_of_line_with_only_space() {
        let text = "= Page 1: Native WYSIWYG Layout\n\nThis is a true hardware-accelerated editor running on GPUI. \n\nTry clicking and dragging text here to see dynamic translucent highlights!".to_string();
        let source = Source::detached(text);
        let world = SimpleWorld::new(source.clone());
        let doc = typst::compile::<typst::layout::PagedDocument>(&world).output.unwrap();
        
        let mut boxes = Vec::new();
        let frame = &doc.pages[0].frame;
        collect_glyph_boxes_with_source(frame, gpui::Point::default(), &source, &mut boxes, 1.0);
        
        println!("COLLECTED GLYPH BOXES FOR SPACE DOCUMENT (offsets 85-100):");
        for (idx, b) in boxes.iter().enumerate() {
            if b.offset >= 85 && b.offset <= 100 {
                let symbol = source.text().get(b.offset..).and_then(|s| s.chars().next()).unwrap_or('?');
                println!("  [{}] Symbol: '{}' (offset={}), bounds={:?}, height={}, baseline={:?}, is_text={}", 
                    idx, symbol, b.offset, b.bounds, b.height, b.baseline, b.is_text);
            }
        }

        // The cursor position right after the space (which is at offset 93)
        let res_93 = find_cursor_position(frame, gpui::Point::default(), 93, &source, 1.0).unwrap();
        println!("CARET POSITION AT OFFSET 93 WITH ONLY SPACE: {:?}", res_93);
    }
}
