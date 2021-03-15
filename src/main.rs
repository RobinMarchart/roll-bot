#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    pretty_env_logger::init();
    log::info!("logger created");
    Bots::new()
        .await
        .run(vec![discord_bot::DiscordBot::new()])
        .await;
}

use bot_utils::{Bot, GlobalUtils};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use toml::Value;

struct Bots {
    utils: Arc<GlobalUtils>,
    config: HashMap<String, Value>,
    base_path: PathBuf,
    config_write: bool,
}

impl Bots {
    async fn new() -> Bots {
        let base_path = PathBuf::from(
            std::env::args_os()
                .skip(1)
                .next()
                .expect("missing first command line argument with storage path")
                .as_os_str(),
        );

        let mut config_write = false;
        let config_path = base_path.join("config.toml");

        let mut config: HashMap<String, Value> = match std::fs::read(&config_path) {
            Ok(content) => match toml::from_slice(&content) {
                Ok(value) => value,
                Err(err) => {
                    log::warn!("unable to parse config at {:?}: {}", &config_path, err);
                    config_write = true;
                    std::collections::HashMap::<String, toml::Value>::new()
                }
            },
            Err(err) => {
                log::warn!("unable to open {:?}: {}", &config_path, err);
                config_write = true;
                std::collections::HashMap::<String, toml::Value>::new()
            }
        };

        let roll_timeout = match config
            .get("roll-timeout")
            .and_then(|t| t.as_integer())
            .and_then(|i| u64::try_from(i).ok())
        {
            Some(timeout) => Duration::from_millis(timeout),
            None => {
                log::warn!("missing or unusable roll timeout");
                config_write = true;
                config.insert("roll-timeout".to_string(), Value::from(2000));
                Duration::from_secs(2)
            }
        };
        log::info!("Using roll timeout of {:?}", roll_timeout);
        let rng_reseed = match config
            .get("rng-reseed")
            .and_then(|t| t.as_integer())
            .and_then(|i| u64::try_from(i).ok())
        {
            Some(time) => Duration::from_secs(time),
            None => {
                log::warn!("missing or unusable rng reseed");
                config_write = true;
                config.insert("rng-reseed".to_string(), Value::Integer(300));
                Duration::from_secs(300)
            }
        };
        log::info!("Using rng reseed of {:?}", rng_reseed);

        Bots {
            utils: Arc::new(GlobalUtils::new(base_path.clone(), roll_timeout, 4, rng_reseed).await),
            config,
            base_path,
            config_write,
        }
    }

    async fn run(mut self, mut bots: Vec<Box<dyn Bot + Send + Sync>>) {
        let write = bots
            .iter_mut()
            .map(|b| b.config(&mut self.config))
            .reduce(|a, b| a | b)
            .unwrap_or(false)
            | self.config_write;
        let barrier = Arc::new(tokio::sync::Barrier::new(bots.len() + 1));
        if write {
            match std::fs::write(
                self.base_path.join("config.toml"),
                &toml::to_vec(&self.config).unwrap_or(vec![]),
            ) {
                Ok(_) => {}
                Err(err) => {
                    log::warn!("Unable to write config: {}", err);
                }
            }
        }
        bots.into_iter()
            .for_each(|bot: std::boxed::Box<dyn Bot + Send + Sync>| {
                let utils_clone = self.utils.clone();
                let barrier_clone = barrier.clone();
                tokio::task::spawn(async move {
                    bot.run(utils_clone).await;
                    barrier_clone.wait().await;
                });
            });

        barrier.wait().await;
    }
}
