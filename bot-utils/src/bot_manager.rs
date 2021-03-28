use crate::client_utils::{rolls::RollExecutor, storage::GlobalStorage, ClientUtilsBuilder};
use crate::tuple_helpers::*;
pub use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};
use tokio::join;

pub struct BotManager<B: BotWrapper> {
    global_handle: ClientUtilsBuilder,
    bots: B,
}

impl<B: BotWrapper> BotManager<B> {
    pub async fn run(self) {
        let (_, r) = join!(self.global_handle.wait(), self.bots.run().join());
        ResultChain::result(r).unwrap();
    }
}

#[async_trait]
pub trait StopListener: Sized + Sync + Clone + Send + 'static {
    async fn wait_stop(&mut self) -> ();
}

#[async_trait]
impl StopListener for tokio::sync::watch::Receiver<bool> {
    async fn wait_stop(&mut self) -> () {
        loop {
            match self.changed().await {
                Err(_) => {
                    break;
                }
                Ok(_) => {
                    if *self.borrow() {
                        break;
                    }
                }
            }
        }
    }
}

pub struct BotManagerBuilder<BB: BotBuilderWrapper> {
    bots: BB,
    storage: GlobalStorage,
    roll_timeout: std::time::Duration,
    rng_reseed: std::time::Duration,
    rng_workers: u32,
    db_handle: std::thread::JoinHandle<()>,
}

#[cfg(target_family = "unix")]
async fn wait_hup() {
    use tokio::signal::unix::*;
    let mut signal = signal(SignalKind::hangup()).unwrap();
    signal.recv().await;
}

impl<BB: BotBuilderWrapper + Send> BotManagerBuilder<BB> {
    pub fn new<S, BC>(config_path: S, bots: BC) -> BotManagerBuilder<BB>
    where
        S: ToString,
        BC: BotConfigWrapper<Output = BB>,
    {
        use std::convert::TryInto;
        use toml::{map::Map, Value};
        let config_path = PathBuf::from(config_path.to_string());
        let mut config: Map<String, Value> =
            match toml::from_slice(&match std::fs::read(&config_path) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("Unable to read config file: {}", e);
                    vec![]
                }
            }) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("Unable to parse config: {}", e);
                    Map::new()
                }
            };
        let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
            config
                .get("db_path")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string())
                .expect("No db_path given through \"DB_PATH\" env var or \"db_path\" config key")
        });

        let db_queue_size: usize = match config
            .get("db_queue_size")
            .and_then(|s| s.as_integer())
            .and_then(|s| s.try_into().ok())
        {
            Some(i) => i,
            None => {
                log::warn!("unable to read db_queue_size, overwriting with 64");
                config.insert("db_queue_size".to_string(), Value::from(64));
                64
            }
        };
        let roll_timeout: std::time::Duration = std::time::Duration::from_millis(
            match config
                .get("roll_timeout_ms")
                .and_then(|t| t.as_integer())
                .and_then(|t| t.try_into().ok())
            {
                Some(t) => t,
                None => {
                    log::warn!("unable to read roll_timeout_ms, overwriting with 2000");
                    config.insert("roll_timeout_ms".to_string(), Value::from(2000));
                    2000
                }
            },
        );
        let rng_reseed = std::time::Duration::from_secs(
            match config
                .get("rng_reseed_s")
                .and_then(|t| t.as_integer())
                .and_then(|t| t.try_into().ok())
            {
                Some(t) => t,
                None => {
                    log::warn!("unable to read rng_reseed_s, overwriting with 300");
                    config.insert("rng_reseed_s".to_string(), toml::Value::from(300));
                    300
                }
            },
        );
        let rng_workers: u32 = match config
            .get("rng_workers")
            .and_then(|t| t.as_integer())
            .and_then(|t| t.try_into().ok())
        {
            Some(t) => t,
            None => {
                log::warn!("unable to read rng_workers, overwriting with 4");
                config.insert("rng_workers".to_string(), toml::Value::from(4));
                4
            }
        };

        let builders: BB = bots.config(&mut config);

        let (storage, db_handle) = GlobalStorage::new(db_path, db_queue_size).unwrap();

        match std::fs::write(config_path, toml::to_vec(&config).unwrap()) {
            Ok(_) => {}
            Err(e) => {
                log::error!("Error writing config: {}", e)
            }
        }

        BotManagerBuilder {
            bots: builders,
            storage,
            roll_timeout,
            rng_reseed,
            rng_workers,
            db_handle,
        }
    }

    pub async fn build_async(
        self,
    ) -> BotManager<
        <<BB::Output as JoinChain>::Output as ResultChain<tokio::task::JoinError>>::Output,
    > {
        let (finished_sender, finished_receiver) = tokio::sync::watch::channel(false);
        tokio::task::spawn(async move {
            #[cfg(target_family = "unix")]
            {
                tokio::select! {
                    _ = tokio::signal::ctrl_c()=>{
                        log::info!("Received Ctrl-C: Shutting down")
                    }
                    _ = wait_hup()=>{
                        log::info!("Received SIGHUP: Shutting down")
                    }
                };
            }
            #[cfg(not(target_family = "unix"))]
            {
                match tokio::signal::ctrl_c().await {
                    _ => log::info!("Received Ctrl-C: Shutting down"),
                }
            }
            match finished_sender.send(true) {
                _ => {}
            }
        });
        let (handle, roll) = RollExecutor::new(
            self.rng_workers,
            self.roll_timeout,
            self.rng_reseed,
            finished_receiver.clone(),
        )
        .await;
        let db_handle_task = self.db_handle;
        let db_handle = tokio::task::spawn_blocking(move || db_handle_task.join().unwrap());
        let bot_config_builder = Arc::new(std::sync::Mutex::new(ClientUtilsBuilder {
            rolls: std::sync::Arc::new(roll),
            storage: std::sync::Arc::new(self.storage),
            join_handles: vec![handle, db_handle],
        }));
        let bots: <<BB::Output as JoinChain>::Output as ResultChain<tokio::task::JoinError>>::Output = ResultChain::result(
            JoinChain::join(BotBuilderWrapper::build(
                self.bots,
                bot_config_builder.clone(),
                finished_receiver,
            ))
            .await,
        )
        .unwrap();
        BotManager {
            global_handle: match Arc::try_unwrap(bot_config_builder) {
                Ok(a) => a,
                Err(_) =>panic!("bot_config_builder is still owned somewhere after building bots should be finished") ,
            }
                .into_inner()
                .unwrap(),
            bots,
        }
    }
}
