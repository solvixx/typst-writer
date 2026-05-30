use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::Sizable;
use gpui_component::WindowExt;
use gpui_component::h_flex;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::list::ListItem;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::switch::Switch;
use gpui_component::tree::{Tree, TreeItem, TreeState};
use lsp_types::Uri;
use std::path::{Path, PathBuf};
use typst::syntax::FileId;

// Define high-fidelity action types for VS Code-like explorer shortcuts and menu entries
gpui::actions!(
    file_tree,
    [NewFile, NewFolder, Delete, Rename, Duplicate, Open]
);

pub struct FileTree {
    _workspace: WeakEntity<EditorWorkspace>,
    tree_state: Entity<TreeState>,
    focus_handle: FocusHandle,
    root: PathBuf,
    editing_path: Option<String>,
    editing_input: Option<Entity<InputState>>,
    _rename_subscription: Option<Subscription>,
}

impl FileTree {
    pub fn new(
        workspace: WeakEntity<EditorWorkspace>,
        root: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let tree_state = cx.new(|cx| {
            // TRULY LAZY: Only load the immediate root level.
            let items = Self::load_level(&root);
            TreeState::new(cx).items(items)
        });

        // Observe tree_state to automatically notify and re-render FileTree upon changes
        cx.observe(&tree_state, |_, _, cx| {
            cx.notify();
        })
        .detach();

        // Register high-fidelity file tree hotkeys for active Tree focus context
        cx.bind_keys([
            KeyBinding::new("a", NewFile, Some("Tree")),
            KeyBinding::new("shift-a", NewFolder, Some("Tree")),
            KeyBinding::new("delete", Delete, Some("Tree")),
            KeyBinding::new("f2", Rename, Some("Tree")),
            KeyBinding::new("ctrl-d", Duplicate, Some("Tree")),
            KeyBinding::new("enter", Open, Some("Tree")),
        ]);

        Self {
            _workspace: workspace,
            tree_state,
            focus_handle,
            root,
            editing_path: None,
            editing_input: None,
            _rename_subscription: None,
        }
    }

    pub fn refresh(tree_state: &Entity<TreeState>, root: &Path, cx: &mut App) {
        let items = Self::load_level(root);
        tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
        });
    }

    pub fn set_root(&mut self, new_root: PathBuf, cx: &mut Context<Self>) {
        self.root = new_root.clone();
        Self::refresh(&self.tree_state, &new_root, cx);
        cx.notify();
    }

    /// Load the directory recursively.
    fn load_level(path: &Path) -> Vec<TreeItem> {
        let mut items = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries: Vec<_> = entries.flatten().collect();
            // Sort: Directories first, then alphabetical
            entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

            for entry in entries {
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // CRITICAL: Skip HIDDEN and MASSIVE directories
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                let id = path.to_string_lossy().to_string();
                let mut item = TreeItem::new(id, name);

                if path.is_dir() {
                    let children = Self::load_level(&path);
                    for child in children {
                        item = item.child(child);
                    }
                    items.push(item);
                } else {
                    items.push(item);
                }
            }
        }
        items
    }

    // --- Action Handlers ---

    fn handle_new_file(&self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_entry();
        let (is_folder, path_str) = selected
            .map(|e| (e.is_folder(), e.item().id.to_string()))
            .unwrap_or((true, self.root.to_string_lossy().to_string()));

        let parent = if is_folder {
            Path::new(&path_str).to_path_buf()
        } else {
            Path::new(&path_str)
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        };

        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value("Untitled.typ", window, cx);
            state.focus(window, cx);
            state
        });

        let parent_clone = parent.clone();
        let tree_state = self.tree_state.clone();
        let root = self.root.clone();
        let workspace = self._workspace.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("New File")
                .confirm()
                .on_ok({
                    let input_state = input_state.clone();
                    let parent = parent_clone.clone();
                    let tree_state = tree_state.clone();
                    let root = root.clone();
                    let workspace = workspace.clone();
                    move |_, window, cx| {
                        let name = input_state.read(cx).text().to_string().trim().to_string();
                        if !name.is_empty() {
                            let mut p = parent.join(name);
                            if p.extension().is_none() {
                                p = p.with_extension("typ");
                            }
                            if std::fs::write(&p, "").is_ok() {
                                Self::refresh(&tree_state, &root, cx);
                                if let Some(workspace) = workspace.upgrade() {
                                    workspace.update(cx, |ws, cx| {
                                        ws.open_file(&p.to_string_lossy(), window, cx);
                                    });
                                }
                            }
                        }
                        true
                    }
                })
                .child(
                    div()
                        .w_full()
                        .py_2()
                        .child(gpui_component::input::Input::new(&input_state)),
                )
        });
    }

    fn handle_new_folder(&self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_entry();
        let (is_folder, path_str) = selected
            .map(|e| (e.is_folder(), e.item().id.to_string()))
            .unwrap_or((true, self.root.to_string_lossy().to_string()));

        let parent = if is_folder {
            Path::new(&path_str).to_path_buf()
        } else {
            Path::new(&path_str)
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        };

        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value("New_Folder", window, cx);
            state.focus(window, cx);
            state
        });

        let parent_clone = parent.clone();
        let tree_state = self.tree_state.clone();
        let root = self.root.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("New Folder")
                .confirm()
                .on_ok({
                    let input_state = input_state.clone();
                    let parent = parent_clone.clone();
                    let tree_state = tree_state.clone();
                    let root = root.clone();
                    move |_, _, cx| {
                        let name = input_state.read(cx).text().to_string().trim().to_string();
                        if !name.is_empty() {
                            let p = parent.join(name);
                            let _ = std::fs::create_dir_all(&p);
                            Self::refresh(&tree_state, &root, cx);
                        }
                        true
                    }
                })
                .child(
                    div()
                        .w_full()
                        .py_2()
                        .child(gpui_component::input::Input::new(&input_state)),
                )
        });
    }

    fn handle_delete(&self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_entry();
        if let Some(entry) = selected {
            let path_str = entry.item().id.to_string();
            let p = Path::new(&path_str).to_path_buf();
            let file_name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let tree_state = self.tree_state.clone();
            let root = self.root.clone();
            let workspace = self._workspace.clone();

            window.open_dialog(cx, move |dialog, _window, _cx| {
                let file_name_str = file_name.clone();
                dialog
                    .title("Delete")
                    .confirm()
                    .on_ok({
                        let path_str = path_str.clone();
                        let p = p.clone();
                        let tree_state = tree_state.clone();
                        let root = root.clone();
                        let workspace = workspace.clone();
                        move |_, window, cx| {
                            let success = if p.is_dir() {
                                std::fs::remove_dir_all(&path_str).is_ok()
                            } else {
                                std::fs::remove_file(&path_str).is_ok()
                            };

                            if success {
                                Self::refresh(&tree_state, &root, cx);

                                if let Some(workspace) = workspace.upgrade() {
                                    workspace.update(cx, |ws, cx| {
                                        let is_active = ws
                                            .world
                                            .root_path
                                            .as_ref()
                                            .and_then(|r| ws.world.main_id.vpath().resolve(r))
                                            .map(|path| path == p || path.starts_with(&p))
                                            .unwrap_or(false);

                                        if is_active {
                                            let main_id = ws.world.main_id;
                                            ws.world.main_source =
                                                typst::syntax::Source::new(main_id, "".to_string());
                                            if let Some(editor) = ws.editors.get(&p) {
                                                editor.update(cx, |editor, cx| {
                                                    editor.input.update(cx, |input, cx| {
                                                        input.set_value("", window, cx);
                                                    });
                                                });
                                            }
                                            ws.compile(cx);
                                        }

                                        // Always remove from open editors if present
                                        ws.editors.remove(&p);
                                        if ws.active_editor_path.as_ref() == Some(&p) {
                                            ws.active_editor_path = None;
                                        }
                                    });
                                }
                            }
                            true
                        }
                    })
                    .child(div().py_2().text_sm().child(format!(
                        "Are you sure you want to delete '{}'?",
                        file_name_str
                    )))
            });
        }
    }

    fn handle_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_entry();
        if let Some(entry) = selected {
            let path_str = entry.item().id.to_string();
            let p = Path::new(&path_str).to_path_buf();
            let file_name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let input_state = cx.new(|cx| {
                let mut state = InputState::new(window, cx);
                state.set_value(&file_name, window, cx);
                let stem = p
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let stem_len = stem.len();
                state.set_selected_range(
                    gpui_component::input::Selection::new(0, stem_len),
                    false,
                    cx,
                );
                state.focus(window, cx);
                state
            });

            let old_path = p.clone();
            let workspace = self._workspace.clone();

            let this_weak = cx.weak_entity();
            let sub = window.subscribe(&input_state, cx, {
                let this_weak = this_weak.clone();
                let file_name = file_name.clone();
                move |_, event: &InputEvent, _window, cx| {
                    if let Some(this) = this_weak.upgrade() {
                        this.update(cx, |this, cx| {
                            match event {
                                InputEvent::PressEnter { .. } => {
                                    let new_name = this.editing_input.as_ref().unwrap().read(cx).text().to_string().trim().to_string();
                                    if !new_name.is_empty() && new_name != file_name
                                        && let Some(parent) = old_path.parent() {
                                            let new_path = parent.join(&new_name);
                                            if std::fs::rename(&old_path, &new_path).is_ok() {
                                                Self::refresh(&this.tree_state, &this.root, cx);
                                                
                                                if let Some(workspace) = workspace.upgrade() {
                                                    workspace.update(cx, |ws, cx| {
                                                        // 1. Update the editor map and the editor's path/URI if it is currently open
                                                        if let Some(editor_view) = ws.editors.remove(&old_path) {
                                                            let new_path_clone = new_path.clone();
                                                            editor_view.update(cx, |editor, cx| {
                                                                let uri_str = format!("file://{}", new_path_clone.to_string_lossy());
                                                                if let Ok(new_uri) = <Uri as std::str::FromStr>::from_str(&uri_str) {
                                                                    editor.uri = new_uri;
                                                                }
                                                                cx.notify();
                                                            });
                                                            ws.editors.insert(new_path.clone(), editor_view);
                                                        }
                                                        
                                                        // 2. Update active editor path
                                                        if ws.active_editor_path.as_ref() == Some(&old_path) {
                                                            ws.active_editor_path = Some(new_path.clone());
                                                        }
                                                        
                                                        // 3. Update main file path if needed
                                                        let is_active = ws.world.root_path.as_ref()
                                                            .and_then(|r| ws.world.main_id.vpath().resolve(r))
                                                            .map(|path| path == old_path)
                                                            .unwrap_or(false);
                                                        
                                                        if is_active {
                                                            ws.project.main_file_path = Some(new_path.clone());
                                                            let root = ws.world.root_path.clone().unwrap_or_else(|| std::path::PathBuf::from("."));
                                                            let vpath = typst::syntax::VirtualPath::within_root(&new_path, &root).unwrap_or_else(|| {
                                                                let filename = new_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("main.typ"));
                                                                typst::syntax::VirtualPath::new(filename)
                                                            });
                                                            let main_id = FileId::new(None, vpath);
                                                            ws.world.main_id = main_id;
                                                            ws.world.main_source = typst::syntax::Source::new(main_id, ws.world.main_source.text().to_string());
                                                            ws.compile(cx);
                                                        }
                                                        
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        }
                                    this.editing_path = None;
                                    this.editing_input = None;
                                    this._rename_subscription = None;
                                    cx.notify();
                                }
                                InputEvent::Blur => {
                                    this.editing_path = None;
                                    this.editing_input = None;
                                    this._rename_subscription = None;
                                    cx.notify();
                                }
                                _ => {}
                            }
                        });
                    }
                }
            });

            self.editing_path = Some(path_str);
            self.editing_input = Some(input_state);
            self._rename_subscription = Some(sub);
            cx.notify();
        }
    }

    fn handle_duplicate(&self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_entry();
        if let Some(entry) = selected
            && !entry.is_folder() {
                let path_str = entry.item().id.to_string();
                let p = Path::new(&path_str).to_path_buf();

                let file_name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let stem = p
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let ext = p
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_default();
                let default_dup_name = format!("{}_copy{}", stem, ext);

                let input_state = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);
                    state.set_value(&default_dup_name, window, cx);
                    state.focus(window, cx);
                    state
                });

                let old_path = p.clone();
                let tree_state = self.tree_state.clone();
                let root = self.root.clone();
                let file_name_clone = file_name.clone();

                window.open_dialog(cx, move |dialog, _window, _cx| {
                    let name_original = file_name_clone.clone();
                    dialog
                        .title("Duplicate File")
                        .confirm()
                        .on_ok({
                            let input_state = input_state.clone();
                            let old_path = old_path.clone();
                            let tree_state = tree_state.clone();
                            let root = root.clone();
                            move |_, _, cx| {
                                let new_name =
                                    input_state.read(cx).text().to_string().trim().to_string();
                                if !new_name.is_empty() && new_name != name_original
                                    && let Some(parent) = old_path.parent() {
                                        let new_path = parent.join(new_name);
                                        if std::fs::copy(&old_path, &new_path).is_ok() {
                                            Self::refresh(&tree_state, &root, cx);
                                        }
                                    }
                                true
                            }
                        })
                        .child(
                            div()
                                .w_full()
                                .py_2()
                                .child(gpui_component::input::Input::new(&input_state)),
                        )
                });
            }
    }

    fn handle_open(&self, window: &mut Window, cx: &mut App) {
        let selected = self.tree_state.read(cx).selected_entry();
        if let Some(entry) = selected
            && !entry.is_folder() {
                let path_str = entry.item().id.to_string();
                if let Some(workspace) = self._workspace.upgrade() {
                    workspace.update(cx, |workspace, cx| {
                        workspace.open_file(&path_str, window, cx);
                    });
                }
            }
    }
}

impl Render for FileTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this_weak = cx.entity().downgrade();
        div()
            .size_full()
            .bg(cx.theme().background)
            .key_context("Tree")
            .track_focus(&self.focus_handle)
            // Register action dispatch listeners for premium keyboard hotkeys!
            .on_action(cx.listener(|this, _action: &NewFile, window, cx| {
                this.handle_new_file(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &NewFolder, window, cx| {
                this.handle_new_folder(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &Delete, window, cx| {
                this.handle_delete(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &Rename, window, cx| {
                this.handle_rename(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &Duplicate, window, cx| {
                this.handle_duplicate(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &Open, window, cx| {
                this.handle_open(window, cx);
            }))
            .child(
                Tree::new(&self.tree_state, {
                    let workspace = self._workspace.clone();
                    let file_tree_state = self.tree_state.clone();
                    let focus_handle_for_menu = self.tree_state.read(cx).focus_handle();
                    let this_weak = this_weak.clone();
                    let editing_path = self.editing_path.clone();
                    let editing_input = self.editing_input.clone();
                    move |ix, entry, _selected, _window, cx| {
                        let item = entry.item();
                        let depth = entry.depth();
                        let is_expanded = entry.is_expanded();
                        let path_str = item.id.to_string();
                        let is_folder = entry.is_folder();
                        let file_tree_state_click = file_tree_state.clone();
                        let this_weak = this_weak.clone();

                        ListItem::new(ix)
                            .on_click({
                                let workspace = workspace.clone();
                                let path_str = path_str.clone();
                                let file_tree_state_click = file_tree_state_click.clone();
                                move |_, window, cx| {
                                    // Focus the tree so the key context and actions work perfectly
                                    file_tree_state_click.read(cx).focus_handle().focus(window);
                                    if !is_folder && let Some(workspace) = workspace.upgrade() {
                                            workspace.update(cx, |workspace, cx| {
                                                workspace.open_file(&path_str, window, cx);
                                            });
                                    }
                                }
                            })
                            .child(
                                h_flex()
                                    .size_full()
                                    .gap_2()
                                    .child(
                                        div()
                                            .context_menu({
                                                let tree_state = file_tree_state.clone();
                                                let focus_handle = focus_handle_for_menu.clone();
                                                let this_weak = this_weak.clone();
                                                move |menu, window, cx| {
                                                    // Select this item automatically on right click
                                                    tree_state.update(cx, |state, cx| {
                                                        state.set_selected_index(Some(ix), cx);
                                                    });
                                                    // Focus the tree focus handle
                                                    focus_handle.focus(window);

                                                    let menu = menu.action_context(focus_handle.clone());

                                                    if is_folder {
                                                        let this_weak = this_weak.clone();
                                                        let this_weak2 = this_weak.clone();
                                                        let this_weak3 = this_weak.clone();
                                                        let this_weak4 = this_weak.clone();
                                                        menu
                                                            .item(PopupMenuItem::new("New File").action(Box::new(NewFile)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_new_file(window, cx));
                                                                }
                                                            }))
                                                            .item(PopupMenuItem::new("New Folder").action(Box::new(NewFolder)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak2.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_new_folder(window, cx));
                                                                }
                                                            }))
                                                            .separator()
                                                            .item(PopupMenuItem::new("Rename").action(Box::new(Rename)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak3.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_rename(window, cx));
                                                                }
                                                            }))
                                                            .item(PopupMenuItem::new("Delete").action(Box::new(Delete)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak4.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_delete(window, cx));
                                                                }
                                                            }))
                                                    } else {
                                                        let this_weak = this_weak.clone();
                                                        let this_weak2 = this_weak.clone();
                                                        let this_weak3 = this_weak.clone();
                                                        let this_weak4 = this_weak.clone();
                                                        menu
                                                            .item(PopupMenuItem::new("Open File").action(Box::new(Open)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak.upgrade() {
                                                                    this.update(cx, |this, cx| {
                                                                        this.handle_open(window, &mut *cx);
                                                                    });
                                                                }
                                                            }))
                                                            .item(PopupMenuItem::new("Duplicate").action(Box::new(Duplicate)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak2.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_duplicate(window, cx));
                                                                }
                                                            }))
                                                            .separator()
                                                            .item(PopupMenuItem::new("Rename").action(Box::new(Rename)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak3.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_rename(window, cx));
                                                                }
                                                            }))
                                                            .item(PopupMenuItem::new("Delete").action(Box::new(Delete)).on_click(move |_, window, cx| {
                                                                if let Some(this) = this_weak4.upgrade() {
                                                                    this.update(cx, |this, cx| this.handle_delete(window, cx));
                                                                }
                                                            }))
                                                    }
                                                }
                                            })
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl(px(16.0 * depth as f32))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .child(if entry.is_folder() {
                                                        if is_expanded { "📂" } else { "📁" }
                                                    } else {
                                                        "📄"
                                                    })
                                            )
                                            .child({
                                                let is_editing = editing_path.as_ref()
                                                    .map(|p| p == &entry.item().id.to_string())
                                                    .unwrap_or(false);
                                                
                                                if is_editing {
                                                    let input_state = editing_input.as_ref().unwrap().clone();
                                                    div()
                                                        .flex_1()
                                                        .child(
                                                            div()
                                                                .on_key_down({
                                                                    let this_weak = this_weak.clone();
                                                                    move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
                                                                        if event.keystroke.key == "escape" {
                                                                            if let Some(entity) = this_weak.upgrade() {
                                                                                entity.update(cx, |this, cx| {
                                                                                    this.editing_path = None;
                                                                                    this.editing_input = None;
                                                                                    this._rename_subscription = None;
                                                                                    cx.notify();
                                                                                });
                                                                            }
                                                                        }
                                                                    }
                                                                })
                                                                .child(
                                                                    gpui_component::input::Input::new(&input_state)
                                                                        .with_size(gpui_component::Size::Small)
                                                                )
                                                        )
                                                } else {
                                                    div()
                                                        .text_sm()
                                                        .text_color(cx.theme().foreground)
                                                        .child(item.label.clone())
                                                }
                                            })
                                    )
                                    .child({
                                        let is_typ = path_str.ends_with(".typ");
                                        if is_typ && !is_folder {
                                            let workspace = workspace.clone();
                                            let path_str = path_str.clone();
                                            let is_main = if let Some(ws) = workspace.upgrade() {
                                                ws.read(cx).project.main_file_path.as_ref()
                                                    .map(|p| p.to_string_lossy() == path_str)
                                                    .unwrap_or(false)
                                            } else {
                                                false
                                            };
                                            
                                            div()
                                                .ml_auto()
                                                .pr_2()
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                                .child(
                                                    Switch::new(SharedString::from(format!("main_{}", path_str)))
                                                        .checked(is_main)
                                                        .on_click({
                                                            let workspace = workspace.clone();
                                                            let path_str = path_str.clone();
                                                            move |_, window, cx| {
                                                                if let Some(ws) = workspace.upgrade() {
                                                                    ws.update(cx, |ws, cx| {
                                                                        ws.set_main_file(PathBuf::from(&path_str), window, cx);
                                                                    });
                                                                }
                                                            }
                                                        })
                                                )
                                        } else {
                                            div()
                                        }
                                    })
                            )
                    }
                })
            )
    }
}

impl Focusable for FileTree {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui_component::dock::Panel for FileTree {
    fn panel_name(&self) -> &'static str {
        "FileTree"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Explorer"
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for FileTree {}
