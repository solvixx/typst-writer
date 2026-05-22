use gpui::{px, point, Point, Pixels};
use typst::layout::{Frame, FrameItem};
use typst::syntax::Source;

// Precise, coordinate-aware pixel hit-testing
pub fn find_closest_offset(
    frame: &Frame,
    origin: Point<Pixels>,
    click_pos: Point<Pixels>,
    source: &Source,
) -> Option<(usize, Point<Pixels>)> {
    let mut closest: Option<(usize, Point<Pixels>, f32)> = None;

    fn walk(
        frame: &Frame,
        origin: Point<Pixels>,
        click_pos: Point<Pixels>,
        source: &Source,
        closest: &mut Option<(usize, Point<Pixels>, f32)>,
    ) {
        for (pos, item) in frame.items() {
            let item_x = origin.x + px(pos.x.to_pt() as f32);
            let item_y = origin.y + px(pos.y.to_pt() as f32);

            match item {
                FrameItem::Text(text_item) => {
                    let size_px = px(text_item.size.to_pt() as f32);
                    let mut current_x = item_x;
                    for glyph in &text_item.glyphs {
                        let glyph_width = px((glyph.x_advance.get() as f32) * (text_item.size.to_pt() as f32));
                        
                        let glyph_min_x = current_x;
                        let glyph_max_x = current_x + glyph_width;
                        let glyph_min_y = item_y - size_px;
                        let glyph_max_y = item_y;

                        // Absolute click distance relative to the bounding box of glyph
                        let dx = if click_pos.x < glyph_min_x {
                            f32::from(glyph_min_x - click_pos.x)
                        } else if click_pos.x > glyph_max_x {
                            f32::from(click_pos.x - glyph_max_x)
                        } else {
                            0.0
                        };

                        let dy = if click_pos.y < glyph_min_y {
                            f32::from(glyph_min_y - click_pos.y)
                        } else if click_pos.y > glyph_max_y {
                            f32::from(click_pos.y - glyph_max_y)
                        } else {
                            0.0
                        };

                        let dist_sq = dx * dx + dy * dy;

                        let is_closer = match closest {
                            Some((_, _, min_dist)) => dist_sq < *min_dist,
                            None => true,
                        };

                        if is_closer {
                            let span = glyph.span.0;
                            if let Some(linked_node) = source.find(span) {
                                let mut offset = linked_node.offset() + (glyph.span.1 as usize);
                                
                                let mut is_placeholder = false;
                                if let Some(text_slice) = linked_node.text().get((glyph.span.1 as usize)..) {
                                    if let Some(c) = text_slice.chars().next() {
                                        is_placeholder = c == '?';
                                    }
                                }

                                let mid_x = current_x + glyph_width / 2.0;
                                let cursor_x = if click_pos.x > mid_x && !is_placeholder {
                                    // Advance offset by the char length when clicking the right half
                                    if let Some(text_slice) = linked_node.text().get((glyph.span.1 as usize)..) {
                                        if let Some(c) = text_slice.chars().next() {
                                            offset += c.len_utf8();
                                        }
                                    }
                                    current_x + glyph_width
                                } else {
                                    current_x
                                };
                                let cursor_pos = point(cursor_x, item_y);
                                *closest = Some((offset, cursor_pos, dist_sq));
                            }
                        }

                        current_x += glyph_width;
                    }
                }
                FrameItem::Group(group) => {
                    let dx = px(group.transform.tx.to_pt() as f32);
                    let dy = px(group.transform.ty.to_pt() as f32);
                    walk(
                        &group.frame,
                        point(item_x + dx, item_y + dy),
                        click_pos,
                        source,
                        closest,
                    );
                }
                _ => {}
            }
        }
    }

    walk(frame, origin, click_pos, source, &mut closest);
    closest.map(|(offset, cursor_pos, _)| (offset, cursor_pos))
}

// Map target cursor character offset back to dynamic visual relative page coordinates
pub fn find_cursor_position(
    frame: &Frame,
    origin: Point<Pixels>,
    target_offset: usize,
    source: &Source,
) -> Option<Point<Pixels>> {
    fn walk(
        frame: &Frame,
        origin: Point<Pixels>,
        target_offset: usize,
        source: &Source,
    ) -> Option<(Point<Pixels>, usize)> {
        let mut best_match: Option<(Point<Pixels>, usize)> = None;

        for (pos, item) in frame.items() {
            let item_x = origin.x + px(pos.x.to_pt() as f32);
            let item_y = origin.y + px(pos.y.to_pt() as f32);

            match item {
                FrameItem::Text(text_item) => {
                    let mut current_x = item_x;
                    for glyph in &text_item.glyphs {
                        let glyph_width = px((glyph.x_advance.get() as f32) * (text_item.size.to_pt() as f32));
                        let span = glyph.span.0;
                        if let Some(linked_node) = source.find(span) {
                            let offset = linked_node.offset() + (glyph.span.1 as usize);
                            let diff = (offset as isize - target_offset as isize).abs() as usize;

                            let is_better = match best_match {
                                Some((_, min_diff)) => diff < min_diff,
                                None => true,
                            };

                            if is_better {
                                best_match = Some((point(current_x, item_y), diff));
                            }
                        }
                        current_x += glyph_width;
                    }
                }
                FrameItem::Group(group) => {
                    let dx = px(group.transform.tx.to_pt() as f32);
                    let dy = px(group.transform.ty.to_pt() as f32);
                    if let Some((pos, diff)) = walk(
                        &group.frame,
                        point(item_x + dx, item_y + dy),
                        target_offset,
                        source,
                    ) {
                        let is_better = match best_match {
                            Some((_, min_diff)) => diff < min_diff,
                            None => true,
                        };
                        if is_better {
                            best_match = Some((pos, diff));
                        }
                    }
                }
                _ => {}
            }
        }
        best_match
    }

    walk(frame, origin, target_offset, source).map(|(pos, _)| pos)
}

/// Given a byte offset inside a math expression, find the exact byte range of
/// the `?` placeholder field at that position.
///
/// Each `?` in a math template (e.g. `sum_(?)^(?)`) is an independent slot.
/// Clicking any `?` selects **only that character** so the user can type to
/// replace it, or press Backspace to clear it — without affecting sibling slots.
pub fn find_math_group_range(
    source: &Source,
    offset: usize,
) -> Option<std::ops::Range<usize>> {
    use typst::syntax::LinkedNode;

    let clamped = offset.min(source.text().len());
    let root = source.root();
    let linked = LinkedNode::new(root);

    // Check both sides of the offset to correctly detect clicks on either half of the placeholder
    let mut target_leaf = linked.leaf_at(clamped, typst::syntax::Side::Before);
    if target_leaf.as_ref().map(|l| source.text().get(l.range()) != Some("?")).unwrap_or(true) {
        target_leaf = linked.leaf_at(clamped, typst::syntax::Side::After);
    }
    
    let leaf = target_leaf?;

    // Treat it as a selectable field if the leaf text is exactly `?`
    let leaf_range = leaf.range();
    let is_placeholder = source.text()
        .get(leaf_range.clone())
        .map(|s| s == "?")
        .unwrap_or(false);

    if is_placeholder { 
        return Some(leaf_range); 
    }

    // Treat MathIdent (like `sum`, `alpha`) as selectable fields
    if leaf.kind() == typst::syntax::SyntaxKind::MathIdent {
        return Some(leaf_range);
    }

    // Treat unwrapped attachment values (like `c` in `sum^c`) as selectable fields
    if let Some(parent) = leaf.parent() {
        if parent.kind() == typst::syntax::SyntaxKind::MathAttach {
            // It's a direct child of MathAttach (so it's unwrapped)
            return Some(leaf_range);
        }
    }

    None
}

/// Find the tightest enclosing math *subfield* range starting from `offset`.
///
/// Used by the two-stage backspace in Math mode:
/// - Stage 1: selects the subfield (e.g. `_(a=1)`) without deleting.
/// - Stage 2: (caller deletes the active selection).
///
/// The returned range always includes any attachment operator (`_` or `^`)
/// immediately preceding a `MathDelimited` that is a child of `MathAttach`,
/// so that deleting the selection leaves syntactically valid source.
pub fn find_math_subfield_range(
    source: &Source,
    offset: usize,
) -> Option<std::ops::Range<usize>> {
    use typst::syntax::{LinkedNode, SyntaxKind, Side};

    let text = source.text();
    let clamped = offset.min(text.len());
    let root = source.root();
    let linked = LinkedNode::new(root);

    let leaf = linked.leaf_at(clamped, Side::Before)?;

    // Walk ancestors upward, looking for the first structured math container
    let mut current = Some(leaf.clone());
    while let Some(node) = current {
        match node.kind() {
            // A delimited group like `(a=1)` — check if it is an attachment arm
            SyntaxKind::MathDelimited => {
                let node_range = node.range();
                if let Some(parent) = node.parent() {
                    if parent.kind() == SyntaxKind::MathAttach {
                        // Include the preceding `_` or `^` attachment operator so
                        // the deleted range is syntactically self-contained
                        let start = node_range.start;
                        if start > 0 {
                            let op = text.get(start - 1..start);
                            if op == Some("_") || op == Some("^") {
                                return Some((start - 1)..node_range.end);
                            }
                        }
                    }
                }
                return Some(node_range);
            }
            // Fraction arm: select the whole a/b expression
            SyntaxKind::MathFrac => {
                return Some(node.range());
            }
            // Root arm: select sqrt(x) or cbrt(x) as a whole
            SyntaxKind::MathRoot => {
                return Some(node.range());
            }
            // Stop climbing at MathAttach — never select the whole base+sub+super group
            SyntaxKind::MathAttach => {
                break;
            }
            _ => {}
        }
        current = node.parent().cloned();
    }

    None
}

pub fn find_math_attachment_ranges(
    source: &Source,
    offset: usize,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    use typst::syntax::{LinkedNode, SyntaxKind, Side};

    let text = source.text();
    let clamped = offset.min(text.len());
    let root = source.root();
    let linked = LinkedNode::new(root);

    let leaf = linked.leaf_at(clamped, Side::Before)?;

    let mut current = Some(leaf.clone());
    while let Some(node) = current {
        if let Some(parent) = node.parent() {
            if parent.kind() == SyntaxKind::MathAttach {
                let node_range = node.range();
                let start = node_range.start;
                if start > 0 {
                    let op = text.get(start - 1..start);
                    if op == Some("_") || op == Some("^") {
                        let subfield_range = (start - 1)..node_range.end;
                        
                        let mut inner_start = node_range.start;
                        let mut inner_end = node_range.end;
                        
                        if inner_end > inner_start {
                            if text.get(inner_start..inner_start+1) == Some("(") && text.get(inner_end-1..inner_end) == Some(")") {
                                inner_start += 1;
                                inner_end -= 1;
                            }
                        }
                        return Some((subfield_range, inner_start..inner_end));
                    }
                }
            }
        }
        current = node.parent().cloned();
    }
    None
}

/// Returns the range to select if a deletion targets structural math elements.
pub fn intercept_math_deletion(
    source: &typst::syntax::Source,
    target_range: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    use typst::syntax::{LinkedNode, SyntaxKind, Side};

    let text = source.text();
    let root = source.root();
    let linked = LinkedNode::new(root);

    let leaf = linked.leaf_at(target_range.start, Side::After)?;

    // Case 1: Base of a MathAttach
    let mut current = Some(leaf.clone());
    while let Some(node) = current {
        if node.kind() == SyntaxKind::MathAttach {
            // Is `leaf` part of the base? The base is the first child.
            if let Some(first_child) = node.children().next() {
                if first_child.range().contains(&target_range.start) {
                    // Only intercept if the base is a simple identifier or text (unwrapped)
                    if first_child.kind() == SyntaxKind::MathIdent || first_child.kind() == SyntaxKind::MathText {
                        return Some(node.range());
                    }
                }
            }
        }
        current = node.parent().cloned();
    }

    // Case 2: Attachment operators `_`, `^`
    if leaf.kind() == SyntaxKind::Underscore || leaf.kind() == SyntaxKind::Hat {
        if let Some(parent) = leaf.parent() {
            if parent.kind() == SyntaxKind::MathAttach {
                let mut next = leaf.next_sibling();
                while let Some(sibling) = &next {
                    if sibling.kind() == SyntaxKind::Space || sibling.kind() == SyntaxKind::Error {
                        next = sibling.next_sibling();
                    } else {
                        break;
                    }
                }
                if let Some(val_node) = next {
                    let mut inner_start = val_node.range().start;
                    let mut inner_end = val_node.range().end;
                    if inner_end > inner_start {
                        if text.get(inner_start..inner_start+1) == Some("(") && text.get(inner_end-1..inner_end) == Some(")") {
                            inner_start += 1;
                            inner_end -= 1;
                        }
                    }
                    return Some(inner_start..inner_end);
                }
            }
        }
    }

    // Case 3: Parentheses of an attachment value
    if leaf.kind() == SyntaxKind::LeftParen || leaf.kind() == SyntaxKind::RightParen {
        if let Some(parent) = leaf.parent() {
            if parent.kind() == SyntaxKind::Math {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.kind() == SyntaxKind::MathAttach {
                        let start = parent.range().start;
                        if start > 0 {
                            let op = text.get(start - 1..start);
                            if op == Some("_") || op == Some("^") {
                                let mut inner_start = parent.range().start;
                                let mut inner_end = parent.range().end;
                                if inner_end > inner_start {
                                    if text.get(inner_start..inner_start+1) == Some("(") && text.get(inner_end-1..inner_end) == Some(")") {
                                        inner_start += 1;
                                        inner_end -= 1;
                                    }
                                }
                                return Some(inner_start..inner_end);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
