use bot_utils::bot_manager::BotManagerBuilder;
use discord_bot::DiscordBotConfig;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    pretty_env_logger::init();
    log::info!("logger created");
    let config_path = std::env::args()
        .skip(1)
        .next()
        .expect("missing first command line argument with config file");
    BotManagerBuilder::new(config_path, DiscordBotConfig {})
        .build_async()
        .await
        .run()
        .await;
}
