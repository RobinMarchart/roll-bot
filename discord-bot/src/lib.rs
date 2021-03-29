use bot_utils::{
    bots::{async_trait, Bot, BotBuilder, BotConfig, Map, Value},
    client_utils::{ClientUtilsBuilder, ClientUtilsConfig},
};

use serenity::client::{Client, ClientBuilder};

use std::sync::Arc;

pub struct DiscordBot {
    client: Client,
}

#[async_trait]
impl Bot for DiscordBot {
    async fn run(mut self) {
        self.client.start_autosharded().await.unwrap();
        log::info!("discord bot stopped")
    }
}

pub struct DiscordBotBuilder {
    invite_url: String,
    token: String,
    dm_utils: ClientUtilsConfig,
    guild_utils: ClientUtilsConfig,
}

#[async_trait]
impl BotBuilder for DiscordBotBuilder {
    type B = DiscordBot;

    async fn build<S: bot_utils::bot_manager::StopListener>(
        self,
        utils: Arc<std::sync::Mutex<ClientUtilsBuilder>>,
        mut stop: S,
    ) -> Self::B {
        let dm_utils = utils.lock().unwrap().get_from_config(self.dm_utils);
        let guild_utils = utils.lock().unwrap().get_from_config(self.guild_utils);
        let client = ClientBuilder::new(self.token)
            .event_handler(DiscordBotHandler {
                dm_utils,
                guild_utils,
                invite_url: self.invite_url,
            })
            .await
            .unwrap();
        let shard = client.shard_manager.clone();
        tokio::task::spawn(async move {
            stop.wait_stop().await;
            shard.lock().await.shutdown_all().await;
        });
        DiscordBot { client }
    }
}

pub struct DiscordBotConfig {}

impl BotConfig for DiscordBotConfig {
    type Builder = DiscordBotBuilder;

    fn config(self, config: &mut bot_utils::bots::Map<String, toml::Value>) -> Self::Builder {
        let token = std::env::var("DISCORD_TOKEN")
            .expect("No Discord API Token provided in DISCORD_TOKEN env var");
        let discord_config = match config.get_mut("discord").and_then(|d| d.as_table_mut()) {
            Some(d) => d,
            None => {
                log::warn!("Missing discord section in config");
                config.insert("discord".to_string(), Value::from(Map::new()));
                config.get_mut("discord").unwrap().as_table_mut().unwrap()
            }
        };
        let invite_url = match discord_config
            .get("invite_url")
            .and_then(|u| u.as_str())
            .map(|u| u.to_owned())
        {
            Some(u) => u,
            None => {
                log::warn!("Unable to read discord invite url!");
                discord_config.insert(
                    "invite_url".to_string(),
                    Value::from("https://example.com".to_string()),
                );
                "https://example.com".to_string()
            }
        };
        let dm_utils = ClientUtilsConfig::from_config(
            "discord-dm",
            match discord_config.get_mut("dm").and_then(|c| c.as_table_mut()) {
                Some(t) => t,
                None => {
                    discord_config.insert("dm".to_string(), Value::from(Map::new()));
                    discord_config
                        .get_mut("dm")
                        .unwrap()
                        .as_table_mut()
                        .unwrap()
                }
            },
        );
        let guild_utils = ClientUtilsConfig::from_config(
            "discord-guild",
            match discord_config
                .get_mut("guild")
                .and_then(|c| c.as_table_mut())
            {
                Some(t) => t,
                None => {
                    discord_config.insert("guild".to_string(), Value::from(Map::new()));
                    discord_config
                        .get_mut("guild")
                        .unwrap()
                        .as_table_mut()
                        .unwrap()
                }
            },
        );
        DiscordBotBuilder {
            invite_url,
            token,
            dm_utils,
            guild_utils,
        }
    }
}

mod handler;
use handler::DiscordBotHandler;
