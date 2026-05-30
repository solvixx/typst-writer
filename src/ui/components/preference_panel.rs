use crate::core::config::{AppConfig, ConfigManager};
use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::divider::Divider;
use gpui_component::switch::Switch;
use gpui_component::{ActiveTheme, Selectable, Sizable, StyledExt, h_flex, v_flex};

pub struct PreferencePanel {
    workspace: WeakEntity<EditorWorkspace>,
    focus_handle: FocusHandle,
}

impl PreferencePanel {
    pub fn new(workspace: WeakEntity<EditorWorkspace>, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for PreferencePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace_handle = self.workspace.clone();

        let ws = if let Some(ws) = self.workspace.upgrade() {
            ws.read(cx).config.clone()
        } else {
            AppConfig::default()
        };

        let config_path_str = ConfigManager::config_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown config path".to_string());

        let config_path_for_copy = config_path_str.clone();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .p_6()
            .gap_6()
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(cx.theme().foreground)
                            .child("Preferences")
                    )
            )
            .child(Divider::horizontal())
            // CONFIG FILE CARD
            .child(
                v_flex()
                    .p_4()
                    .rounded_md()
                    .bg(cx.theme().secondary)
                    .border_1()
                    .border_color(cx.theme().border)
                    .gap_3()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("CONFIGURATION FILE PATH")
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_color(cx.theme().foreground)
                                    .child(config_path_str)
                            )
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("copy-config-path")
                                    .ghost()
                                    .small()
                                    .child("Copy Path")
                                    .on_click(move |_, _window, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(config_path_for_copy.clone()));
                                    })
                            )
                    )
            )
            // SETTINGS ROWS
            .child(
                v_flex()
                    .gap_4()
                    // THEME COLOR
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .child(div().text_sm().font_semibold().text_color(cx.theme().foreground).child("Theme Color"))
                                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Switch application UI theme mode"))
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("theme-dark")
                                            .outline()
                                            .selected(ws.theme == "dark")
                                            .child("Dark")
                                            .on_click({
                                                let workspace_handle = workspace_handle.clone();
                                                move |_, window, cx| {
                                                    if let Some(ws) = workspace_handle.upgrade() {
                                                        ws.update(cx, |this, cx| {
                                                            this.config.theme = "dark".to_string();
                                                            ConfigManager::save(&this.config);
                                                            gpui_component::theme::Theme::change(gpui_component::theme::ThemeMode::Dark, Some(window), cx);
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            })
                                    )
                                    .child(
                                        Button::new("theme-light")
                                            .outline()
                                            .selected(ws.theme == "light")
                                            .child("Light")
                                            .on_click({
                                                let workspace_handle = workspace_handle.clone();
                                                move |_, window, cx| {
                                                    if let Some(ws) = workspace_handle.upgrade() {
                                                        ws.update(cx, |this, cx| {
                                                            this.config.theme = "light".to_string();
                                                            ConfigManager::save(&this.config);
                                                            gpui_component::theme::Theme::change(gpui_component::theme::ThemeMode::Light, Some(window), cx);
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            })
                                    )
                            )
                    )
                    .child(Divider::horizontal())
                    // AUTO COMPILE
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .child(div().text_sm().font_semibold().text_color(cx.theme().foreground).child("Auto Compile"))
                                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Automatically recompile PDF on every keystroke"))
                            )
                            .child(
                                Switch::new("auto-compile-switch")
                                    .checked(ws.auto_compile)
                                    .on_click({
                                        let workspace_handle = workspace_handle.clone();
                                        move |checked, _window, cx| {
                                            if let Some(ws) = workspace_handle.upgrade() {
                                                ws.update(cx, |this, cx| {
                                                    this.config.auto_compile = *checked;
                                                    ConfigManager::save(&this.config);
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                            )
                    )
                    .child(Divider::horizontal())
                    // EDITOR FONT SIZE
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .child(div().text_sm().font_semibold().text_color(cx.theme().foreground).child("Editor Font Size"))
                                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Adjust the font size used in the source code editors"))
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        Button::new("dec-font-size")
                                            .outline()
                                            .compact()
                                            .child("-")
                                            .on_click({
                                                let workspace_handle = workspace_handle.clone();
                                                let ws_size = ws.font_size;
                                                move |_, _window, cx| {
                                                    if let Some(ws) = workspace_handle.upgrade() {
                                                        ws.update(cx, |this, cx| {
                                                            this.config.font_size = (ws_size - 1.0).max(8.0);
                                                            ConfigManager::save(&this.config);
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            })
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().foreground)
                                            .child(format!("{:.1} px", ws.font_size))
                                    )
                                    .child(
                                        Button::new("inc-font-size")
                                            .outline()
                                            .compact()
                                            .child("+")
                                            .on_click({
                                                let workspace_handle = workspace_handle.clone();
                                                let ws_size = ws.font_size;
                                                move |_, _window, cx| {
                                                    if let Some(ws) = workspace_handle.upgrade() {
                                                        ws.update(cx, |this, cx| {
                                                            this.config.font_size = (ws_size + 1.0).min(36.0);
                                                            ConfigManager::save(&this.config);
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            })
                                    )
                            )
                    )
                    .child(Divider::horizontal())
                    // FONTS PREVIEW CARD
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().font_semibold().text_color(cx.theme().foreground).child("Font Families"))
                            .child(
                                v_flex()
                                    .p_3()
                                    .rounded_md()
                                    .bg(cx.theme().secondary)
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("UI Font"))
                                            .child(div().text_xs().font_bold().text_color(cx.theme().foreground).child(ws.ui_font))
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Monospace Font"))
                                            .child(div().text_xs().font_bold().font_family(cx.theme().mono_font_family.clone()).text_color(cx.theme().foreground).child(ws.mono_font))
                                    )
                            )
                    )
            )
    }
}

impl Focusable for PreferencePanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui_component::dock::Panel for PreferencePanel {
    fn panel_name(&self) -> &'static str {
        "PreferencePanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Preferences"
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for PreferencePanel {}
