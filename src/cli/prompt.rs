use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tokio::time::delay_for;

use crate::cli::ui::{Text, TextSegment};

#[async_trait]
pub trait PromptModule {
    /// Computes the module prompt content.
    async fn compute(&mut self) -> Option<Text>;
    /// Should the module content be recomputed.
    async fn should_update(&self, wd_changed: bool) -> bool;
    /// Timeout threshold to check for updates.
    async fn update_threshold(&self) -> Option<Duration>;
    /// The position in the prompt compared to other modules.
    fn position(&self) -> isize;
}

pub struct PromptConfig {
    pub threshold: Duration,
}

impl Default for PromptConfig {
    fn default() -> Self {
        PromptConfig {
            threshold: Duration::from_millis(200),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PromptHandle {
    last_prompt: Arc<RwLock<Arc<Text>>>,

    update_req_tx: Sender<bool>,
    late_updates_rx: Arc<Mutex<Receiver<()>>>,
}

type ModuleEntry = (Box<dyn PromptModule>, Option<Text>, Instant);

pub struct Prompt {
    modules: Vec<ModuleEntry>,
    config: PromptConfig,

    last_wd: Option<PathBuf>,
    last_prompt: Arc<RwLock<Arc<Text>>>,

    update_req_rx: Receiver<bool>,
    late_updates_tx: Sender<()>,
}

impl PromptHandle {
    pub async fn prompt(&self) -> Arc<Text> {
        self.last_prompt.read().await.clone()
    }

    pub async fn update(&mut self, force: bool) -> Result<()> {
        match self.update_req_tx.try_send(force) {
            // If successful or full an update is queued.
            Ok(()) | Err(TrySendError::Full(_)) => Ok(()),
            // The channel is disconnected.
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub fn late_updates(&self) -> Arc<Mutex<Receiver<()>>> {
        Arc::clone(&self.late_updates_rx)
    }
}

impl Prompt {
    pub fn new(config: PromptConfig) -> (Prompt, PromptHandle) {
        let modules = Vec::new();

        let last_wd = env::current_dir().ok();
        let last_prompt = Arc::new(RwLock::new(Arc::new(Text::EMPTY)));

        let (update_req_tx, update_req_rx) = mpsc::channel(1);
        let (late_updates_tx, late_updates_rx) = mpsc::channel(1);

        let prompt = Prompt {
            modules,
            config,

            last_wd,
            last_prompt,

            update_req_rx,
            late_updates_tx,
        };

        let handle = PromptHandle {
            last_prompt: Arc::clone(&prompt.last_prompt),

            update_req_tx,
            late_updates_rx: Arc::new(Mutex::new(late_updates_rx)),
        };

        (prompt, handle)
    }

    pub fn add_module(&mut self, module: Box<dyn PromptModule>) {
        self.modules.push((module, None, Instant::now()));
        self.modules
            .sort_by_cached_key(|(module, _, _)| module.position())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Set a minimum threshold to check for updates.
            let mut threshold = self.config.threshold;
            for (module, _, _) in &self.modules {
                if let Some(module_threshold) = module.update_threshold().await {
                    if module_threshold < threshold {
                        threshold = module_threshold;
                    }
                }
            }

            // Has the working directory changed.
            let wd_changed = env::current_dir().ok() == self.last_wd;

            tokio::select! {
                // Received update request.
                Some(force) = self.update_req_rx.recv() => {
                    // Update prompt.
                    self.update(force, wd_changed).await;
                }
                // Check for modules to update.
                _ = delay_for(threshold) => {
                    let late_update = check_module_updates(self.modules.as_mut(), wd_changed).await;
                    if late_update {
                        // TODO: Check performance of using second loop here,
                        //       instead of computing prompt in `check_module_updates`.

                        // Update prompt.
                        let mut prompt = Text::EMPTY;
                        for (_, cached, _) in &self.modules {
                            if let Some(cached) = cached {
                                push_module_text(&mut prompt, cached);
                            }
                        }
                        self.set_prompt(prompt).await;

                        // Send late update.
                        self.late_updates_tx.send(()).await?;
                    }
                }
            }
        }
    }

    async fn set_prompt(&mut self, prompt: Text) {
        let mut last_prompt = self.last_prompt.write().await;
        *last_prompt = Arc::new(prompt);
    }

    async fn update(&mut self, force: bool, wd_changed: bool) {
        let mut prompt = Text::EMPTY;

        for (module, cached, last_update) in &mut self.modules {
            // Check if module should be updated.
            if force || module.should_update(wd_changed).await {
                update_module(module, cached, last_update).await;
            }

            if let Some(cached) = cached {
                push_module_text(&mut prompt, cached);
            }
        }

        self.set_prompt(prompt).await;
    }
}

fn push_module_text(prompt: &mut Text, text: &Text) {
    let prompt_len = prompt
        .iter()
        .map(|s| s.text.len())
        .fold(0usize, std::ops::Add::add);

    if prompt_len > 0 {
        prompt.push(TextSegment::plain(" "));
    }

    prompt.extend(text);
}

async fn update_module(
    module: &mut Box<dyn PromptModule>,
    cached: &mut Option<Text>,
    last_update: &mut Instant,
) {
    // Compute module.
    let computed = module.compute().await;

    *cached = computed;
    *last_update = Instant::now();
}

async fn check_module_update_threshold(
    module: &mut Box<dyn PromptModule>,
    cached: &mut Option<Text>,
    last_update: &mut Instant,
    wd_changed: bool,
) -> bool {
    let should_update = tokio::select! {
        // Reached update threshold.
        Some(threshold) = module.update_threshold() => last_update.elapsed() > threshold,
        // Module told us it should be updated.
        true = module.should_update(wd_changed) => true,
        // Otherwise leave it as it is.
        else => false,
    };

    if should_update {
        update_module(module, cached, last_update).await;
    }
    should_update
}

async fn check_module_updates(modules: &mut [ModuleEntry], wd_changed: bool) -> bool {
    let mut late_update = false;

    for (module, cached, last_update) in modules {
        late_update |= check_module_update_threshold(module, cached, last_update, wd_changed).await;
    }

    late_update
}
