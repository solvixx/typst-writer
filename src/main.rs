use gpui::*;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use typst::layout::PagedDocument;
use typst::syntax::{Source, LinkedNode, SyntaxKind};

use typst_writer::compiler::SimpleWorld;
use typst_writer::geometry;
use typst_writer::painter;

// --- APP STATE & VIEW ---

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorContext {
    Markup,
    Math,
    Code,
}

struct TypstWysiwygApp {
    world: SimpleWorld,
    compiled_document: Option<PagedDocument>,
    error_message: Option<String>,
    cursor_offset: usize,
    cursor_relative_pos: Option<(usize, Point<Pixels>)>, // (page_index, coordinate relative to page origin)
    selected_node_info: Option<String>,
    editor_context: EditorContext,
    context_title: String,
    context_desc: String,
    focus_handle: FocusHandle,

    // Drag-highlighting selection range
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    is_dragging: bool,

    // Background compilation debounced task
    background_task: Option<gpui::Task<()>>,
    is_compiling: bool,
    needs_recompile: bool,
}

impl TypstWysiwygApp {
    fn new(cx: &mut Context<Self>, initial_text: &str) -> Self {
        let source = Source::detached(initial_text);
        let world = SimpleWorld::new(source);
        let mut app = Self {
            world,
            compiled_document: None,
            error_message: None,
            cursor_offset: 0,
            cursor_relative_pos: None,
            selected_node_info: None,
            editor_context: EditorContext::Markup,
            context_title: "Document Mode 📄".to_string(),
            context_desc: "Writing regular rich text markup.".to_string(),
            focus_handle: cx.focus_handle(),
            selection_start: None,
            selection_end: None,
            is_dragging: false,
            background_task: None,
            is_compiling: false,
            needs_recompile: false,
        };
        app.compile(cx);
        app
    }

    fn get_active_slice(&self) -> (Source, usize) {
        let text = self.world.source_ref().text();
        let cursor = self.cursor_offset.min(text.len());
        
        // 2,500 characters before and after (approx 5KB slice, ~1 page of active writing)
        let mut start = cursor.saturating_sub(2500);
        let mut end = (cursor + 2500).min(text.len());
        
        if start > 0 {
            if let Some(pos) = text[start..cursor].find("\n\n") {
                start += pos + 2;
            } else if let Some(pos) = text[start..cursor].find("=") {
                start += pos;
            }
        }
        
        if end < text.len() {
            if let Some(pos) = text[cursor..end].rfind("\n\n") {
                end = cursor + pos;
            } else if let Some(pos) = text[cursor..end].rfind("=") {
                end = cursor + pos;
            }
        }
        
        while !text.is_char_boundary(start) && start > 0 {
            start -= 1;
        }
        while !text.is_char_boundary(end) && end < text.len() {
            end += 1;
        }
        
        let slice_str = &text[start..end];
        let slice_source = Source::detached(slice_str);
        let relative_cursor = cursor - start;
        
        (slice_source, relative_cursor)
    }

    fn schedule_background_compile(&mut self, cx: &mut Context<Self>) {
        if self.is_compiling {
            self.needs_recompile = true;
            return;
        }
        
        self.is_compiling = true;
        self.needs_recompile = false;
        
        let world_clone = self.world.clone();
        let cursor_offset = self.cursor_offset;
        
        let task = cx.spawn(move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                // Debounce delay of 250ms (reduced for even greater responsiveness)
                cx.background_executor().timer(Duration::from_millis(250)).await;
                
                // Execute heavy compilation asynchronously on the thread pool
                let warned = cx.background_executor().spawn(async move {
                    typst::compile::<PagedDocument>(&world_clone)
                }).await;
                
                this.update(&mut cx, |this, cx| {
                    this.is_compiling = false;
                    
                    match warned.output {
                        Ok(doc) => {
                            this.compiled_document = Some(doc.clone());
                            this.error_message = None;
                            
                            let mut found = false;
                            for (i, page) in doc.pages.iter().enumerate() {
                                if let Some(pos) = geometry::find_cursor_position(
                                    &page.frame,
                                    Point::default(),
                                    cursor_offset,
                                    this.world.source_ref(),
                                ) {
                                    this.cursor_relative_pos = Some((i, pos));
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                this.cursor_relative_pos = None;
                            }
                        }
                        Err(diags) => {
                            let errors = diags.iter()
                                .map(|d| format!("{:?}", d.message))
                                .collect::<Vec<_>>()
                                .join("\n");
                            this.error_message = Some(errors);
                        }
                    }
                    this.update_cursor_node_info();
                    cx.notify();
                    
                    // If edits happened during the compile phase, immediately trigger next serial compilation
                    if this.needs_recompile {
                        this.schedule_background_compile(cx);
                    }
                }).ok();
            }
        });
        
        self.background_task = Some(task);
    }

    fn compile(&mut self, cx: &mut Context<Self>) {
        let text = self.world.source_ref().text();
        
        if text.len() > 100_000 {
            // Large document -> fast viewport-bound slice compilation
            let (slice_source, relative_cursor) = self.get_active_slice();
            let slice_world = SimpleWorld::new(slice_source.clone());
            let warned = typst::compile::<PagedDocument>(&slice_world);
            
            match warned.output {
                Ok(doc) => {
                    self.compiled_document = Some(doc.clone());
                    self.error_message = None;
                    
                    let mut found = false;
                    for (i, page) in doc.pages.iter().enumerate() {
                        if let Some(pos) = geometry::find_cursor_position(
                            &page.frame,
                            Point::default(),
                            relative_cursor,
                            &slice_source,
                        ) {
                            self.cursor_relative_pos = Some((i, pos));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        self.cursor_relative_pos = None;
                    }
                }
                Err(diags) => {
                    let errors = diags.iter()
                        .map(|d| format!("{:?}", d.message))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.error_message = Some(errors);
                }
            }
            
            // Trigger background full compile sync
            self.schedule_background_compile(cx);
        } else {
            // Standard small document -> full compile
            let warned = typst::compile::<PagedDocument>(&self.world);
            
            match warned.output {
                Ok(doc) => {
                    self.compiled_document = Some(doc.clone());
                    self.error_message = None;
                    
                    let mut found = false;
                    for (i, page) in doc.pages.iter().enumerate() {
                        if let Some(pos) = geometry::find_cursor_position(
                            &page.frame,
                            Point::default(),
                            self.cursor_offset,
                            self.world.source_ref(),
                        ) {
                            self.cursor_relative_pos = Some((i, pos));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        self.cursor_relative_pos = None;
                    }
                }
                Err(diags) => {
                    let errors = diags.iter()
                        .map(|d| format!("{:?}", d.message))
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.error_message = Some(errors);
                }
            }
        }
        
        self.update_cursor_node_info();
        cx.notify();
    }

    fn update_cursor_node_info(&mut self) {
        let root = self.world.source_ref().root();
        let linked = LinkedNode::new(root);
        
        let offset = self.cursor_offset.min(self.world.source_ref().text().len());
        if let Some(leaf) = linked.leaf_at(offset, typst::syntax::Side::Before) {
            self.selected_node_info = Some(format!(
                "Kind: {:?}\nRange: {:?}\nText: {:?}",
                leaf.kind(),
                leaf.range(),
                leaf.text().to_string()
            ));

            // Traverse ancestors to determine specialized cursor context
            let mut resolved = false;
            let mut current = Some(leaf.clone());
            while let Some(node) = current {
                match node.kind() {
                    SyntaxKind::Equation | SyntaxKind::Math | SyntaxKind::MathText | SyntaxKind::MathIdent => {
                        self.editor_context = EditorContext::Math;
                        self.context_title = "Math Mode 📐".to_string();
                        self.context_desc = "Editing algebraic expressions. Use '^' for superscripts, '_' for subscripts.".to_string();
                        resolved = true;
                        break;
                    }
                    SyntaxKind::CodeBlock 
                    | SyntaxKind::Code 
                    | SyntaxKind::LetBinding 
                    | SyntaxKind::SetRule
                    | SyntaxKind::ShowRule
                    | SyntaxKind::FuncCall => {
                        self.editor_context = EditorContext::Code;
                        self.context_title = "Code Mode 💻".to_string();
                        self.context_desc = "Editing functional code blocks, styles, or bindings.".to_string();
                        resolved = true;
                        break;
                    }
                    _ => {}
                }
                current = node.parent().cloned();
            }

            if !resolved {
                self.editor_context = EditorContext::Markup;
                self.context_title = "Document Mode 📄".to_string();
                self.context_desc = "Writing regular rich text markup.".to_string();
            }
        } else {
            self.selected_node_info = None;
            self.editor_context = EditorContext::Markup;
            self.context_title = "Document Mode 📄".to_string();
            self.context_desc = "Writing regular rich text markup.".to_string();
        }
    }

    fn render_source_with_caret(&self) -> String {
        let text = self.world.source_ref().text();
        let offset = self.cursor_offset.min(text.len());
        let mut display = text[..offset].to_string();
        display.push('▎'); // Elegant high-contrast caret
        display.push_str(&text[offset..]);
        display
    }
}

impl Render for TypstWysiwygApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let doc = self.compiled_document.clone();
        let error = self.error_message.clone();
        let source_display = self.render_source_with_caret();
        let node_info = self.selected_node_info.clone();
        let context_title = self.context_title.clone();
        let context_desc = self.context_desc.clone();
        let cursor_offset = self.cursor_offset;

        // Parent container capturing key focus
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x0f172a)) // Slate-900
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _window, cx| {
                let key = &event.keystroke.key;
                let mut changed = false;

                // Preserve existing selection for delete/overwrite; navigation keys clear it
                let had_selection = this.selection_start.zip(this.selection_end)
                    .filter(|(s, e)| s != e)
                    .map(|(s, e)| if s < e { (s, e) } else { (e, s) });
                
                let current_text = this.world.source_ref().text();
                let pos = this.cursor_offset.min(current_text.len());
                
                match key.as_str() {
                    "backspace" => {
                        if let Some((sel_start, sel_end)) = had_selection {
                            let mut handled = false;
                            if this.editor_context == EditorContext::Math {
                                let source_snap = this.world.source_ref().clone();
                                
                                if current_text[sel_start..sel_end] == *"?" {
                                    if let Some((full_range, _)) = geometry::find_math_attachment_ranges(&source_snap, sel_start + 1) {
                                        if full_range != (sel_start..sel_end) {
                                            // Stage 2 is skipped: delete the full attachment field directly
                                            this.world.source_mut().edit(full_range.clone(), "");
                                            this.cursor_offset = full_range.start;
                                            this.selection_start = None;
                                            this.selection_end = None;
                                            this.compile(cx);
                                            cx.notify();
                                            return;
                                        }
                                    }
                                }

                                // Stage 1.5 -> 2: If the selection is exactly the inner content of an attachment (e.g. `a=1` inside `_(a=1)`),
                                // deleting it should replace it with `?` and select `?`!
                                if let Some((_full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, sel_end) {
                                    if inner_range == (sel_start..sel_end) {
                                        this.world.source_mut().edit(sel_start..sel_end, "?");
                                        this.selection_start = Some(sel_start);
                                        this.selection_end = Some(sel_start + 1);
                                        this.cursor_offset = sel_start + 1;
                                        handled = true;
                                    }
                                }

                                // Expand: if the selection is the base of a MathAttach (e.g. `sum` selected),
                                // expand selection to the full structure `sum_(a)^(b)` instead of deleting just the base.
                                if !handled {
                                    if let Some(intercept_range) = geometry::intercept_math_deletion(&source_snap, sel_start..sel_end) {
                                        if intercept_range != (sel_start..sel_end) {
                                            this.selection_start = Some(intercept_range.start);
                                            this.selection_end = Some(intercept_range.end);
                                            this.cursor_offset = intercept_range.start;
                                            cx.notify();
                                            return;
                                        }
                                    }
                                }
                            }
                            
                            if !handled {
                                // Stage 3 or normal delete: active selection → delete it atomically
                                this.world.source_mut().edit(sel_start..sel_end, "");
                                this.cursor_offset = sel_start;
                                this.selection_start = None;
                                this.selection_end = None;
                            }
                            changed = true;
                        } else if pos > 0 {
                            let mut prev_char_len = 1;
                            if let Some(prev_idx) = current_text[..pos].char_indices().map(|(idx, _)| idx).last() {
                                prev_char_len = pos - prev_idx;
                            }
                            let start = pos - prev_char_len;

                            if this.editor_context == EditorContext::Math {
                                let source_snap = this.world.source_ref().clone();
                                
                                // Intercept deletion if it targets a structural math element
                                if let Some(intercept_range) = geometry::intercept_math_deletion(&source_snap, start..pos) {
                                    let mut final_range = intercept_range.clone();
                                    
                                    if current_text[intercept_range.clone()] == *"?" {
                                        if let Some((full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, intercept_range.end) {
                                            if inner_range == intercept_range {
                                                final_range = full_range;
                                            }
                                        }
                                    }

                                    this.selection_start = Some(final_range.start);
                                    this.selection_end = Some(final_range.end);
                                    this.cursor_offset = final_range.start;
                                    cx.notify();
                                    return;
                                }

                                if let Some((full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, pos) {
                                    if inner_range == (start..pos) {
                                        if current_text[start..pos] == *"?" {
                                            // The content is ALREADY `?`. Select the WHOLE field.
                                            this.selection_start = Some(full_range.start);
                                            this.selection_end = Some(full_range.end);
                                            this.cursor_offset = full_range.start;
                                            cx.notify();
                                            return;
                                        } else {
                                            // Stage 1: deleting the ONLY character inside the attachment field
                                            // replace it with `?` and select the `?`
                                            this.world.source_mut().edit(start..pos, "?");
                                            this.selection_start = Some(start);
                                            this.selection_end = Some(start + 1);
                                            this.cursor_offset = start + 1;
                                            changed = true;
                                        }
                                    } else {
                                        // Normal deletion inside the math field
                                        this.world.source_mut().edit(start..pos, "");
                                        this.cursor_offset = start;
                                        changed = true;
                                    }
                                } else {
                                    this.world.source_mut().edit(start..pos, "");
                                    this.cursor_offset = start;
                                    changed = true;
                                }
                            } else {
                                this.world.source_mut().edit(start..pos, "");
                                this.cursor_offset = start;
                                changed = true;
                            }
                        }
                    }
                    "delete" => {
                        if let Some((sel_start, sel_end)) = had_selection {
                            let mut handled = false;
                            if this.editor_context == EditorContext::Math {
                                let source_snap = this.world.source_ref().clone();
                                
                                if current_text[sel_start..sel_end] == *"?" {
                                    if let Some((full_range, _)) = geometry::find_math_attachment_ranges(&source_snap, sel_start + 1) {
                                        if full_range != (sel_start..sel_end) {
                                            // Stage 2 is skipped: delete the full attachment field directly
                                            this.world.source_mut().edit(full_range.clone(), "");
                                            this.cursor_offset = full_range.start;
                                            this.selection_start = None;
                                            this.selection_end = None;
                                            this.compile(cx);
                                            cx.notify();
                                            return;
                                        }
                                    }
                                }

                                // Stage 1.5 -> 2: If the selection is exactly the inner content of an attachment (e.g. `a=1` inside `_(a=1)`),
                                // deleting it should replace it with `?` and select `?`!
                                if let Some((_full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, sel_end) {
                                    if inner_range == (sel_start..sel_end) {
                                        this.world.source_mut().edit(sel_start..sel_end, "?");
                                        this.selection_start = Some(sel_start);
                                        this.selection_end = Some(sel_start + 1);
                                        this.cursor_offset = sel_start + 1;
                                        handled = true;
                                    }
                                }

                                // Expand: if the selection is the base of a MathAttach (e.g. `sum` selected),
                                // expand selection to the full structure `sum_(a)^(b)` instead of deleting just the base.
                                if !handled {
                                    if let Some(intercept_range) = geometry::intercept_math_deletion(&source_snap, sel_start..sel_end) {
                                        if intercept_range != (sel_start..sel_end) {
                                            this.selection_start = Some(intercept_range.start);
                                            this.selection_end = Some(intercept_range.end);
                                            this.cursor_offset = intercept_range.start;
                                            cx.notify();
                                            return;
                                        }
                                    }
                                }
                            }
                            if !handled {
                                // Erase the selected field entirely
                                this.world.source_mut().edit(sel_start..sel_end, "");
                                this.cursor_offset = sel_start;
                                this.selection_start = None;
                                this.selection_end = None;
                            }
                            changed = true;
                        } else if pos < current_text.len() {
                            let mut next_char_len = 1;
                            if let Some(next_char) = current_text[pos..].chars().next() {
                                next_char_len = next_char.len_utf8();
                            }
                            let end = pos + next_char_len;
                            
                            if this.editor_context == EditorContext::Math {
                                let source_snap = this.world.source_ref().clone();
                                if let Some(intercept_range) = geometry::intercept_math_deletion(&source_snap, pos..end) {
                                    let mut final_range = intercept_range.clone();
                                    
                                    if current_text[intercept_range.clone()] == *"?" {
                                        if let Some((full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, intercept_range.end) {
                                            if inner_range == intercept_range {
                                                final_range = full_range;
                                            }
                                        }
                                    }

                                    this.selection_start = Some(final_range.start);
                                    this.selection_end = Some(final_range.end);
                                    this.cursor_offset = final_range.start;
                                    cx.notify();
                                    return;
                                }

                                // Stage 1: deleting the ONLY character inside the attachment field
                                // replace it with `?` and select the `?`
                                if let Some((full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, end) {
                                    if inner_range == (pos..end) {
                                        if current_text[pos..end] == *"?" {
                                            // The content is ALREADY `?`. Select the WHOLE field.
                                            this.selection_start = Some(full_range.start);
                                            this.selection_end = Some(full_range.end);
                                            this.cursor_offset = full_range.start;
                                            cx.notify();
                                            return;
                                        } else {
                                            this.world.source_mut().edit(pos..end, "?");
                                            this.selection_start = Some(pos);
                                            this.selection_end = Some(pos + 1);
                                            this.cursor_offset = pos + 1;
                                            this.compile(cx);
                                            cx.notify();
                                            return;
                                        }
                                    }
                                }
                            }
                            
                            this.world.source_mut().edit(pos..end, "");
                            changed = true;
                        }
                    }
                    "left" => {
                        this.selection_start = None;
                        this.selection_end = None;
                        if pos > 0 {
                            if let Some(prev_idx) = current_text[..pos].char_indices().map(|(idx, _)| idx).last() {
                                this.cursor_offset = prev_idx;
                                this.update_cursor_node_info();
                                // Refresh caret location on arrow movement
                                if let Some(doc) = &this.compiled_document {
                                    let mut found = false;
                                    for (i, page) in doc.pages.iter().enumerate() {
                                        if let Some(pos) = geometry::find_cursor_position(
                                            &page.frame,
                                            Point::default(),
                                            this.cursor_offset,
                                            this.world.source_ref(),
                                        ) {
                                            this.cursor_relative_pos = Some((i, pos));
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !found {
                                        this.cursor_relative_pos = None;
                                    }
                                }
                                cx.notify();
                            }
                        }
                    }
                    "right" => {
                        this.selection_start = None;
                        this.selection_end = None;
                        if pos < current_text.len() {
                            if let Some(next_char) = current_text[pos..].chars().next() {
                                this.cursor_offset = pos + next_char.len_utf8();
                                this.update_cursor_node_info();
                                // Refresh caret location on arrow movement
                                if let Some(doc) = &this.compiled_document {
                                    let mut found = false;
                                    for (i, page) in doc.pages.iter().enumerate() {
                                        if let Some(pos) = geometry::find_cursor_position(
                                            &page.frame,
                                            Point::default(),
                                            this.cursor_offset,
                                            this.world.source_ref(),
                                        ) {
                                            this.cursor_relative_pos = Some((i, pos));
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !found {
                                        this.cursor_relative_pos = None;
                                    }
                                }
                                cx.notify();
                            }
                        }
                    }
                    "enter" => {
                        this.world.source_mut().edit(pos..pos, "\n");
                        this.cursor_offset += 1;
                        changed = true;
                    }
                    "space" => {
                        this.world.source_mut().edit(pos..pos, " ");
                        this.cursor_offset += 1;
                        changed = true;
                    }
                    "tab" => {
                        if this.editor_context == EditorContext::Math {
                            let text = this.world.source_ref().text();
                            if event.keystroke.modifiers.shift {
                                // Shift-Tab: Find PREVIOUS '?' placeholder
                                let start_search = this.cursor_offset.min(text.len());
                                if let Some(offset) = text[..start_search].rfind('?') {
                                    this.cursor_offset = offset;
                                    this.update_cursor_node_info();
                                    this.cursor_relative_pos = None;
                                    cx.notify();
                                } else if let Some(offset) = text[start_search..].rfind('?') {
                                    // Wrap around from end
                                    this.cursor_offset = start_search + offset;
                                    this.update_cursor_node_info();
                                    this.cursor_relative_pos = None;
                                    cx.notify();
                                }
                            } else {
                                // Tab: Find NEXT '?' placeholder
                                let start_search = this.cursor_offset.min(text.len());
                                let mut found = false;
                                if start_search < text.len() {
                                    if let Some(offset) = text[(start_search + 1).min(text.len())..].find('?') {
                                        this.cursor_offset = (start_search + 1).min(text.len()) + offset;
                                        found = true;
                                    }
                                }
                                if !found {
                                    // Wrap around from start
                                    if let Some(offset) = text[..start_search].find('?') {
                                        this.cursor_offset = offset;
                                    }
                                }
                                this.update_cursor_node_info();
                                this.cursor_relative_pos = None;
                                cx.notify();
                            }
                        } else {
                            this.world.source_mut().edit(pos..pos, "  ");
                            this.cursor_offset += 2;
                            changed = true;
                        }
                    }
                    other => {
                        // Accept standard typed unicode characters (excluding modifier chords)
                        if other.chars().count() == 1 {
                            if let Some((sel_start, sel_end)) = had_selection {
                                let mut handled = false;
                                if this.editor_context == EditorContext::Math {
                                    let source_snap = this.world.source_ref().clone();
                                    // The selection might be an entire math attachment field like `_(?)` or `^(a=1)`.
                                    // If so, typing should only replace the inner value, preserving the attachment wrapper.
                                    if let Some((full_range, inner_range)) = geometry::find_math_attachment_ranges(&source_snap, sel_end) {
                                        if full_range == (sel_start..sel_end) {
                                            this.world.source_mut().edit(inner_range.clone(), other);
                                            this.cursor_offset = inner_range.start + other.len();
                                            this.selection_start = None;
                                            this.selection_end = None;
                                            handled = true;
                                        }
                                    }
                                }

                                if !handled {
                                    // Replace entire selected group with typed char
                                    this.world.source_mut().edit(sel_start..sel_end, other);
                                    this.cursor_offset = sel_start + other.len();
                                    this.selection_start = None;
                                    this.selection_end = None;
                                }
                            } else if this.editor_context == EditorContext::Math && current_text[pos..].starts_with('?') {
                                // Smart overwrite: cursor is directly before a lone `?` placeholder
                                this.world.source_mut().edit(pos..(pos + 1), other);
                                this.cursor_offset = pos + other.len();
                            } else {
                                this.world.source_mut().edit(pos..pos, other);
                                this.cursor_offset += other.len();
                            }
                            changed = true;
                        }
                    }
                }
                
                if changed {
                    this.compile(cx);
                    cx.notify();
                }
            }))
            .child(
                // Header / Ribbon bar with smooth glassmorphism
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .h(px(64.0))
                    .px_6()
                    .bg(rgb(0x1e293b)) // Slate-800
                    .border_b_1()
                    .border_color(rgb(0x334155)) // Slate-700
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_x_3()
                            .child(
                                // Gradient Logo
                                div()
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .size(px(32.0))
                                    .rounded_md()
                                    .bg(linear_gradient(
                                        135.,
                                        linear_color_stop(rgb(0x3b82f6), 0.0), // Blue-500
                                        linear_color_stop(rgb(0x8b5cf6), 1.0), // Violet-500
                                    ))
                                    .child(
                                        div()
                                            .text_color(rgb(0xf8fafc))
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .child("T")
                                    )
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_color(rgb(0xf8fafc))
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Typst Writer")
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x94a3b8))
                                            .text_xs()
                                            .child("Next-Gen Native WYSIWYG")
                                    )
                            )
                    )
                    .child(
                        // Context-aware dynamic toolbar action strip
                        div()
                            .flex()
                            .items_center()
                            .gap_x_2()
                            .children(match self.editor_context {
                                EditorContext::Markup => vec![
                                    self.action_button("H1", "heading", cx),
                                    self.action_button("Bold", "bold", cx),
                                    self.action_button("Italic", "italic", cx),
                                    self.action_button("List", "list", cx),
                                    self.action_button("Formula", "math_formula", cx),
                                ],
                                EditorContext::Math => vec![
                                    self.action_button("Fraction", "math_frac", cx),
                                    self.action_button("Power", "math_super", cx),
                                    self.action_button("Subscript", "math_sub", cx),
                                    self.action_button("Sqrt", "math_root", cx),
                                    self.action_button("Matrix", "math_mat", cx),
                                    self.action_button("Sum", "math_sum", cx),
                                    self.action_button("Int", "math_int", cx),
                                    self.action_button("α", "math_alpha", cx),
                                    self.action_button("β", "math_beta", cx),
                                    self.action_button("γ", "math_gamma", cx),
                                    self.action_button("θ", "math_theta", cx),
                                    self.action_button("π", "math_pi", cx),
                                    self.action_button("ω", "math_omega", cx),
                                    self.action_button("Exit Math", "math_exit", cx),
                                ],
                                EditorContext::Code => vec![
                                    self.action_button("Let Binding", "code_let", cx),
                                    self.action_button("Set Rule", "code_set", cx),
                                    self.action_button("Show Rule", "code_show", cx),
                                    self.action_button("Exit Code", "code_exit", cx),
                                ],
                            })
                    )
            )
            .child(
                // Main Workspace Layout
                div()
                    .flex()
                    .size_full()
                    .child(
                        // Left-pane Source Panel
                        div()
                            .flex()
                            .flex_col()
                            .w_1_2()
                            .h_full()
                            .border_r_1()
                            .border_color(rgb(0x334155)) // Slate-700
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .h(px(40.0))
                                    .px_4()
                                    .bg(rgb(0x1e293b))
                                    .border_b_1()
                                    .border_color(rgb(0x334155))
                                    .child(
                                        div()
                                            .text_color(rgb(0x3b82f6))
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("SOURCE CODE")
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x64748b))
                                            .text_xs()
                                            .child(format!("Offset: {}", cursor_offset))
                                    )
                            )
                            .child(
                                // Source scroll viewport (Stateful Div)
                                div()
                                    .id("source-scroll")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .px_6()
                                    .py_4()
                                    .bg(rgb(0x0f172a))
                                    .child(
                                        div()
                                            .font_family("DejaVu Sans Mono")
                                            .text_color(rgb(0xe2e8f0))
                                            .text_sm()
                                            .child(source_display)
                                    )
                            )
                            .child(
                                // Metadata / CST Inspector panel
                                div()
                                    .h(px(140.0))
                                    .bg(rgb(0x0b0f19))
                                    .border_t_1()
                                    .border_color(rgb(0x1e293b))
                                    .px_4()
                                    .py_3()
                                    .child(
                                        div()
                                            .text_color(rgb(0x64748b))
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .child("ROWAN CST SYNTAX INSPECTOR")
                                    )
                                    .child(
                                        div()
                                            .mt_2()
                                            .font_family("DejaVu Sans Mono")
                                            .text_color(rgb(0x94a3b8))
                                            .text_xs()
                                            .child(node_info.unwrap_or_else(|| "No node selected. Click the right page canvas to inspect.".to_string()))
                                    )
                            )
                    )
                    .child(
                        // Right-pane Interactive WYSIWYG canvas
                        div()
                            .flex()
                            .flex_col()
                            .w_1_2()
                            .h_full()
                            .bg(rgb(0x1e293b)) // Dark charcoal preview backdrop
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .h(px(40.0))
                                    .px_4()
                                    .bg(rgb(0x0f172a))
                                    .border_b_1()
                                    .border_color(rgb(0x1e293b))
                                    .child(
                                        div()
                                            .text_color(rgb(0x3b82f6))
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("LIVE WYSIWYG RENDER")
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x3b82f6))
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .child(context_title)
                                    )
                            )
                            .child(
                                // WYSIWYG scroll workspace with smooth padding (Stateful Div)
                                div()
                                    .id("wysiwyg-scroll")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .px_8()
                                    .py_6()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .w_full()
                                            .child(
                                                if let Some(err) = error {
                                                    // Beautiful error visual banner
                                                    div()
                                                        .w_full()
                                                        .px_4()
                                                        .py_3()
                                                        .rounded_md()
                                                        .bg(rgba(0xef444415))
                                                        .border_1()
                                                        .border_color(rgb(0xef4444))
                                                        .child(
                                                            div()
                                                                .text_color(rgb(0xef4444))
                                                                .text_sm()
                                                                .font_weight(FontWeight::BOLD)
                                                                .child("Compilation Diagnostic:")
                                                        )
                                                        .child(
                                                            div()
                                                                .mt_1()
                                                                .font_family("DejaVu Sans Mono")
                                                                .text_color(rgb(0xfca5a5))
                                                                .text_xs()
                                                                .child(err)
                                                        )
                                                } else {
                                                    div().w_full().child("")
                                                }
                                            )
                                            .child(
                                                if let Some(d) = doc {
                                                    self.render_typst_document(d, cx)
                                                } else {
                                                    div().child("")
                                                }
                                            )
                                    )
                            )
                            .child(
                                // Bottom hint bar
                                div()
                                    .h(px(40.0))
                                    .bg(rgb(0x0f172a))
                                    .border_t_1()
                                    .border_color(rgb(0x1e293b))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_color(rgb(0x64748b))
                                            .text_xs()
                                            .child(context_desc)
                                    )
                            )
                    )
            )
    }
}

impl TypstWysiwygApp {
    fn action_button(&self, label: &'static str, action: &'static str, cx: &Context<Self>) -> impl IntoElement {
        div()
            .id(SharedString::from(action))
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(0x334155)) // Slate-700
            .text_color(rgb(0xf8fafc))
            .text_xs()
            .hover(|style| style.bg(rgb(0x475569))) // Slate-600
            .active(|style| style.bg(rgb(0x3b82f6))) // Blue-500
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                let current_text = this.world.source_ref().text();
                let edit_pos = this.cursor_offset.min(current_text.len());

                let (insert_str, cursor_rel) = match action {
                    // Markup Actions
                    "heading" => ("\n= New Section\n", 14),
                    "bold" => (" *bold text* ", 12),
                    "italic" => (" _italic text_ ", 14),
                    "list" => ("\n- List Item\n", 13),
                    "math_formula" => (" $? = ?_?$ ", 2),
                    
                    // Math Actions
                    "math_frac" => (" (? / ?) ", 2),
                    "math_super" => (" ?^? ", 1),
                    "math_sub" => (" ?_? ", 1),
                    "math_root" => (" sqrt(?) ", 6),
                    "math_mat" => (" mat(?, ?; ?, ?) ", 5),
                    "math_sum" => (" sum_(?)^? ? ", 6),
                    "math_int" => (" integral_(?)^? ? ", 10),
                    
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

                this.world.source_mut().edit(edit_pos..edit_pos, insert_str);
                this.cursor_offset = edit_pos + cursor_rel;
                this.cursor_relative_pos = None;
                this.compile(cx);
                cx.notify();
            }))
    }

    fn render_typst_document(&self, doc: PagedDocument, cx: &mut Context<Self>) -> Div {
        let pages = doc.pages.clone();
        
        // Multi-page normalizations
        let selection_range = if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            if start != end {
                Some((start, end))
            } else {
                None
            }
        } else {
            None
        };

        // Rendered Multi-Page Stack
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_y_6()
            .w_full()
            .children(
                pages.into_iter().enumerate().map(|(i, page)| {
                    let frame = page.frame.clone();
                    let page_width = px(frame.width().to_pt() as f32);
                    let page_height = px(frame.height().to_pt() as f32);

                    let frame_clone = frame.clone();
                    let source_paint = self.world.source_ref().clone();
                    
                    let frame_drag = frame.clone();
                    let source_drag = self.world.source_ref().clone();
                    let source_down = self.world.source_ref().clone();

                    let absolute_cursor = self.cursor_relative_pos;

                    // Dedicated origins to translate clicks precisely on each page frame
                    let page_origin = std::sync::Arc::new(std::sync::Mutex::new(Point::default()));
                    let page_origin_clone = page_origin.clone();
                    let page_origin_drag = page_origin.clone();

                    div()
                        .w(page_width)
                        .h(page_height)
                        .bg(gpui::white())
                        .shadow_xl()
                        .child(
                            canvas(
                                move |_, _, _| {},
                                move |bounds, _prepaint, window, cx| {
                                    if let Ok(mut origin) = page_origin_clone.lock() {
                                        *origin = bounds.origin;
                                    }
                                    painter::paint_frame(&frame_clone, bounds.origin, window, cx, &source_paint, selection_range);

                                    // Paint a visual cursor if active on this specific page index
                                    if let Some((caret_page_idx, rel_cursor)) = absolute_cursor {
                                        if caret_page_idx == i {
                                            let cursor_height = px(16.0);
                                            let cursor_bounds = Bounds {
                                                origin: point(
                                                    bounds.origin.x + rel_cursor.x,
                                                    bounds.origin.y + rel_cursor.y - cursor_height
                                                ),
                                                size: size(px(2.0), cursor_height),
                                            };
                                            window.paint_quad(quad(
                                                cursor_bounds,
                                                px(0.0),
                                                rgb(0x3b82f6), // Vibrant Blue-500 Caret
                                                px(0.0),
                                                gpui::transparent_black(),
                                                Default::default(),
                                            ));
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &gpui::MouseDownEvent, window, cx| {
                                let click_pos = ev.position;
                                let origin_val = if let Ok(origin) = page_origin.lock() {
                                    *origin
                                } else {
                                    Point::default()
                                };
                                
                                let relative_click = point(click_pos.x - origin_val.x, click_pos.y - origin_val.y);
                                if let Some((offset, rel_pos)) = geometry::find_closest_offset(&frame, Point::default(), relative_click, &source_down) {
                                    this.cursor_offset = offset;
                                    this.cursor_relative_pos = Some((i, rel_pos));
                                    this.update_cursor_node_info();

                                    // --- Math group selection on placeholder click ---
                                    // If the clicked character is `?` (a placeholder), expand the
                                    // selection to cover the full enclosing math AST group so the
                                    // user can delete or overwrite the whole structure atomically.
                                    if let Some(group_range) = geometry::find_math_group_range(&source_down, offset) {
                                        // First try to expand to the whole attachment field
                                        // Use group_range.end because Side::Before will perfectly resolve the target node
                                        if let Some((full_range, _)) = geometry::find_math_attachment_ranges(&source_down, group_range.end) {
                                            this.selection_start = Some(full_range.start);
                                            this.selection_end = Some(full_range.end);
                                            this.cursor_offset = group_range.start;
                                            this.is_dragging = false;
                                            window.focus(&this.focus_handle);
                                            cx.notify();
                                            return;
                                        }

                                        // Fallback to selecting just the placeholder character
                                        this.selection_start = Some(group_range.start);
                                        this.selection_end = Some(group_range.end);
                                        this.cursor_offset = group_range.start;
                                        this.is_dragging = false; // selection is complete, no drag needed
                                        window.focus(&this.focus_handle);
                                        cx.notify();
                                        return;
                                    }

                                    // Normal click: start interactive drag-highlighting selection
                                    this.selection_start = Some(offset);
                                    this.selection_end = Some(offset);
                                    this.is_dragging = true;

                                    window.focus(&this.focus_handle);
                                    cx.notify();
                                }
                            }),
                        )
                        .on_mouse_move(
                            cx.listener(move |this, ev: &gpui::MouseMoveEvent, _window, cx| {
                                if this.is_dragging {
                                    let click_pos = ev.position;
                                    let origin_val = if let Ok(origin) = page_origin_drag.lock() {
                                        *origin
                                    } else {
                                        Point::default()
                                    };

                                    let relative_click = point(click_pos.x - origin_val.x, click_pos.y - origin_val.y);
                                    if let Some((offset, rel_pos)) = geometry::find_closest_offset(&frame_drag, Point::default(), relative_click, &source_drag) {
                                        this.cursor_offset = offset;
                                        this.cursor_relative_pos = Some((i, rel_pos));
                                        this.selection_end = Some(offset);
                                        cx.notify();
                                    }
                                }
                            }),
                        )
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |this, _ev: &gpui::MouseUpEvent, _window, cx| {
                                this.is_dragging = false;
                                cx.notify();
                            }),
                        )
                })
            )
    }
}

fn main() {
    typst_writer::font_provisioner::provision_fonts();
    Application::new().run(|cx| {
        let mut fonts = Vec::new();
        
        // Register standard default system UI font
        let font_path = "/usr/share/fonts/noto/NotoSans-Regular.ttf";
        if let Ok(mut file) = File::open(font_path) {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() {
                fonts.push(std::borrow::Cow::Owned(buffer));
            }
        }

        // Register all bundled typst-assets fonts
        for font_bytes in typst_assets::fonts() {
            fonts.push(std::borrow::Cow::Owned(font_bytes.to_vec()));
        }

        cx.text_system().add_fonts(fonts).unwrap();

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                // Initialize with a beautiful, long multi-page sample document to showcase visual stack scrolling!
                let sample_doc = "= Page 1: Native WYSIWYG Layout\n\nThis is a true hardware-accelerated editor running on GPUI.\n\nTry clicking and dragging text here to see dynamic translucent highlights!\n\n#pagebreak()\n= Page 2: Mathematical Expressions\n\nInline equations like $a^2 + b^2 = c^2$ compile on every keystroke. Below is a block matrix:\n\n$ d / (d x) integral_a^x f(t) d t = f(x) $\n\nTry clicking math elements or writing code blocks to see live adaptive ribbon changes!\n$ sqrt(2 x + 1) $";
                let view = cx.new(|cx| TypstWysiwygApp::new(cx, sample_doc));
                view.update(cx, |this, _cx| {
                    window.focus(&this.focus_handle);
                });
                view
            },
        )
        .unwrap();
        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();
        cx.activate(true);
    });
}
