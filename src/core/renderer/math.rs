use crate::core::editor::state::{EditorAction, EditorState};
use std::ops::Range;
use typst::syntax::{LinkedNode, Source, SyntaxKind};

fn is_structural_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Slash
            | SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::Comma
            | SyntaxKind::Semicolon
            | SyntaxKind::Underscore
            | SyntaxKind::Hat
    )
}

fn is_placeholder(text: &str, range: Range<usize>) -> bool {
    text.get(range)
        .map(|s| s == "?" || s == "(?)")
        .unwrap_or(false)
}

pub fn find_parent_math_structure_for_leaf<'a>(node: &LinkedNode<'a>) -> Option<LinkedNode<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        let k = parent.kind();
        if k == SyntaxKind::MathFrac
            || k == SyntaxKind::MathRoot
            || k == SyntaxKind::MathDelimited
            || k == SyntaxKind::FuncCall
            || k == SyntaxKind::MathAttach
        {
            return Some(parent.clone());
        }
        current = parent.parent();
    }
    None
}

pub fn find_math_structure_ranges(
    source: &Source,
    offset: usize,
) -> Option<(Range<usize>, Range<usize>)> {
    let text = source.text();
    let root = source.root();
    let linked = LinkedNode::new(root);

    let clamped = offset.min(text.len());

    // Check both sides to resolve the leaf node accurately
    let mut leaf = linked.leaf_at(clamped, typst::syntax::Side::Before);
    if (leaf.is_none()
        || leaf.as_ref().unwrap().kind() == SyntaxKind::RightParen
        || leaf.as_ref().unwrap().kind() == SyntaxKind::LeftParen)
        && let Some(other_leaf) = linked.leaf_at(clamped, typst::syntax::Side::After)
    {
        leaf = Some(other_leaf);
    }

    let leaf = leaf?;

    let mut current = Some(leaf.clone());
    while let Some(node) = current {
        // 1. Delimited group or function call argument list
        if node.kind() == SyntaxKind::Args
            && let Some(parent) = node.parent()
            && parent.kind() == SyntaxKind::FuncCall
        {
            let mut current_arg_nodes = Vec::new();
            let mut args = Vec::new();
            for child in node.children() {
                let nk = child.kind();
                if nk == SyntaxKind::LeftParen
                    || nk == SyntaxKind::RightParen
                    || nk == SyntaxKind::Comma
                    || nk == SyntaxKind::Semicolon
                {
                    if !current_arg_nodes.is_empty() {
                        args.push(current_arg_nodes.clone());
                        current_arg_nodes.clear();
                    }
                } else {
                    current_arg_nodes.push(child);
                }
            }
            if !current_arg_nodes.is_empty() {
                args.push(current_arg_nodes);
            }
            for arg_nodes in args {
                let first = arg_nodes.iter().find(|n| n.kind() != SyntaxKind::Space);
                let last = arg_nodes
                    .iter()
                    .rev()
                    .find(|n| n.kind() != SyntaxKind::Space);
                if let (Some(f), Some(l)) = (first, last) {
                    let r = f.range().start..l.range().end;
                    if r.contains(&offset) || (offset == r.end && !r.is_empty()) {
                        return Some((parent.range(), r));
                    }
                }
            }
        }

        // 2. MathAttach attachment
        let mut current_node = node.clone();
        while let Some(parent) = current_node.parent() {
            if parent.kind() == SyntaxKind::MathAttach {
                // Find which child of MathAttach we are in (or under)
                let mut top_child = current_node;
                while let Some(p) = top_child.parent() {
                    if p.kind() == SyntaxKind::MathAttach {
                        break;
                    }
                    top_child = p.clone();
                }

                // CASE A: We are in/after the value (preceded by _ or ^)
                let mut prev = top_child.prev_sibling();
                while let Some(sib) = &prev {
                    if sib.kind() == SyntaxKind::Space {
                        prev = sib.prev_sibling();
                    } else {
                        break;
                    }
                }
                if let Some(op) = prev
                    .filter(|o| o.kind() == SyntaxKind::Underscore || o.kind() == SyntaxKind::Hat)
                {
                    let mut inner_start = top_child.range().start;
                    let mut inner_end = top_child.range().end;

                    // Un-nest Math if it's just a wrapper
                    if let Some(inner) = Some(&top_child)
                        .filter(|n| n.kind() == SyntaxKind::Math && n.children().count() == 1)
                        .and_then(|n| n.children().next())
                    {
                        inner_start = inner.range().start;
                        inner_end = inner.range().end;
                    }

                    // Handle parentheses wrapping
                    if inner_end > inner_start + 1
                        && text.get(inner_start..inner_start + 1) == Some("(")
                        && text.get(inner_end - 1..inner_end) == Some(")")
                    {
                        inner_start += 1;
                        inner_end -= 1;
                    }

                    return Some((
                        op.range().start..top_child.range().end,
                        inner_start..inner_end,
                    ));
                }

                // CASE B: We are at the operator itself (_ or ^)
                if top_child.kind() == SyntaxKind::Underscore || top_child.kind() == SyntaxKind::Hat
                {
                    let mut next = top_child.next_sibling();
                    while let Some(sib) = &next {
                        if sib.kind() == SyntaxKind::Space {
                            next = sib.next_sibling();
                        } else {
                            break;
                        }
                    }
                    if let Some(val) = next {
                        let mut inner_start = val.range().start;
                        let mut inner_end = val.range().end;

                        if let Some(inner) = Some(&val)
                            .filter(|n| n.kind() == SyntaxKind::Math && n.children().count() == 1)
                            .and_then(|n| n.children().next())
                        {
                            inner_start = inner.range().start;
                            inner_end = inner.range().end;
                        }

                        if inner_end > inner_start + 1
                            && text.get(inner_start..inner_start + 1) == Some("(")
                            && text.get(inner_end - 1..inner_end) == Some(")")
                        {
                            inner_start += 1;
                            inner_end -= 1;
                        }

                        return Some((
                            top_child.range().start..val.range().end,
                            inner_start..inner_end,
                        ));
                    }
                }

                break;
            }
            if parent.kind() == SyntaxKind::Equation || parent.kind() == SyntaxKind::Markup {
                break;
            }
            current_node = parent.clone();
        }

        // 3. Fraction arms
        if node.kind() == SyntaxKind::MathFrac {
            for child in node.children() {
                if child.kind() != SyntaxKind::Slash
                    && child.kind() != SyntaxKind::Space
                    && (child.range().contains(&offset)
                        || (offset == child.range().end && !child.range().is_empty()))
                {
                    return Some((node.range(), child.range()));
                }
            }
        }

        // 4. Delimited math groups
        if node.kind() == SyntaxKind::MathDelimited {
            for child in node.children() {
                if child.kind() != SyntaxKind::LeftParen
                    && child.kind() != SyntaxKind::RightParen
                    && (child.range().contains(&offset)
                        || (offset == child.range().end && !child.range().is_empty()))
                {
                    return Some((node.range(), child.range()));
                }
            }
        }

        // 5. MathRoot radicand
        if node.kind() == SyntaxKind::MathRoot {
            for child in node.children() {
                if child.kind() != SyntaxKind::Root
                    && child.kind() != SyntaxKind::Space
                    && (child.range().contains(&offset)
                        || (offset == child.range().end && !child.range().is_empty()))
                {
                    return Some((node.range(), child.range()));
                }
            }
        }

        current = node.parent().cloned();
    }
    None
}

pub fn walk_ast(source: &Source, cursor: usize, direction: &str) -> Option<usize> {
    let text = source.text();
    let root = source.root();
    let linked = LinkedNode::new(root);

    if direction == "right" {
        let mut node = linked.leaf_at(cursor, typst::syntax::Side::After);
        while let Some(current) = node {
            let range = current.range();
            if is_semantic(&current) {
                if cursor < range.start {
                    return Some(range.start);
                }
                if cursor < range.end {
                    let next_idx = text[cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| cursor + i)
                        .unwrap_or(range.end);
                    return Some(next_idx);
                }
            }
            node = current.next_leaf();
        }
        (cursor < text.len()).then_some(text.len())
    } else {
        let mut node = linked.leaf_at(cursor, typst::syntax::Side::Before);
        while let Some(current) = node {
            let range = current.range();
            if is_semantic(&current) {
                if cursor > range.end {
                    return Some(range.end);
                }
                if cursor > range.start {
                    let prev_idx = text[..cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(range.start);
                    return Some(prev_idx);
                }
            }
            node = current.prev_leaf();
        }
        (cursor > 0).then_some(0)
    }
}

pub fn is_semantic(node: &LinkedNode) -> bool {
    let kind = node.kind();
    if node.text() == "?" {
        return true;
    }

    !matches!(
        kind,
        SyntaxKind::Space
            | SyntaxKind::Underscore
            | SyntaxKind::Hat
            | SyntaxKind::Slash
            | SyntaxKind::LeftParen
            | SyntaxKind::RightParen
            | SyntaxKind::LeftBracket
            | SyntaxKind::RightBracket
            | SyntaxKind::LeftBrace
            | SyntaxKind::RightBrace
            | SyntaxKind::Comma
            | SyntaxKind::Semicolon
            | SyntaxKind::Root
            | SyntaxKind::Dollar
            | SyntaxKind::Math
    )
}

pub fn handle_math_selection_deletion(
    source: &Source,
    text: &str,
    sel_start: usize,
    sel_end: usize,
    _is_backspace: bool,
) -> Option<EditorAction> {
    let state = EditorState {
        text,
        cursor: sel_start,
        selection: Some(sel_start..sel_end),
        context: crate::core::editor::EditorContext::Math,
    };
    handle_math_deletion(source, &state, "backspace")
}

pub fn handle_math_deletion(
    source: &Source,
    state: &EditorState<'_>,
    key: &str,
) -> Option<EditorAction> {
    let text = source.text();
    let is_backspace = key == "backspace";
    let had_selection = state
        .selection
        .clone()
        .filter(|r| r.start != r.end)
        .map(|r| if r.start < r.end { r } else { r.end..r.start });

    // =======================================================
    // CASE A: ACTIVE SELECTION (Atomic Removal or Restoration)
    // =======================================================
    if let Some(sel) = had_selection {
        // 1. Expand incomplete selection first
        if let Some(expanded) = expand_incomplete_selection(source, sel.start, sel.end) {
            return Some(EditorAction::Select {
                range: expanded,
                reversed: true,
            });
        }

        // 2. Principle 3: Atomic Removal (Whole structures or identified arms)
        if find_math_structure_ranges(source, sel.start + 1).is_some_and(|(f, _)| f == sel) {
            return Some(EditorAction::Edit {
                range: sel.clone(),
                replacement: String::new(),
                new_cursor: sel.start,
                new_selection: None,
            });
        }

        let linked = LinkedNode::new(source.root());
        if linked
            .leaf_at(sel.start, typst::syntax::Side::After)
            .and_then(|leaf| find_parent_math_structure_for_leaf(&leaf))
            .is_some_and(|p| p.range() == sel)
        {
            return Some(EditorAction::Edit {
                range: sel.clone(),
                replacement: String::new(),
                new_cursor: sel.start,
                new_selection: None,
            });
        }

        // 3. Principle 3: Atomic Removal (Placeholders)
        if let Some(full) = Some(sel.clone())
            .filter(|s| is_placeholder(text, s.clone()))
            .and_then(|s| find_math_structure_ranges(source, s.start + 1).filter(|(_, i)| i == &s))
            .map(|(f, _)| f)
        {
            return Some(EditorAction::Edit {
                range: full.clone(),
                replacement: String::new(),
                new_cursor: full.start,
                new_selection: None,
            });
        }

        // 4. Principle 2 (Selection-based): Restoration of nested structures
        // e.g. deleting `2` in `(2)` -> `(?)`
        if find_math_structure_ranges(source, sel.start + 1)
            .is_some_and(|(_, i)| i == sel && !is_placeholder(text, sel.clone()))
        {
            return Some(EditorAction::Edit {
                range: sel.clone(),
                replacement: "?".to_string(),
                new_cursor: sel.start,
                new_selection: None,
            });
        }

        return None;
    }

    // =======================================================
    // CASE B: NO SELECTION (Upgrade or Restoration)
    // =======================================================
    let pos = state.cursor.min(text.len());

    // Find the range of consecutive spaces around the cursor
    let mut start_spaces = pos;
    while start_spaces > 0 {
        if text[..start_spaces].ends_with(' ') {
            start_spaces -= 1;
        } else {
            break;
        }
    }
    let mut end_spaces = pos;
    while end_spaces < text.len() {
        if text[end_spaces..].starts_with(' ') {
            end_spaces += 1;
        } else {
            break;
        }
    }
    let space_count = end_spaces - start_spaces;

    if space_count > 1 {
        // If there are multiple spaces, collapse them down to a single space!
        return Some(EditorAction::Edit {
            range: start_spaces..end_spaces,
            replacement: " ".to_string(),
            new_cursor: start_spaces + 1,
            new_selection: None,
        });
    }

    // Otherwise, if there is at most a single space, we act on the adjacent non-whitespace character.
    // For backspace, we act before the space (at start_spaces).
    // For delete, we act after the space (at end_spaces).
    let active_pos = if is_backspace {
        start_spaces
    } else {
        end_spaces
    };

    let target_range = if is_backspace {
        if active_pos == 0 {
            return None;
        }
        let mut prev_char_len = 1;
        if let Some(prev_idx) = text[..active_pos]
            .char_indices()
            .map(|(idx, _)| idx)
            .next_back()
        {
            prev_char_len = active_pos - prev_idx;
        }
        (active_pos - prev_char_len)..active_pos
    } else {
        if active_pos >= text.len() {
            return None;
        }
        let mut next_char_len = 1;
        if let Some(next_char) = text[active_pos..].chars().next() {
            next_char_len = next_char.len_utf8();
        }
        active_pos..(active_pos + next_char_len)
    };

    let linked = LinkedNode::new(source.root());
    let mut leaf = if is_backspace {
        linked.leaf_at(active_pos, typst::syntax::Side::Before)
    } else {
        linked.leaf_at(active_pos, typst::syntax::Side::After)
    }?;

    // If the resolved leaf is a space node, traverse to the next/prev non-space leaf
    while leaf.kind() == SyntaxKind::Space {
        if is_backspace {
            if let Some(prev) = leaf.prev_leaf() {
                leaf = prev;
            } else {
                break;
            }
        } else {
            if let Some(next) = leaf.next_leaf() {
                leaf = next;
            } else {
                break;
            }
        }
    }

    // 1. Principle 1: Selection Upgrade for placeholders
    if is_placeholder(text, target_range.clone()) {
        if let Some(full) = find_math_structure_ranges(source, target_range.end)
            .filter(|(_, i)| i == &target_range)
            .map(|(f, _)| f)
        {
            return Some(EditorAction::Select {
                range: full,
                reversed: true,
            });
        }
        if let Some(parent) = find_parent_math_structure_for_leaf(&leaf) {
            return Some(EditorAction::Select {
                range: parent.range(),
                reversed: true,
            });
        }
    }

    // 1.5 Principle 1 (Inverse): Selection Upgrade for placeholders (trailing edge)
    if !is_backspace && active_pos > 0 {
        let mut prev_char_len = 1;
        if let Some(prev_idx) = text[..active_pos]
            .char_indices()
            .map(|(idx, _)| idx)
            .next_back()
        {
            prev_char_len = active_pos - prev_idx;
        }
        let prev_range = (active_pos - prev_char_len)..active_pos;
        if let Some(full) = Some(prev_range.clone())
            .filter(|pr| is_placeholder(text, pr.clone()))
            .and_then(|pr| find_math_structure_ranges(source, active_pos).filter(|(_, i)| i == &pr))
            .map(|(f, _)| f)
        {
            return Some(EditorAction::Select {
                range: full,
                reversed: true,
            });
        }
    }

    // 2. Principle 1: Selection Upgrade for structural delimiters and operators
    if is_structural_kind(leaf.kind()) {
        if let Some((full, _inner)) = find_math_structure_ranges(
            source,
            if is_backspace {
                active_pos
            } else {
                active_pos + 1
            },
        ) {
            return Some(EditorAction::Select {
                range: full,
                reversed: true,
            });
        }
        if let Some(parent) = find_parent_math_structure_for_leaf(&leaf) {
            if leaf.kind() == SyntaxKind::LeftParen {
                let inner_start = parent.range().start + 1;
                let mut inner_end = parent.range().end;
                if inner_end > inner_start && text.get(inner_end - 1..inner_end) == Some(")") {
                    inner_end -= 1;
                }
                return Some(EditorAction::Select {
                    range: inner_start..inner_end,
                    reversed: true,
                });
            }
            return Some(EditorAction::Select {
                range: parent.range(),
                reversed: true,
            });
        }
    }

    // 3. Principle 2: Field Restoration (boundary detection)
    if let Some((_full, inner)) = find_math_structure_ranges(source, target_range.end) {
        let is_at_field_end = inner == target_range;
        let is_at_field_start = is_backspace && active_pos == inner.start;

        if (is_at_field_end || is_at_field_start) && !is_placeholder(text, inner.clone()) {
            return Some(EditorAction::Edit {
                range: inner.clone(),
                replacement: "?".to_string(),
                new_cursor: inner.start,
                new_selection: None,
            });
        }

        // If we are inside a rich field, let standard deletion handle it.
        if inner.contains(&target_range.start) && !is_placeholder(text, target_range.clone()) {
            return None;
        }
    }

    // 4. Principle 1: Selection Upgrade for atomic IDs and structure edges
    if leaf.kind() == SyntaxKind::MathIdent {
        return Some(EditorAction::Select {
            range: leaf.range(),
            reversed: true,
        });
    }
    if let Some(parent) = find_parent_math_structure_for_leaf(&leaf).filter(|p| {
        (is_backspace && p.range().end == active_pos)
            || (!is_backspace && p.range().start == active_pos)
    }) {
        return Some(EditorAction::Select {
            range: parent.range(),
            reversed: true,
        });
    }

    None
}

pub fn expand_incomplete_selection(
    source: &Source,
    sel_start: usize,
    sel_end: usize,
) -> Option<Range<usize>> {
    let root = source.root();
    let linked = LinkedNode::new(root);

    let mut current_offset = sel_start;
    let mut expanded_start = sel_start;
    let mut expanded_end = sel_end;

    while current_offset < sel_end {
        if let Some(leaf) = linked.leaf_at(current_offset, typst::syntax::Side::After) {
            let mut unit_start = leaf.range().start;
            let mut unit_end = leaf.range().end;

            match leaf.kind() {
                SyntaxKind::MathIdent => {
                    if let Some(parent) = leaf.parent() {
                        if parent.kind() == SyntaxKind::MathAttach {
                            let mut is_base = true;
                            let mut prev = leaf.prev_sibling();
                            while let Some(sib) = prev {
                                if sib.kind() == SyntaxKind::Underscore
                                    || sib.kind() == SyntaxKind::Hat
                                {
                                    is_base = false;
                                    break;
                                }
                                prev = sib.prev_sibling();
                            }
                            if is_base {
                                unit_start = parent.range().start;
                                unit_end = parent.range().end;
                            }
                        } else if parent.kind() == SyntaxKind::FuncCall
                            && parent
                                .children()
                                .next()
                                .is_some_and(|fc| fc.range() == leaf.range())
                        {
                            unit_start = parent.range().start;
                            unit_end = parent.range().end;
                        }
                    }
                }
                SyntaxKind::Underscore | SyntaxKind::Hat => {
                    if let Some(val) = leaf
                        .parent()
                        .filter(|p| p.kind() == SyntaxKind::MathAttach)
                        .and_then(|_p| {
                            let mut next = leaf.next_sibling();
                            while let Some(sib) = &next {
                                if sib.kind() == SyntaxKind::Space
                                    || sib.kind() == SyntaxKind::Error
                                {
                                    next = sib.next_sibling();
                                } else {
                                    break;
                                }
                            }
                            next
                        })
                    {
                        unit_end = val.range().end;
                    }
                }
                SyntaxKind::LeftParen
                | SyntaxKind::RightParen
                | SyntaxKind::LeftBracket
                | SyntaxKind::RightBracket
                | SyntaxKind::LeftBrace
                | SyntaxKind::RightBrace => {
                    if let Some(parent) = leaf.parent().filter(|p| {
                        p.kind() == SyntaxKind::MathDelimited || p.kind() == SyntaxKind::Args
                    }) {
                        unit_start = parent.range().start;
                        unit_end = parent.range().end;
                    }
                }
                SyntaxKind::Slash => {
                    if let Some(parent) = leaf.parent().filter(|p| p.kind() == SyntaxKind::MathFrac)
                    {
                        unit_start = parent.range().start;
                        unit_end = parent.range().end;
                    }
                }
                SyntaxKind::Root => {
                    if let Some(parent) = leaf.parent().filter(|p| p.kind() == SyntaxKind::MathRoot)
                    {
                        unit_start = parent.range().start;
                        unit_end = parent.range().end;
                    }
                }
                SyntaxKind::Comma | SyntaxKind::Semicolon => {
                    if let Some(parent) = leaf.parent().filter(|p| {
                        p.kind() == SyntaxKind::Args || p.kind() == SyntaxKind::MathRoot
                    }) {
                        unit_start = parent.range().start;
                        unit_end = parent.range().end;
                    }
                }
                _ => {}
            }

            if unit_start < expanded_start {
                expanded_start = unit_start;
            }
            if unit_end > expanded_end {
                expanded_end = unit_end;
            }

            current_offset = leaf.range().end.max(current_offset + 1);
        } else {
            break;
        }
    }

    let result = expanded_start..expanded_end;
    if result != (sel_start..sel_end) {
        Some(result)
    } else {
        None
    }
}
