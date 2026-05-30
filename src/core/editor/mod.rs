pub mod state;
pub mod undo;

pub use state::*;
pub use undo::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::renderer::keys::wysiwyg_key_event as handle_key_event;
    use typst::syntax::Source;

    #[test]
    fn test_sqrt_three_stage_atomic_deletion() {
        let source = Source::detached("$sqrt(2)$");

        // --- STAGE 1: Pressing backspace/delete on content inside a math structure restores it to '?' placeholder ---
        let state_delete = EditorState {
            text: source.text(),
            cursor: 6, // Before '2'
            selection: None,
            context: EditorContext::Math,
        };
        let action_del = handle_key_event(&source, &state_delete, "delete", false);
        assert_eq!(
            action_del,
            EditorAction::Edit {
                range: 6..7,
                replacement: "?".to_string(),
                new_cursor: 6,
                new_selection: None,
            }
        );

        let state_bs = EditorState {
            text: source.text(),
            cursor: 7, // After '2'
            selection: None,
            context: EditorContext::Math,
        };
        let action_bs = handle_key_event(&source, &state_bs, "backspace", false);
        assert_eq!(
            action_bs,
            EditorAction::Edit {
                range: 6..7,
                replacement: "?".to_string(),
                new_cursor: 6,
                new_selection: None,
            }
        );

        // --- STAGE 2: Pressing backspace on cursor positioned before '?' selects the whole structure ---
        let source_placeholder = Source::detached("$sqrt(?)$");
        let state_before_placeholder = EditorState {
            text: source_placeholder.text(),
            cursor: 6, // Before '?'
            selection: None,
            context: EditorContext::Math,
        };
        let action_sel = handle_key_event(
            &source_placeholder,
            &state_before_placeholder,
            "backspace",
            false,
        );
        assert_eq!(
            action_sel,
            EditorAction::Select {
                range: 1..8,
                reversed: true
            }
        );

        // --- STAGE 3: Pressing backspace when whole structure is selected deletes it ---
        let state_sel = EditorState {
            text: source_placeholder.text(),
            cursor: 1,
            selection: Some(1..8),
            context: EditorContext::Math,
        };
        let action_del_final =
            handle_key_event(&source_placeholder, &state_sel, "backspace", false);
        assert_eq!(
            action_del_final,
            EditorAction::Edit {
                range: 1..8,
                replacement: "".to_string(),
                new_cursor: 1,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_frac_three_stage_atomic_deletion() {
        let source = Source::detached("$1/2$");

        // --- STAGE 1: Backspace after '1' (numerator) restores it to '?' ---
        let state_bs = EditorState {
            text: source.text(),
            cursor: 2, // after '1'
            selection: None,
            context: EditorContext::Math,
        };
        let action_bs = handle_key_event(&source, &state_bs, "backspace", false);
        assert_eq!(
            action_bs,
            EditorAction::Edit {
                range: 1..2,
                replacement: "?".to_string(),
                new_cursor: 1,
                new_selection: None,
            }
        );

        // --- STAGE 2: Pressing backspace after denominator '2' restores it to '?' ---
        let state_denom_bs = EditorState {
            text: source.text(),
            cursor: 4, // after '2'
            selection: None,
            context: EditorContext::Math,
        };
        let action_denom_bs = handle_key_event(&source, &state_denom_bs, "backspace", false);
        assert_eq!(
            action_denom_bs,
            EditorAction::Edit {
                range: 3..4,
                replacement: "?".to_string(),
                new_cursor: 3,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_root_three_stage_atomic_deletion() {
        let source = Source::detached("$√x$");

        // --- STAGE 1: Backspace after radicand 'x' restores it to '?' ---
        let state_bs = EditorState {
            text: source.text(),
            cursor: 5, // after 'x' (byte index 5 because '√' is 3 bytes)
            selection: None,
            context: EditorContext::Math,
        };
        let action_bs = handle_key_event(&source, &state_bs, "backspace", false);
        assert_eq!(
            action_bs,
            EditorAction::Edit {
                range: 4..5, // Radicand 'x'
                replacement: "?".to_string(),
                new_cursor: 4,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_sqrt_typing_over_args_preserves_wrapper() {
        let source = Source::detached("$sqrt(2)$");
        let state = EditorState {
            text: source.text(),
            cursor: 8,
            selection: Some(5..8), // Selection is '(2)'
            context: EditorContext::Math,
        };
        // Typing '3' over selection '(2)' overwrites only the inner '2', preserving the parentheses
        let action = handle_key_event(&source, &state, "3", false);
        assert_eq!(
            action,
            EditorAction::Edit {
                range: 6..7, // Only inner '2'
                replacement: "3".to_string(),
                new_cursor: 7,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_root_two_stage_atomic_deletion() {
        let source = Source::detached("$root(?,?)$");

        // 1. Backspace after first '?' -> Selects whole root(?,?)
        let state1 = EditorState {
            text: source.text(),
            cursor: 7, // after first '?'
            selection: None,
            context: EditorContext::Math,
        };
        let action1 = handle_key_event(&source, &state1, "backspace", false);
        assert_eq!(
            action1,
            EditorAction::Select {
                range: 1..10,
                reversed: true
            }
        );

        // 2. Backspace after comma -> Selects whole root(?,?)
        let state2 = EditorState {
            text: source.text(),
            cursor: 8, // after comma
            selection: None,
            context: EditorContext::Math,
        };
        let action2 = handle_key_event(&source, &state2, "backspace", false);
        assert_eq!(
            action2,
            EditorAction::Select {
                range: 1..10,
                reversed: true
            }
        );

        // 3. Backspace when whole root(?,?) is selected -> Deletes it
        let state3 = EditorState {
            text: source.text(),
            cursor: 10,
            selection: Some(1..10),
            context: EditorContext::Math,
        };
        let action3 = handle_key_event(&source, &state3, "backspace", false);
        assert_eq!(
            action3,
            EditorAction::Edit {
                range: 1..10,
                replacement: "".to_string(),
                new_cursor: 1,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_frac_two_stage_atomic_deletion() {
        let source = Source::detached("$?/?$");

        // 1. Backspace after first '?' -> Selects whole ?/?
        let state1 = EditorState {
            text: source.text(),
            cursor: 2, // after '?'
            selection: None,
            context: EditorContext::Math,
        };
        let action1 = handle_key_event(&source, &state1, "backspace", false);
        assert_eq!(
            action1,
            EditorAction::Select {
                range: 1..4,
                reversed: true
            }
        );

        // 2. Backspace when whole ?/? is selected -> Deletes it
        let state2 = EditorState {
            text: source.text(),
            cursor: 4,
            selection: Some(1..4),
            context: EditorContext::Math,
        };
        let action2 = handle_key_event(&source, &state2, "backspace", false);
        assert_eq!(
            action2,
            EditorAction::Edit {
                range: 1..4,
                replacement: "".to_string(),
                new_cursor: 1,
                new_selection: None,
            }
        );
    }

    #[test]
    fn test_math_arrow_navigation() {
        let source = Source::detached("$sqrt(?)$");

        // 1. Moving right when cursor is before '?' -> Selects '?'
        let state1 = EditorState {
            text: source.text(),
            cursor: 6, // before '?'
            selection: None,
            context: EditorContext::Math,
        };
        let action1 = handle_key_event(&source, &state1, "right", false);
        assert_eq!(
            action1,
            EditorAction::Select {
                range: 6..7,
                reversed: false
            }
        );

        // 2. Moving right when '?' is selected -> Moves past '?' to offset 7
        let state2 = EditorState {
            text: source.text(),
            cursor: 6,
            selection: Some(6..7),
            context: EditorContext::Math,
        };
        let action2 = handle_key_event(&source, &state2, "right", false);
        assert_eq!(action2, EditorAction::MoveCursor { new_cursor: 7 });

        // 3. Moving left when cursor is after '?' -> Selects '?'
        let state3 = EditorState {
            text: source.text(),
            cursor: 7, // after '?'
            selection: None,
            context: EditorContext::Math,
        };
        let action3 = handle_key_event(&source, &state3, "left", false);
        assert_eq!(
            action3,
            EditorAction::Select {
                range: 6..7,
                reversed: true
            }
        );

        // 4. Moving left when '?' is selected -> Moves past '?' to offset 6
        let state4 = EditorState {
            text: source.text(),
            cursor: 6,
            selection: Some(6..7),
            context: EditorContext::Math,
        };
        let action4 = handle_key_event(&source, &state4, "left", false);
        assert_eq!(action4, EditorAction::MoveCursor { new_cursor: 6 });
    }

    #[test]
    fn test_math_whitespace_deletion() {
        // Source contains "2 x     + 1" where '2' is at 1, 'x' is at 3, spaces are 4..9, '+' is at 9.
        let source = Source::detached("$2 x     + 1$");

        // 1. Backspace from middle/end of whitespace (offset 9) -> Collapses spaces 4..9 to a single space " "
        let state_bs = EditorState {
            text: source.text(),
            cursor: 9,
            selection: None,
            context: EditorContext::Math,
        };
        let action_bs = handle_key_event(&source, &state_bs, "backspace", false);
        assert_eq!(
            action_bs,
            EditorAction::Edit {
                range: 4..9,
                replacement: " ".to_string(),
                new_cursor: 5,
                new_selection: None,
            }
        );

        // 2. Delete from start of whitespace (offset 4) -> Collapses spaces 4..9 to a single space " "
        let state_del = EditorState {
            text: source.text(),
            cursor: 4,
            selection: None,
            context: EditorContext::Math,
        };
        let action_del = handle_key_event(&source, &state_del, "delete", false);
        assert_eq!(
            action_del,
            EditorAction::Edit {
                range: 4..9,
                replacement: " ".to_string(),
                new_cursor: 5,
                new_selection: None,
            }
        );

        // 3. User's exact example: "sqrt(2 x   [here] + 1)"
        // spaces are 9..12. Cursor [here] is at offset 10 (after two spaces).
        let user_source = Source::detached("$sqrt(2 x   + 1)$");
        let user_state = EditorState {
            text: user_source.text(),
            cursor: 10,
            selection: None,
            context: EditorContext::Math,
        };
        let user_action1 = handle_key_event(&user_source, &user_state, "backspace", false);
        assert_eq!(
            user_action1,
            EditorAction::Edit {
                range: 9..12,
                replacement: " ".to_string(),
                new_cursor: 10,
                new_selection: None,
            }
        );

        // After applying user_action1, text becomes "$sqrt(2 x + 1)$" and cursor is at 10 (before '+').
        let source2 = Source::detached("$sqrt(2 x + 1)$");
        let state2 = EditorState {
            text: source2.text(),
            cursor: 10,
            selection: None,
            context: EditorContext::Math,
        };
        let user_action2 = handle_key_event(&source2, &state2, "backspace", false);
        // Pressing backspace again deletes the single space separator at 9..10.
        assert_eq!(
            user_action2,
            EditorAction::Edit {
                range: 9..10,
                replacement: "".to_string(),
                new_cursor: 9,
                new_selection: None,
            }
        );

        // After applying user_action2, text becomes "$sqrt(2 x+ 1)$" and cursor is at 9 (directly after 'x').
        let source3 = Source::detached("$sqrt(2 x+ 1)$");
        let state3 = EditorState {
            text: source3.text(),
            cursor: 9,
            selection: None,
            context: EditorContext::Math,
        };
        let user_action3 = handle_key_event(&source3, &state3, "backspace", false);
        // Pressing backspace once more deletes the single-character MathText 'x' directly.
        assert_eq!(
            user_action3,
            EditorAction::Edit {
                range: 8..9,
                replacement: "".to_string(),
                new_cursor: 8,
                new_selection: None,
            }
        );
    }
}
