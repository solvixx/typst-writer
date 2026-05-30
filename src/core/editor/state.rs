use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorContext {
    Markup,
    Math,
    Code,
}

#[derive(Debug, Clone)]
pub struct EditorState<'a> {
    pub text: &'a str,
    pub cursor: usize,
    pub selection: Option<Range<usize>>,
    pub context: EditorContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    Edit {
        range: Range<usize>,
        replacement: String,
        new_cursor: usize,
        new_selection: Option<Range<usize>>,
    },
    Select {
        range: Range<usize>,
        reversed: bool,
    },
    MoveCursor {
        new_cursor: usize,
    },
    None,
}

/// Maps a byte offset from one version of text to another, attempting to preserve
/// the logical position across edits using longest common prefix/suffix matching.
pub fn map_offset_between_texts(current: &str, compiled: &str, offset: usize) -> usize {
    let current_len = current.len();
    let compiled_len = compiled.len();

    // Find longest common prefix
    let mut prefix_len = 0;
    let current_chars: Vec<char> = current.chars().collect();
    let compiled_chars: Vec<char> = compiled.chars().collect();

    while prefix_len < current_chars.len() && prefix_len < compiled_chars.len() {
        if current_chars[prefix_len] == compiled_chars[prefix_len] {
            prefix_len += 1;
        } else {
            break;
        }
    }

    // Convert character prefix length back to byte offset
    let prefix_byte = current_chars[..prefix_len]
        .iter()
        .map(|c| c.len_utf8())
        .sum::<usize>();

    if offset <= prefix_byte {
        return offset;
    }

    // Find longest common suffix (not overlapping with prefix)
    let mut suffix_len = 0;
    while suffix_len < current_chars.len() - prefix_len
        && suffix_len < compiled_chars.len() - prefix_len
    {
        let cur_idx = current_chars.len() - 1 - suffix_len;
        let comp_idx = compiled_chars.len() - 1 - suffix_len;
        if current_chars[cur_idx] == compiled_chars[comp_idx] {
            suffix_len += 1;
        } else {
            break;
        }
    }

    let suffix_byte_cur = current_chars[current_chars.len() - suffix_len..]
        .iter()
        .map(|c| c.len_utf8())
        .sum::<usize>();
    let suffix_byte_comp = compiled_chars[compiled_chars.len() - suffix_len..]
        .iter()
        .map(|c| c.len_utf8())
        .sum::<usize>();

    if offset >= current_len - suffix_byte_cur {
        let diff_from_end = current_len - offset;
        return compiled_len.saturating_sub(diff_from_end);
    }

    // Inside the edit zone, interpolate or snap to prefix_byte
    let edit_len_cur = (current_len - suffix_byte_cur).saturating_sub(prefix_byte);
    let edit_len_comp = (compiled_len - suffix_byte_comp).saturating_sub(prefix_byte);

    if edit_len_cur == 0 {
        prefix_byte
    } else {
        let ratio = (offset - prefix_byte) as f64 / edit_len_cur as f64;
        let mapped = prefix_byte as f64 + ratio * edit_len_comp as f64;
        (mapped.round() as usize).min(compiled_len - suffix_byte_comp)
    }
}
