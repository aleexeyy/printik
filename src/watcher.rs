use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
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
                        for path in event.paths {
                            if let Some(path_str) = path.to_str() {
                                tx_clone.send(path_str.to_string()).unwrap();
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
