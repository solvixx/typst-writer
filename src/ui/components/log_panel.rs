use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::StyledExt;
use gpui_component::button::Button;
use gpui_component::scroll::ScrollableElement;
use gpui_component::tooltip::Tooltip;
use gpui_component::v_flex;
use gpui_component::{Selectable, Sizable};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterLevel {
    All,
    Info,
    Warn,
    Error,
    #[cfg(debug_assertions)]
    Debug,
}

pub struct LogPanel {
    workspace: WeakEntity<EditorWorkspace>,
    focus_handle: FocusHandle,
    filter: FilterLevel,
}

impl LogPanel {
    pub fn new(workspace: WeakEntity<EditorWorkspace>, cx: &mut Context<Self>) -> Self {
        if let Some(ws) = workspace.upgrade() {
            cx.observe(&ws, |_this, _, cx| {
                cx.notify();
            })
            .detach();
        }
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            filter: FilterLevel::All,
        }
    }

    fn render_filter_button(
        &self,
        level: FilterLevel,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.filter == level;

        Button::new(label)
            .label(label)
            .selected(is_selected)
            .small()
            .compact()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.filter = level;
                cx.notify();
            }))
    }
}

impl Render for LogPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let logs = if let Some(ws) = self.workspace.upgrade() {
            ws.read(cx).logs.clone()
        } else {
            Vec::new()
        };

        let filtered_logs: Vec<(String, String, String)> = logs
            .into_iter()
            .filter(|(_, level, _)| match self.filter {
                FilterLevel::All => true,
                FilterLevel::Info => level == "INFO",
                FilterLevel::Warn => level == "WARN",
                FilterLevel::Error => level == "ERROR",
                #[cfg(debug_assertions)]
                FilterLevel::Debug => level == "DEBUG",
            })
            .collect();

        div()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .child(
                // Header / Toolbar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().tab_bar)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_x_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child("CONSOLE LOGS"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_x_1()
                                    .child(self.render_filter_button(FilterLevel::All, "All", cx))
                                    .child(self.render_filter_button(FilterLevel::Info, "Info", cx))
                                    .child(self.render_filter_button(FilterLevel::Warn, "Warn", cx))
                                    .child(self.render_filter_button(
                                        FilterLevel::Error,
                                        "Error",
                                        cx,
                                    ))
                                    .children({
                                        #[allow(unused_mut)]
                                        let mut items = Vec::<AnyElement>::new();
                                        #[cfg(debug_assertions)]
                                        {
                                            items.push(
                                                self.render_filter_button(
                                                    FilterLevel::Debug,
                                                    "Debug",
                                                    cx,
                                                )
                                                .into_any_element(),
                                            );
                                        }
                                        items
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_x_2()
                            .child(
                                Button::new("copy-all")
                                    .label("Copy All")
                                    .small()
                                    .compact()
                                    .on_click(cx.listener({
                                        let filtered_logs = filtered_logs.clone();
                                        move |_, _, _, cx| {
                                            let full_text = filtered_logs
                                                .iter()
                                                .map(|(ts, level, msg)| {
                                                    format!("[{}] [{}] {}", ts, level, msg)
                                                })
                                                .collect::<Vec<String>>()
                                                .join("\n");
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                full_text,
                                            ));
                                        }
                                    })),
                            )
                            .child(
                                Button::new("clear")
                                    .label("Clear")
                                    .small()
                                    .compact()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(ws) = this.workspace.upgrade() {
                                            ws.update(cx, |ws, cx| {
                                                ws.logs.clear();
                                                cx.notify();
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // Logs Content
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .px_3()
                    .child(v_flex().py_2().gap_1().children(
                        filtered_logs.into_iter().rev().enumerate().map(
                            |(idx, (ts, level, msg))| {
                                let badge_color = match level.as_str() {
                                    "ERROR" => cx.theme().danger,
                                    "WARN" => cx.theme().warning,
                                    "INFO" => cx.theme().info,
                                    "DEBUG" => cx.theme().muted_foreground,
                                    _ => cx.theme().foreground,
                                };
                                let badge_bg = match level.as_str() {
                                    "ERROR" => cx.theme().danger.opacity(0.12),
                                    "WARN" => cx.theme().warning.opacity(0.12),
                                    "INFO" => cx.theme().info.opacity(0.12),
                                    "DEBUG" => cx.theme().muted.opacity(0.12),
                                    _ => cx.theme().secondary,
                                };
                                let badge_border = match level.as_str() {
                                    "ERROR" => cx.theme().danger.opacity(0.35),
                                    "WARN" => cx.theme().warning.opacity(0.35),
                                    "INFO" => cx.theme().info.opacity(0.35),
                                    "DEBUG" => cx.theme().muted.opacity(0.35),
                                    _ => cx.theme().border,
                                };

                                div()
                                    .id(SharedString::from(format!("log-row-{}", idx)))
                                    .flex()
                                    .items_center()
                                    .gap_x_3()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_xs()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(gpui::transparent_black())
                                    .hover(|style| {
                                        style
                                            .bg(cx.theme().list_hover)
                                            .border_color(cx.theme().border)
                                    })
                                    .cursor_pointer()
                                    .tooltip(move |window, cx| {
                                        Tooltip::new("Click to copy log line").build(window, cx)
                                    })
                                    .on_mouse_down(MouseButton::Left, {
                                        let level = level.clone();
                                        let msg = msg.clone();
                                        let ts = ts.clone();
                                        cx.listener(move |_, _, _, cx| {
                                            let log_line = format!("[{}] [{}] {}", ts, level, msg);
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                log_line,
                                            ));
                                        })
                                    })
                                    .child(
                                        // Timestamp
                                        div()
                                            .flex_none()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("[{}]", ts)),
                                    )
                                    .child(
                                        // Badge
                                        div()
                                            .flex_none()
                                            .w(px(56.0))
                                            .h(px(20.0))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(badge_border)
                                            .bg(badge_bg)
                                            .font_semibold()
                                            .text_xs()
                                            .text_color(badge_color)
                                            .child(level),
                                    )
                                    .child(
                                        div().flex_1().text_color(cx.theme().foreground).child(msg),
                                    )
                            },
                        ),
                    )),
            )
    }
}

impl Focusable for LogPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui_component::dock::Panel for LogPanel {
    fn panel_name(&self) -> &'static str {
        "LogPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Logs"
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for LogPanel {}
