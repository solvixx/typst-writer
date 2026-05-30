use std::ops::Range;

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub range: Range<usize>,
    pub old_text: String,
    pub new_text: String,
    pub old_cursor: usize,
    pub new_cursor: usize,
    pub old_selection: Option<Range<usize>>,
    pub new_selection: Option<Range<usize>>,
}

pub struct UndoManager {
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    max_depth: usize,
}

impl UndoManager {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn push(&mut self, entry: UndoEntry) {
        self.undo_stack.push(entry);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<UndoEntry> {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<UndoEntry> {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(entry.clone());
            Some(entry)
        } else {
            None
        }
    }
}
