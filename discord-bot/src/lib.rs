use bot_utils::{Bot, ClientUtils, CommandResult, GlobalUtils};

use serenity::{
    client::ClientBuilder,
    framework::Framework,
    model::{
        channel::{Channel, Message},
        id::{GuildId, UserId},
    },
};

use async_trait::async_trait;
use std::sync::Arc;

pub struct DiscordBot {}

impl DiscordBot {
    pub fn new() -> Box<DiscordBot> {
        Box::new(DiscordBot {})
    }
}

struct DiscordBotHandler {
    guild_utils: ClientUtils<GuildId>,
    dm_utils: ClientUtils<UserId>,
}

trait FoldFirstIterator: Iterator {
    fn fold_first<F: std::ops::FnMut(Self::Item, Self::Item) -> Self::Item>(
        self,
        f: F,
    ) -> Option<Self::Item>;
}

impl<I: Iterator + Sized> FoldFirstIterator for I {
    fn fold_first<F: std::ops::FnMut(Self::Item, Self::Item) -> Self::Item>(
        self,
        mut f: F,
    ) -> Option<Self::Item> {
        self.fold(None, |a, b| match a {
            Some(v) => Some(f(v, b)),
            None => Some(b),
        })
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
            .framework(handler)
            .await
            .expect("Error creating Client");
        log::info!("created client");
        client.start().await.expect("Client error")
    }
}

async fn respond(
    context: serenity::client::Context,
    message: serenity::model::channel::Message,
    response: CommandResult,
) {
    match response {
        CommandResult::Help => {}
        CommandResult::RollHelp => {}
        CommandResult::Info => {}
        CommandResult::SetCommandPrefix(prefix) => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
            if let Some(guild) = message.guild_id {
                if let Some(err) = guild
                    .edit_nickname(&context, Some(&format!("[{}] Roll Bot", prefix)))
                    .await
                    .err()
                {
                    log::warn!("Unable to change nickname in {}: {}", guild, err);
                }
            }
        }
        CommandResult::GetCommandPrefix(prefix) => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
            if let Err(err) = Message::reply(&message, &context, format!("`{}`", prefix)).await {
                log::warn!("Unable to reply to message: {}", err)
            }
        }
        CommandResult::AddRollPrefix(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::RemoveRollPrefix(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::ListRollPrefix(prefixes) => {
            if let Some(m) = prefixes
                .iter()
                .map(|p| format!("`{}`", p))
                .fold_first(|p1, p2| format!("{}\n{}", p1, p2))
            {
                if let Err(err) = Message::reply(&message, &context, m).await {
                    log::warn!("Unable to reply to message: {}", err)
                }
            }
        }
        CommandResult::AddAlias => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::RemoveAlias(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::ListAlias(alias) => {
            if let Some(m) = alias
                .iter()
                .map(|(alias, expr)| format!("`{}` => `{}`", alias, expr))
                .fold_first(|p1, p2| format!("{}\n{}", p1, p2))
            {
                if let Err(err) = Message::reply(&message, &context, m).await {
                    log::warn!("Unable to reply to message: {}", err)
                }
            }
        }
        CommandResult::Roll(res, expr) => {
            let mes = match res {
                Ok(r) => {
                    format!(
                        "{} => [{}]",
                        expr,
                        r.iter()
                            .map(|result| format!("`{}`", result.0))
                            .fold_first(|r1, r2| format!("{}, {}", r1, r2))
                            .unwrap_or(" ".to_string())
                    )
                }
                Err(e) => match e {
                    bot_utils::EvaluationErrors::DivideByZero => {
                        "*Division by 0 detected*".to_string()
                    }
                    bot_utils::EvaluationErrors::Timeout => "*Timeout*".to_string(),
                    bot_utils::EvaluationErrors::Overflow => "*Overflow detected*".to_string(),
                },
            };
            if let Err(err) = Message::reply(&message, &context, mes).await {
                log::warn!("Unable to reply to message: {}", err)
            }
        }
    }
}

#[async_trait]
impl Framework for DiscordBotHandler {
    async fn dispatch(
        &self,
        context: serenity::client::Context,
        message: serenity::model::channel::Message,
    ) {
        if let Some(guild) = message.guild_id {
            if let Some(response) = self
                .guild_utils
                .eval(guild.clone(), message.content.as_str())
                .await
            {
                log::info!("{:?}", &response);
                respond(context, message, response).await
            }
        } else if let Some(Channel::Private(dm_channel)) = message.channel(&context).await {
            if let Some(response) = self
                .dm_utils
                .eval(dm_channel.recipient.id.clone(), message.content.as_str())
                .await
            {
                log::info!("{:?}", &response);
                respond(context, message, response).await
            }
        }
    }
}
