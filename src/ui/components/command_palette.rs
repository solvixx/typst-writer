use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::select::{SearchableVec, Select, SelectItem, SelectState};

pub struct CommandItem {
    pub name: String,
    pub action: Box<dyn Action>,
}

impl Clone for CommandItem {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            action: self.action.boxed_clone(),
        }
    }
}

impl SelectItem for CommandItem {
    type Value = CommandItem;

    fn title(&self) -> SharedString {
        SharedString::from(self.name.clone())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

impl PartialEq for CommandItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub struct CommandPalette {
    workspace: WeakEntity<EditorWorkspace>,
    select_state: Entity<SelectState<SearchableVec<CommandItem>>>,
}

impl CommandPalette {
    pub fn new(
        workspace: WeakEntity<EditorWorkspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let items = vec![
            CommandItem {
                name: "Export PDF".to_string(),
                action: Box::new(ExportPdf),
            },
            CommandItem {
                name: "Undo".to_string(),
                action: Box::new(Undo),
            },
            CommandItem {
                name: "Redo".to_string(),
                action: Box::new(Redo),
            },
        ];

        let select_state = cx.new(|cx| {
            SelectState::new(SearchableVec::new(items), None, window, cx).searchable(true)
        });

        cx.subscribe(&select_state, |this, _, event, cx| {
            if let gpui_component::select::SelectEvent::Confirm(Some(item)) = event {
                if let Some(_ws) = this.workspace.upgrade() {
                    cx.dispatch_action(item.action.as_ref());
                }
            }
        })
        .detach();

        Self {
            workspace,
            select_state,
        }
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_64()
            .child(Select::new(&self.select_state).placeholder("Search commands..."))
    }
}

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct ExportPdf;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct Undo;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct Redo;
