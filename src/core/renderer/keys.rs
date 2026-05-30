use super::math::find_math_structure_ranges;
use crate::core::editor::state::{EditorAction, EditorContext, EditorState};
use typst::syntax::Source;

fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Process a keyboard event for the WYSIWYG editor.
///
/// This function determines the appropriate `EditorAction` based on the current
/// editor state (markup, math, or code) and the key pressed.
/// It handles complex math structures, atomic deletions, and text navigation.
pub fn wysiwyg_key_event(
    source: &Source,
    state: &EditorState<'_>,
    key: &str,
    _shift_pressed: bool,
) -> EditorAction {
    let text = source.text();
    let pos = floor_char_boundary(text, state.cursor.min(text.len()));
    let had_selection = state
        .selection
        .clone()
        .filter(|r| r.start != r.end)
        .map(|r| {
            let start = floor_char_boundary(text, r.start.min(text.len()));
            let end = floor_char_boundary(text, r.end.min(text.len()));
            if start < end { start..end } else { end..start }
        });

    match key {
        "backspace" => {
            if let Some(action) = Some(state.context)
                .filter(|c| *c == EditorContext::Math)
                .and_then(|_| super::math::handle_math_deletion(source, state, "backspace"))
            {
                return action;
            }

            if let Some(sel) = had_selection {
                return EditorAction::Edit {
                    range: sel.start..sel.end,
                    replacement: String::new(),
                    new_cursor: sel.start,
                    new_selection: None,
                };
            } else if pos > 0 {
                let mut prev_char_len = 1;
                if let Some(prev_idx) = text[..pos].char_indices().map(|(idx, _)| idx).next_back() {
                    prev_char_len = pos - prev_idx;
                }
                let start = pos - prev_char_len;
                return EditorAction::Edit {
                    range: start..pos,
                    replacement: String::new(),
                    new_cursor: start,
                    new_selection: None,
                };
            }
        }
        "delete" => {
            if let Some(action) = Some(state.context)
                .filter(|c| *c == EditorContext::Math)
                .and_then(|_| super::math::handle_math_deletion(source, state, "delete"))
            {
                return action;
            }

            if let Some(sel) = had_selection {
                return EditorAction::Edit {
                    range: sel.start..sel.end,
                    replacement: String::new(),
                    new_cursor: sel.start,
                    new_selection: None,
                };
            } else if pos < text.len() {
                let mut next_char_len = 1;
                if let Some(next_char) = text[pos..].chars().next() {
                    next_char_len = next_char.len_utf8();
                }
                let end = pos + next_char_len;
                return EditorAction::Edit {
                    range: pos..end,
                    replacement: String::new(),
                    new_cursor: pos,
                    new_selection: None,
                };
            }
        }
        "left" => {
            if state.context == EditorContext::Math {
                if let Some(action) = had_selection
                    .as_ref()
                    .filter(|s| s.start + 1 == s.end && text.get((*s).clone()) == Some("?"))
                    .map(|s| EditorAction::MoveCursor {
                        new_cursor: s.start,
                    })
                {
                    return action;
                }
                if text[..pos].ends_with('?') {
                    return EditorAction::Select {
                        range: (pos - 1)..pos,
                        reversed: true,
                    };
                }
            }
            if pos > 0
                && let Some(prev_idx) = text[..pos].char_indices().map(|(idx, _)| idx).next_back()
            {
                if _shift_pressed {
                    let anchor = if let Some(sel) = had_selection {
                        if pos == sel.start { sel.end } else { sel.start }
                    } else {
                        pos
                    };
                    let range = if prev_idx < anchor {
                        prev_idx..anchor
                    } else {
                        anchor..prev_idx
                    };
                    return EditorAction::Select {
                        range,
                        reversed: prev_idx < anchor,
                    };
                }
                return EditorAction::MoveCursor {
                    new_cursor: prev_idx,
                };
            }
        }
        "right" => {
            if state.context == EditorContext::Math {
                if let Some(action) = had_selection
                    .as_ref()
                    .filter(|s| s.start + 1 == s.end && text.get((*s).clone()) == Some("?"))
                    .map(|s| EditorAction::MoveCursor { new_cursor: s.end })
                {
                    return action;
                }
                if text[pos..].starts_with('?') {
                    return EditorAction::Select {
                        range: pos..(pos + 1),
                        reversed: false,
                    };
                }
            }
            if pos < text.len()
                && let Some(next_char) = text[pos..].chars().next()
            {
                let next_idx = pos + next_char.len_utf8();
                if _shift_pressed {
                    let anchor = if let Some(sel) = had_selection {
                        if pos == sel.start { sel.end } else { sel.start }
                    } else {
                        pos
                    };
                    let range = if next_idx < anchor {
                        next_idx..anchor
                    } else {
                        anchor..next_idx
                    };
                    return EditorAction::Select {
                        range,
                        reversed: next_idx < anchor,
                    };
                }
                return EditorAction::MoveCursor {
                    new_cursor: next_idx,
                };
            }
        }
        "home" => {
            let start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if _shift_pressed {
                let anchor = if let Some(sel) = had_selection {
                    if pos == sel.start { sel.end } else { sel.start }
                } else {
                    pos
                };
                let range = if start < anchor {
                    start..anchor
                } else {
                    anchor..start
                };
                return EditorAction::Select {
                    range,
                    reversed: start < anchor,
                };
            }
            return EditorAction::MoveCursor { new_cursor: start };
        }
        "end" => {
            let end = text[pos..]
                .find('\n')
                .map(|i| pos + i)
                .unwrap_or(text.len());
            if _shift_pressed {
                let anchor = if let Some(sel) = had_selection {
                    if pos == sel.start { sel.end } else { sel.start }
                } else {
                    pos
                };
                let range = if end < anchor {
                    end..anchor
                } else {
                    anchor..end
                };
                return EditorAction::Select {
                    range,
                    reversed: end < anchor,
                };
            }
            return EditorAction::MoveCursor { new_cursor: end };
        }
        "enter" => {
            return EditorAction::Edit {
                range: pos..pos,
                replacement: "\n".to_string(),
                new_cursor: pos + 1,
                new_selection: None,
            };
        }
        "space" | " " => {
            return EditorAction::Edit {
                range: pos..pos,
                replacement: " ".to_string(),
                new_cursor: pos + 1,
                new_selection: None,
            };
        }
        "tab" => {
            if state.context == EditorContext::Math {
                // Find next `?` placeholder
                if pos < text.len() {
                    let next_char_len = text[pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    let search_start =
                        floor_char_boundary(text, (pos + next_char_len).min(text.len()));
                    if let Some(offset) = text[search_start..].find('?') {
                        return EditorAction::MoveCursor {
                            new_cursor: search_start + offset,
                        };
                    }
                }
                if let Some(offset) = text[..pos].find('?') {
                    return EditorAction::MoveCursor { new_cursor: offset };
                }
            } else {
                return EditorAction::Edit {
                    range: pos..pos,
                    replacement: "  ".to_string(),
                    new_cursor: pos + 2,
                    new_selection: None,
                };
            }
        }
        other => {
            if other.chars().count() == 1 {
                let typed_char = other.to_string();
                if let Some(sel) = had_selection {
                    let sel_start = sel.start;
                    let sel_end = sel.end;

                    if state.context == EditorContext::Math {
                        // Preserve wrappers: if selecting the entire attachment field, only overwrite inner
                        if let Some((full_range, inner_range)) =
                            find_math_structure_ranges(source, sel_start + 1)
                        {
                            if full_range == (sel_start..sel_end) {
                                return EditorAction::Edit {
                                    range: inner_range.clone(),
                                    replacement: typed_char.clone(),
                                    new_cursor: inner_range.start + typed_char.len(),
                                    new_selection: None,
                                };
                            }

                            // Case B: Selection is the argument list of a function call (e.g. `(2)` in `sqrt(2)`)
                            let args_range = inner_range.start.saturating_sub(1)
                                ..inner_range.end.saturating_add(1);
                            if args_range == (sel_start..sel_end)
                                && text.get(sel_start..sel_start + 1) == Some("(")
                                && text.get(sel_end - 1..sel_end) == Some(")")
                            {
                                return EditorAction::Edit {
                                    range: inner_range.clone(),
                                    replacement: typed_char.clone(),
                                    new_cursor: inner_range.start + typed_char.len(),
                                    new_selection: None,
                                };
                            }
                        }
                    }

                    // Normal replacement of selection
                    return EditorAction::Edit {
                        range: sel_start..sel_end,
                        replacement: typed_char.clone(),
                        new_cursor: sel_start + typed_char.len(),
                        new_selection: None,
                    };
                } else if state.context == EditorContext::Math
                    && text[pos..].trim_start().starts_with('?')
                {
                    // Find the exact index of `?` to handle any intermediate spaces gracefully
                    let offset = text[pos..].find('?').unwrap();
                    let q_pos = pos + offset;
                    // Overwrite the placeholder directly
                    return EditorAction::Edit {
                        range: q_pos..(q_pos + 1),
                        replacement: typed_char.clone(),
                        new_cursor: q_pos + typed_char.len(),
                        new_selection: None,
                    };
                } else {
                    return EditorAction::Edit {
                        range: pos..pos,
                        replacement: typed_char.clone(),
                        new_cursor: pos + typed_char.len(),
                        new_selection: None,
                    };
                }
            }
        }
    }
    EditorAction::None
}
