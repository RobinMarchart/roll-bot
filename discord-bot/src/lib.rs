use bot_utils::{Bot, ClientUtils, GlobalUtils};

use serenity::client::ClientBuilder;

use async_trait::async_trait;
use std::sync::Arc;

pub struct DiscordBot {
    config: DiscordBotConfig,
}

impl DiscordBot {
    pub fn new() -> Box<DiscordBot> {
        Box::new(DiscordBot {
            config: DiscordBotConfig::default(),
        })
    }
}

mod handler;
use handler::DiscordBotHandler;

struct DiscordBotConfig {
    invite_url: String,
}
impl Default for DiscordBotConfig {
    fn default() -> Self {
        DiscordBotConfig {
            invite_url: "https://example.com".to_string(),
        }
    }
}

#[async_trait]
impl Bot for DiscordBot {
    async fn run(&self, utils: Arc<GlobalUtils>) {
        let token = std::env::var("DISCORD_TOKEN")
            .expect("No Discord API Token provided in DISCORD_TOKEN env var");
        let handler = DiscordBotHandler {
            guild_utils: ClientUtils::new(utils.clone(), "discord_guilds")
                .await
                .expect("error creating guild storage"),
            dm_utils: ClientUtils::new(utils, "discord_dm")
                .await
                .expect("error creating dm storage"),
        };

        let mut client = ClientBuilder::new(token)
            .raw_event_handler(handler)
            .await
            .expect("Error creating Client");
        log::info!("created client");
        client.start().await.expect("Client error")
    }

    fn config(&mut self, config: &mut std::collections::HashMap<String, toml::Value>) -> bool {
        match config.get_mut("discord").and_then(|d| d.as_table_mut()) {
            Some(table) => match table.get("invite-url").and_then(|i| i.as_str()) {
                Some(url) => {
                    self.config.invite_url = url.to_string();
                    false
                }
                None => {
                    table.insert(
                        "invite-url".to_string(),
                        toml::Value::from(self.config.invite_url.clone()),
                    );
                    true
                }
            },
            None => {
                let mut discord = std::collections::HashMap::<String, toml::Value>::new();
                discord.insert(
                    "invite-url".to_string(),
                    toml::Value::from(self.config.invite_url.clone()),
                );
                config.insert("discord".to_string(), toml::Value::from(discord));
                true
            }
        }
    }
}
