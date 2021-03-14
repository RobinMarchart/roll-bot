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
use std::sync::Arc;
struct Bots {
    utils: Arc<GlobalUtils>,
}

impl Bots {
    async fn new() -> Bots {
        Bots {
            utils: Arc::new(
                GlobalUtils::new(
                    std::path::PathBuf::from(
                        std::env::args_os()
                            .skip(1)
                            .next()
                            .expect("missing first command line argument with storage path")
                            .as_os_str(),
                    )
                    .into_boxed_path(),
                    std::time::Duration::from_secs(2),
                    4,
                    std::time::Duration::from_secs(300),
                )
                .await,
            ),
        }
    }

    async fn run(self, bots: Vec<Box<dyn Bot + Send + Sync>>) {
        let barrier = Arc::new(tokio::sync::Barrier::new(bots.len() + 1));
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
