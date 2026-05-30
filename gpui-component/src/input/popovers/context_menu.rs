use gpui::{
    App, AppContext as _, Context, Corner, DismissEvent, Entity, IntoElement, MouseDownEvent,
    ParentElement as _, Pixels, Point, Render, Styled, Subscription, Window, anchored, deferred,
    div, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _,
    input::{self, InputState, popovers::ContextMenu},
    menu::PopupMenu,
};

/// Context menu for mouse right clicks.
pub(crate) struct MouseContextMenu {
    editor: Entity<InputState>,
    menu: Entity<PopupMenu>,
    mouse_position: Point<Pixels>,
    open: bool,

    _subscriptions: Vec<Subscription>,
}

impl InputState {
    pub(crate) fn handle_right_click_menu(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Show Mouse context menu
        if !self.selected_range.contains(offset) {
            self.move_to(offset, None, cx);
        }

        self.context_menu = Some(ContextMenu::MouseContext(self.mouse_context_menu.clone()));

        let is_enable = !self.disabled;
        let is_selected = !self.selected_range.is_empty();
        let has_paste = is_enable && cx.read_from_clipboard().is_some();

        let action_context = self.focus_handle.clone();
        self.mouse_context_menu.update(cx, |this, cx| {
            this.mouse_position = event.position;
            this.menu.update(cx, |menu, cx| {
                let new_menu = PopupMenu::new(cx)
                    .menu_with_enable(
                        t!("Input.Cut"),
                        Box::new(input::Cut),
                        is_enable && is_selected,
                    )
                    .menu_with_enable(t!("Input.Copy"), Box::new(input::Copy), is_selected)
                    .menu_with_enable(t!("Input.Paste"), Box::new(input::Paste), has_paste)
                    .separator()
                    .menu(t!("Input.Select All"), Box::new(input::SelectAll));

                menu.menu_items = new_menu.menu_items;
                menu.action_context = Some(action_context);
                cx.notify();
            });
            this.open = true;

            cx.notify();
        });
    }
}

impl MouseContextMenu {
    pub(crate) fn new(
        editor: Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let menu = cx.new(|cx| PopupMenu::new(cx).small());

            let _subscriptions = vec![cx.subscribe_in(&menu, window, {
                move |this: &mut Self, _, _: &DismissEvent, window, cx| {
                    this.close(window, cx);
                }
            })];

            Self {
                editor,
                menu,
                mouse_position: Point::default(),
                open: false,
                _subscriptions,
            }
        })
    }

    #[inline]
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    #[inline]
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.editor.update(cx, |this, cx| {
            this.focus(window, cx);
        });
    }
}

impl Render for MouseContextMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        deferred(
            anchored()
                .snap_to_window_with_margin(px(8.))
                .anchor(Corner::TopLeft)
                .position(self.mouse_position)
                .child(
                    div()
                        .font_family(cx.theme().font_family.clone())
                        .cursor_default()
                        .child(self.menu.clone()),
                ),
        )
        .into_any_element()
    }
}
