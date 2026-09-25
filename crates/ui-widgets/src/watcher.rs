// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Asynchronous theme file watcher and dynamic hot-reloader
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use crate::theme::Theme;

/// File watcher that monitors a theme TOML configuration on disk and hot-reloads it in-memory.
pub struct ThemeWatcher {
    _watcher: RecommendedWatcher,
    pub path: PathBuf,
}

impl ThemeWatcher {
    /// Starts watching a theme TOML file. On modification, updates the given `Arc<RwLock<Theme>>`
    /// and invokes `on_change`.
    ///
    /// Malformed theme content or IO errors will be logged via `tracing::warn!` without panicking (INV-SEC-2).
    pub fn watch_file<P: AsRef<Path>, F>(
        path: P,
        theme_target: Arc<RwLock<Theme>>,
        on_change: F,
    ) -> anyhow::Result<Self>
    where
        F: Fn(&Theme) + Send + Sync + 'static,
    {
        let path_buf = path.as_ref().canonicalize().unwrap_or_else(|_| path.as_ref().to_path_buf());
        let watched_path = path_buf.clone();
        let target_clone = theme_target.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for p in &event.paths {
                            if p == &watched_path || watched_path.ends_with(p.file_name().unwrap_or_default()) {
                                match std::fs::read_to_string(&watched_path) {
                                    Ok(content) => match Theme::from_toml(&content) {
                                        Ok(new_theme) => {
                                            tracing::info!("Theme successfully hot-reloaded from {:?}", watched_path);
                                            if let Ok(mut lock) = target_clone.write() {
                                                *lock = new_theme.clone();
                                            }
                                            on_change(&new_theme);
                                        }
                                        Err(err) => {
                                            tracing::warn!("Failed to parse hot-reloaded theme from {:?}: {}", watched_path, err);
                                        }
                                    },
                                    Err(err) => {
                                        tracing::warn!("Failed to read theme file {:?}: {}", watched_path, err);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("ThemeWatcher notification error: {}", err);
                }
            },
            Config::default(),
        )?;

        // Watch the parent directory if possible, or the file itself
        if let Some(parent) = path_buf.parent() {
            if parent.exists() {
                watcher.watch(parent, RecursiveMode::NonRecursive)?;
            } else {
                watcher.watch(&path_buf, RecursiveMode::NonRecursive)?;
            }
        } else {
            watcher.watch(&path_buf, RecursiveMode::NonRecursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            path: path_buf,
        })
    }
}
