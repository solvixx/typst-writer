use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Selectable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RibbonTab {
    Home,
    Insert,
    Math,
    Developer,
}

pub struct Ribbon {
    pub active_tab: RibbonTab,
    pub workspace: WeakEntity<EditorWorkspace>,
}

impl Ribbon {
    pub fn new(workspace: WeakEntity<EditorWorkspace>) -> Self {
        Self {
            active_tab: RibbonTab::Home,
            workspace,
        }
    }

    pub fn set_tab(&mut self, tab: RibbonTab, cx: &mut Context<Self>) {
        if self.active_tab != tab {
            self.active_tab = tab;
            cx.notify();
        }
    }

    fn render_tab_selector(
        &self,
        tab: RibbonTab,
        label: &'static str,
        icon: IconName,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_tab == tab;

        let mut button = Button::new(SharedString::from(format!("tab_{}", label)))
            .label(label)
            .icon(icon)
            .compact()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_tab(tab, cx);
            }));

        if is_active {
            button = button.primary();
        } else {
            button = button.ghost();
        }

        button
    }

    fn render_action_button(
        &self,
        label: &'static str,
        action: &'static str,
        icon: Option<IconName>,
    ) -> impl IntoElement {
        let workspace = self.workspace.clone();

        // Use a compact button or ghost button
        let mut btn = Button::new(SharedString::from(format!("btn_{}", action)))
            .ghost()
            .compact()
            .tooltip(label);

        if let Some(icon_val) = icon {
            btn = btn.icon(icon_val);
        } else {
            btn = btn.label(label);
        }

        btn.on_click(move |_, window, cx| {
            if let Some(ws) = workspace.upgrade() {
                ws.update(cx, |this, cx| {
                    let current_text = this.world.source_ref().text();
                    let edit_pos = this.cursor_offset.min(current_text.len());
                    let selection = this.selection.filter(|s| {
                        !s.is_empty()
                            && s.start <= current_text.len()
                            && s.end <= current_text.len()
                    });

                    let action_obj = match action {
                        "bold" | "italic" => {
                            let is_bold = action == "bold";
                            let symbol = if is_bold { "*" } else { "_" };
                            let other_symbol = if is_bold { "_" } else { "*" };
                            let func_open = if is_bold { "#strong[" } else { "#emph[" };

                            let process_format = |sel_start: usize, sel_end: usize| {
                                let clamp_to_char_boundary = |pos: usize, text: &str| -> usize {
                                    let len = text.len();
                                    if pos >= len {
                                        return len;
                                    }
                                    let mut p = pos;
                                    while p > 0 && !text.is_char_boundary(p) {
                                        p -= 1;
                                    }
                                    p
                                };
                                let start = clamp_to_char_boundary(
                                    sel_start.min(sel_end).min(current_text.len()),
                                    &current_text,
                                );
                                let end = clamp_to_char_boundary(
                                    sel_start.max(sel_end).min(current_text.len()),
                                    &current_text,
                                );

                                // 1. Check Case A1: Wrapped by same shorthand inside selection
                                let has_shorthand_inside = (end - start >= 2)
                                    && current_text[start..].starts_with(symbol)
                                    && current_text[..end].ends_with(symbol);

                                // 2. Check Case A2: Wrapped by same function inside selection
                                let has_func_inside = (end - start >= func_open.len() + 1)
                                    && current_text[start..].starts_with(func_open)
                                    && current_text[..end].ends_with(']');

                                // 3. Check Case B1: Wrapped by same shorthand outside selection
                                let has_shorthand_outside = start >= 1
                                    && end + 1 <= current_text.len()
                                    && current_text[..start].ends_with(symbol)
                                    && current_text[end..].starts_with(symbol);

                                // 4. Check Case B2: Wrapped by same function outside selection
                                let has_func_outside = start >= func_open.len()
                                    && end + 1 <= current_text.len()
                                    && current_text[..start].ends_with(func_open)
                                    && current_text[end..].starts_with(']');

                                if has_shorthand_inside {
                                    let replacement = current_text[start + 1..end - 1].to_string();
                                    let new_sel = start..(end - 2);
                                    let new_cursor = end - 2;
                                    return crate::core::editor::EditorAction::Edit {
                                        range: start..end,
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    };
                                } else if has_func_inside {
                                    let replacement =
                                        current_text[start + func_open.len()..end - 1].to_string();
                                    let diff = func_open.len() + 1;
                                    let new_sel = start..(end - diff);
                                    let new_cursor = end - diff;
                                    return crate::core::editor::EditorAction::Edit {
                                        range: start..end,
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    };
                                } else if has_shorthand_outside {
                                    let replacement = current_text[start..end].to_string();
                                    let new_sel = (start - 1)..(end - 1);
                                    let new_cursor = end - 1;
                                    return crate::core::editor::EditorAction::Edit {
                                        range: (start - 1)..(end + 1),
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    };
                                } else if has_func_outside {
                                    let replacement = current_text[start..end].to_string();
                                    let diff = func_open.len();
                                    let new_sel = (start - diff)..(end - diff);
                                    let new_cursor = end - diff;
                                    return crate::core::editor::EditorAction::Edit {
                                        range: (start - diff)..(end + 1),
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    };
                                }

                                // 5. Wrap or Split Overlaps
                                let selected_text = &current_text[start..end];

                                // Split around other formatting boundaries if present inside the selection
                                if selected_text.contains(other_symbol) {
                                    let mut replacement = String::new();
                                    let mut last_idx = 0;

                                    for (idx, _) in selected_text.match_indices(other_symbol) {
                                        let segment = &selected_text[last_idx..idx];
                                        if !segment.is_empty() {
                                            replacement.push_str(symbol);
                                            replacement.push_str(segment);
                                            replacement.push_str(symbol);
                                        }
                                        replacement.push_str(other_symbol);
                                        last_idx = idx + other_symbol.len();
                                    }

                                    let last_segment = &selected_text[last_idx..];
                                    if !last_segment.is_empty() {
                                        replacement.push_str(symbol);
                                        replacement.push_str(last_segment);
                                        replacement.push_str(symbol);
                                    }

                                    let new_len = replacement.len();
                                    let new_sel = start..(start + new_len);
                                    let new_cursor = start + new_len;
                                    return crate::core::editor::EditorAction::Edit {
                                        range: start..end,
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    };
                                }

                                // Evaluate boundary characters to check for letter adjacency (Solution A)
                                let left_char = if start > 0 {
                                    current_text[..start].chars().next_back()
                                } else {
                                    None
                                };
                                let right_char = if end < current_text.len() {
                                    current_text[end..].chars().next()
                                } else {
                                    None
                                };

                                let left_is_alphanumeric =
                                    left_char.map_or(false, |c| c.is_alphanumeric());
                                let right_is_alphanumeric =
                                    right_char.map_or(false, |c| c.is_alphanumeric());

                                if left_is_alphanumeric || right_is_alphanumeric {
                                    // Use high-fidelity function syntax fallback
                                    let replacement = format!("{}{}]", func_open, selected_text);
                                    let new_len = replacement.len();
                                    let new_sel = (start + func_open.len())..(start + new_len - 1);
                                    let new_cursor = start + new_len;
                                    crate::core::editor::EditorAction::Edit {
                                        range: start..end,
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    }
                                } else {
                                    // Use standard shorthand
                                    let replacement =
                                        format!("{}{}{}", symbol, selected_text, symbol);
                                    let new_len = replacement.len();
                                    let new_sel = (start + 1)..(start + new_len - 1);
                                    let new_cursor = start + new_len;
                                    crate::core::editor::EditorAction::Edit {
                                        range: start..end,
                                        replacement,
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    }
                                }
                            };

                            if let Some(sel) = selection {
                                process_format(sel.start, sel.end)
                            } else {
                                // No selection: find word boundary
                                let mut word_start = edit_pos;
                                while word_start > 0 {
                                    let prev_char =
                                        current_text[..word_start].chars().next_back().unwrap();
                                    if prev_char.is_alphanumeric() {
                                        word_start -= prev_char.len_utf8();
                                    } else {
                                        break;
                                    }
                                }

                                let mut word_end = edit_pos;
                                while word_end < current_text.len() {
                                    let next_char =
                                        current_text[word_end..].chars().next().unwrap();
                                    if next_char.is_alphanumeric() {
                                        word_end += next_char.len_utf8();
                                    } else {
                                        break;
                                    }
                                }

                                if word_start < word_end {
                                    process_format(word_start, word_end)
                                } else {
                                    // Fallback space-padded placeholder insertion (Solution C)
                                    let placeholder = if is_bold {
                                        " *bold text* "
                                    } else {
                                        " _italic text_ "
                                    };
                                    let cur_rel = if is_bold { 11 } else { 13 };
                                    let start_offset = 2;
                                    let len_offset = if is_bold { 9 } else { 11 };
                                    let new_sel = (edit_pos + start_offset)
                                        ..(edit_pos + start_offset + len_offset);
                                    let new_cursor = edit_pos + cur_rel;
                                    crate::core::editor::EditorAction::Edit {
                                        range: edit_pos..edit_pos,
                                        replacement: placeholder.to_string(),
                                        new_cursor,
                                        new_selection: Some(new_sel),
                                    }
                                }
                            }
                        }
                        _ => {
                            let (insert_str, cursor_rel) = match action {
                                // Markup Actions
                                "heading" => ("\n= New Section\n", 14),
                                "list" => ("\n- List Item\n", 13),
                                "math_formula" => (" $? = ?_?$ ", 2),

                                // Math Actions
                                "math_frac" => (" (? / ?) ", 2),
                                "math_super" => (" ?^? ", 1),
                                "math_sub" => (" ?_? ", 1),
                                "math_root" => (" sqrt(?) ", 6),
                                "math_mat" => (" mat(?, ?; ?, ?) ", 5),
                                "math_sum" => (" sum_(?)^(?) ", 6),
                                "math_int" => (" integral_(?)^(?) ", 10),

                                // Greek Alphabet
                                "math_alpha" => (" alpha ", 6),
                                "math_beta" => (" beta ", 5),
                                "math_gamma" => (" gamma ", 6),
                                "math_theta" => (" theta ", 6),
                                "math_pi" => (" pi ", 3),
                                "math_omega" => (" omega ", 6),
                                "math_exit" => (" ", 1),

                                // Code Actions
                                "code_let" => ("\n#let x = 10\n", 12),
                                "code_set" => ("\n#set text(fill: red)\n", 21),
                                "code_show" => ("\n#show heading: emph\n", 21),
                                "code_exit" => (" ", 1),

                                _ => ("", 0),
                            };
                            crate::core::editor::EditorAction::Edit {
                                range: edit_pos..edit_pos,
                                replacement: insert_str.to_string(),
                                new_cursor: edit_pos + cursor_rel,
                                new_selection: None,
                            }
                        }
                    };

                    this.apply_editor_action(action_obj, window, cx);
                });
            }
        })
    }

    fn render_group(
        &self,
        name: &'static str,
        items: Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .items_center()
            .justify_between()
            .child(h_flex().gap_1().children(items))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(name),
            )
    }
}

impl Render for Ribbon {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_compiling = self
            .workspace
            .upgrade()
            .map(|ws| ws.read(cx).compiler.is_compiling)
            .unwrap_or(false);

        let mut right_actions = h_flex().gap_2().items_center();

        // Fixed-width container for the spinner to prevent any layout shifts!
        let mut spinner_container = div()
            .w(px(20.))
            .h(px(20.))
            .flex()
            .items_center()
            .justify_center();

        if is_compiling {
            spinner_container = spinner_container.child(gpui_component::spinner::Spinner::new());
        }

        right_actions = right_actions.child(spinner_container);

        if let Some(ws_handle) = self.workspace.upgrade() {
            let ws = ws_handle.read(cx);
            let sidebar_visible = ws.config.sidebar_visible;
            let logs_visible = ws._show_logs;
            let source_code_visible = ws.config.source_code_visible;

            right_actions = right_actions.child(
                Button::new("save-file")
                    .icon(IconName::Save)
                    .ghost()
                    .compact()
                    .tooltip("Save")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this.save_file(window, cx);
                            });
                        }
                    }),
            );

            right_actions = right_actions.child(
                Button::new("save-file-as")
                    .label("Save As")
                    .icon(IconName::Save)
                    .ghost()
                    .compact()
                    .tooltip("Save As")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this.save_file_as(window, cx);
                            });
                        }
                    }),
            );

            right_actions = right_actions.child(
                Button::new("toggle-sidebar")
                    .icon(IconName::PanelLeft)
                    .ghost()
                    .compact()
                    .selected(sidebar_visible)
                    .tooltip("Toggle Sidebar")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this.config.sidebar_visible = !this.config.sidebar_visible;
                                crate::core::config::ConfigManager::save(&this.config);
                                this.update_panel_visibility(window, cx);
                                cx.notify();
                            });
                        }
                    }),
            );

            right_actions = right_actions.child(
                Button::new("toggle-source")
                    .icon(IconName::Code)
                    .ghost()
                    .compact()
                    .selected(source_code_visible)
                    .tooltip("Toggle Source Code")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this.config.source_code_visible = !this.config.source_code_visible;
                                crate::core::config::ConfigManager::save(&this.config);
                                this.update_panel_visibility(window, cx);
                                cx.notify();
                            });
                        }
                    }),
            );

            right_actions = right_actions.child(
                Button::new("toggle-preview")
                    .icon(IconName::Eye)
                    .ghost()
                    .compact()
                    .selected(ws.config.preview_panel_visible)
                    .tooltip("Toggle Preview")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this.config.preview_panel_visible =
                                    !this.config.preview_panel_visible;
                                crate::core::config::ConfigManager::save(&this.config);
                                this.update_panel_visibility(window, cx);
                                cx.notify();
                            });
                        }
                    }),
            );

            right_actions = right_actions.child(
                Button::new("toggle-logs")
                    .icon(IconName::LayoutDashboard)
                    .ghost()
                    .compact()
                    .selected(logs_visible)
                    .tooltip("Toggle Logs")
                    .on_click({
                        let ws_handle = ws_handle.clone();
                        move |_, window, cx| {
                            ws_handle.update(cx, |this, cx| {
                                this._show_logs = !this._show_logs;
                                this.config.log_panel_visible = this._show_logs;
                                crate::core::config::ConfigManager::save(&this.config);
                                this.update_panel_visibility(window, cx);
                                cx.notify();
                            });
                        }
                    }),
            );
        }

        right_actions = right_actions.child(
            Button::new("export-pdf")
                .label("Export PDF")
                .icon(IconName::Download)
                .compact()
                .on_click({
                    let workspace = self.workspace.clone();
                    move |_, window, cx| {
                        if let Some(ws) = workspace.upgrade() {
                            ws.update(cx, |this, cx| this.export_pdf(window, cx));
                        }
                    }
                }),
        );

        #[cfg(debug_assertions)]
        if let Some(ws) = self.workspace.upgrade() {
            let show = ws.read(cx).show_glyph_boxes;
            right_actions = right_actions.child(
                Button::new("toggle-glyph-boxes")
                    .label("Debug Boxes")
                    .icon(IconName::Bug)
                    .selected(show)
                    .compact()
                    .on_click(move |_, _, cx| {
                        ws.update(cx, |this, cx| {
                            this.show_glyph_boxes = !this.show_glyph_boxes;
                            cx.notify();
                        });
                    }),
            );
        }

        let tabs = h_flex()
            .gap_1()
            .px_2()
            .py_1()
            .items_center()
            .w_full()
            .child(self.render_tab_selector(RibbonTab::Home, "Home", IconName::Home, cx))
            .child(div().w_px().h_4().bg(cx.theme().border))
            .child(self.render_tab_selector(RibbonTab::Insert, "Insert", IconName::PlusSquare, cx))
            .child(div().w_px().h_4().bg(cx.theme().border))
            .child(self.render_tab_selector(RibbonTab::Math, "Math", IconName::Function, cx))
            .child(div().w_px().h_4().bg(cx.theme().border))
            .child(self.render_tab_selector(RibbonTab::Developer, "Developer", IconName::Code, cx))
            .child(
                h_flex()
                    .ml_auto()
                    .gap_2()
                    .items_center()
                    .child(right_actions),
            );

        let content = match self.active_tab {
            RibbonTab::Home => h_flex()
                .gap_4()
                .child(
                    self.render_group(
                        "Font",
                        vec![
                            self.render_action_button("Bold", "bold", Some(IconName::Bold))
                                .into_any_element(),
                            self.render_action_button("Italic", "italic", Some(IconName::Italic))
                                .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .child(div().w_px().h_full().bg(cx.theme().border))
                .child(
                    self.render_group(
                        "Paragraph",
                        vec![
                            self.render_action_button(
                                "Heading",
                                "heading",
                                Some(IconName::Heading),
                            )
                            .into_any_element(),
                            self.render_action_button("List", "list", Some(IconName::List))
                                .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .into_any_element(),
            RibbonTab::Insert => h_flex()
                .gap_4()
                .child(
                    self.render_group(
                        "Symbols",
                        vec![
                            self.render_action_button(
                                "Formula",
                                "math_formula",
                                Some(IconName::Function),
                            )
                            .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .into_any_element(),
            RibbonTab::Math => h_flex()
                .gap_4()
                .child(
                    self.render_group(
                        "Structures",
                        vec![
                            self.render_action_button(
                                "Fraction",
                                "math_frac",
                                Some(IconName::Fraction),
                            )
                            .into_any_element(),
                            self.render_action_button(
                                "Power",
                                "math_super",
                                Some(IconName::Superscript),
                            )
                            .into_any_element(),
                            self.render_action_button(
                                "Subscript",
                                "math_sub",
                                Some(IconName::Subscript),
                            )
                            .into_any_element(),
                            self.render_action_button("Root", "math_root", Some(IconName::Sqrt))
                                .into_any_element(),
                            self.render_action_button("Matrix", "math_mat", Some(IconName::Matrix))
                                .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .child(div().w_px().h_full().bg(cx.theme().border))
                .child(
                    self.render_group(
                        "Calculus",
                        vec![
                            self.render_action_button("Sum", "math_sum", Some(IconName::Sigma))
                                .into_any_element(),
                            self.render_action_button(
                                "Integral",
                                "math_int",
                                Some(IconName::Integral),
                            )
                            .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .child(div().w_px().h_full().bg(cx.theme().border))
                .child(
                    self.render_group(
                        "Symbols",
                        vec![
                            self.render_action_button("α", "math_alpha", Some(IconName::Alpha))
                                .into_any_element(),
                            self.render_action_button("β", "math_beta", Some(IconName::Beta))
                                .into_any_element(),
                            self.render_action_button("γ", "math_gamma", Some(IconName::Gamma))
                                .into_any_element(),
                            self.render_action_button("θ", "math_theta", Some(IconName::Theta))
                                .into_any_element(),
                            self.render_action_button("ω", "math_omega", Some(IconName::Omega))
                                .into_any_element(),
                            self.render_action_button("π", "math_pi", Some(IconName::Pi))
                                .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .into_any_element(),
            RibbonTab::Developer => h_flex()
                .gap_4()
                .child(
                    self.render_group(
                        "Bindings",
                        vec![
                            self.render_action_button("Let", "code_let", Some(IconName::Variable))
                                .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .child(div().w_px().h_full().bg(cx.theme().border))
                .child(
                    self.render_group(
                        "Rules",
                        vec![
                            self.render_action_button(
                                "Set Rule",
                                "code_set",
                                Some(IconName::Settings2),
                            )
                            .into_any_element(),
                            self.render_action_button(
                                "Show Rule",
                                "code_show",
                                Some(IconName::Eye),
                            )
                            .into_any_element(),
                        ],
                        cx,
                    ),
                )
                .child(div().w_px().h_full().bg(cx.theme().border))
                .child(self.render_group(
                    "Config",
                    vec![
                        Button::new("ribbon-preferences")
                            .icon(IconName::Settings)
                            .ghost()
                            .compact()
                            .tooltip("Preferences")
                            .on_click({
                                let workspace = self.workspace.clone();
                                move |_, window, cx| {
                                    if let Some(ws) = workspace.upgrade() {
                                        ws.update(cx, |this, cx| this.open_preferences(window, cx));
                                    }
                                }
                            })
                            .into_any_element()
                    ],
                    cx,
                ))
                .into_any_element(),
        };

        v_flex()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(tabs)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .bg(cx.theme().secondary) // slightly different background for the ribbon body
                    .child(content),
            )
    }
}
