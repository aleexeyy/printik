use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config, EventKind, event::CreateKind};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::path::PathBuf;

pub struct FolderWatcher {
    watcher: Option<RecommendedWatcher>,
    tx: Sender<String>,
}

impl FolderWatcher {
    pub fn new(tx: Sender<String>) -> Self {
        Self {
            watcher: None,
            tx,
        }
    }

    pub fn spawn_watcher(&mut self, path: PathBuf) -> notify::Result<()> {
        // Stop existing watcher if any
        if let Some(mut w) = self.watcher.take() {
            w.unwatch(&path)?;
        }

        let tx_clone = self.tx.clone();
        let (watcher_tx, watcher_rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(watcher_tx, Config::default())?;
        watcher.watch(&path, RecursiveMode::Recursive)?;

        // Spawn a thread to handle events
        thread::spawn(move || {
            for res in watcher_rx {
                match res {
                    Ok(event) => {
                        if event.kind == EventKind::Create(CreateKind::File) {
                            for path in event.paths {
                                if let Some(ext) = path.extension() {
                                    if ext == "png" || ext == "jpg" || ext == "jpeg" {
                                        tx_clone.send(path.to_string_lossy().into_owned()).unwrap();
                                    }
                                
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Watch error: {:?}", e),
                }
            }
        });

        self.watcher = Some(watcher);
        Ok(())
    }
}
