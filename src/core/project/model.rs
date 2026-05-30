use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

pub struct Project {
    pub root: PathBuf,
    pub _watcher: Option<Box<dyn Watcher + Send + Sync>>,
}

impl Project {
    pub fn new<F>(root: PathBuf, mut on_change: F) -> Self
    where
        F: FnMut(PathBuf) + Send + Sync + 'static,
    {
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                // CRITICAL: Only react to actual write/modify/create/delete events.
                // Inotify also fires CLOSE_NOWRITE when files are opened for *reading*.
                // Our handler calls std::fs::read_to_string() which opens+closes the file,
                // which fires CLOSE_NOWRITE → re-enters handler → reads file again → infinite loop!
                // Filtering to write-type events breaks this cycle.
                let is_write_event = matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                );
                if !is_write_event {
                    return;
                }

                for path in event.paths {
                    let mut is_ignored = false;
                    for component in path.components() {
                        let name = component.as_os_str().to_string_lossy();
                        if name == "target"
                            || name == ".git"
                            || name == ".gemini"
                            || name == "node_modules"
                        {
                            is_ignored = true;
                            break;
                        }
                    }
                    if !is_ignored {
                        on_change(path);
                    }
                }
            }
        })
        .ok();

        if let Some(watcher) = watcher.as_mut() {
            let _ = watcher.watch(&root, RecursiveMode::Recursive);
        }

        Self {
            root,
            _watcher: watcher.map(|w| Box::new(w) as Box<dyn Watcher + Send + Sync>),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
