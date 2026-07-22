use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

/// Watches the assets directory for file changes to support hot reload
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    changed: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new(assets_dir: &Path) -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        watcher.watch(assets_dir, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
            changed: Vec::new(),
        })
    }

    /// Poll for file changes since last check
    pub fn poll_changes(&mut self) -> Vec<PathBuf> {
        self.changed.clear();

        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        if !self.changed.contains(&path) {
                            self.changed.push(path);
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        self.changed.clone()
    }
}