use async_trait::async_trait;
use bot_utils::ClientUtils;
use serenity::{
    model::{
        channel::Message,
        event::Event,
        id::{GuildId, UserId},
    },
    prelude::RawEventHandler,
};

pub(crate) struct DiscordBotHandler {
    pub guild_utils: ClientUtils<GuildId>,
    pub dm_utils: ClientUtils<UserId>,
}
#[async_trait]
impl RawEventHandler for DiscordBotHandler {
    async fn raw_event(&self, ctx: serenity::client::Context, ev: Event) {
        match ev {
            Event::MessageCreate(event) => {
                if event.message.author.bot {
                } else if let Some(guild) = event.message.guild_id {
                    if let Some(response) = self
                        .guild_utils
                        .eval(guild.clone(), &event.message.content, || {
                            check_priviledged_access(&ctx, &event.message)
                        })
                        .await
                    {
                        respond(ctx, event.message, response).await;
                    }
                } else {
                    if let Some(response) = self
                        .dm_utils
                        .eval(
                            event.message.author.id.clone(),
                            &event.message.content,
                            || std::future::ready(true),
                        )
                        .await
                    {
                        respond(ctx, event.message, response).await;
                    }
                }
            }
            _ => {}
        }
    }
}

async fn check_priviledged_access(context: &serenity::client::Context, message: &Message) -> bool {
    match message.guild_id {
        Some(guild) => match guild.to_partial_guild(&context).await {
            Ok(g) => {
                if g.owner_id == message.author.id {
                    true
                } else {
                    match g.member(&context, message.author.id.clone()).await {
                        Ok(member) => {
                            for roll in member
                                .roles
                                .iter()
                                .map(|id| g.roles.get(id))
                                .filter_map(|x| x)
                            {
                                if roll.permissions.administrator() {
                                    return true;
                                }
                            }
                            false
                        }
                        Err(err) => {
                            log::warn!("unable to get member {}: {}", &message.author.id, err);
                            false
                        }
                    }
                }
            }
            Err(err) => {
                log::warn!("unable to retrieve guild {}: {}", &guild, err);
                false
            }
        },
        None => true, //user is always allowed to run every command in dm channels
    }
}

use bot_utils::CommandResult;

mod help;
use help::help;
mod command_prefix;
use command_prefix::{get_command_prefix, set_command_prefix};
mod roll_prefix;
use roll_prefix::{add_roll_prefix, list_roll_prefix, remove_roll_prefix};
mod alias;
use alias::{add_alias, list_aliases, remove_alias};
mod roll;
use roll::roll;
mod permissions;
use permissions::insufficent_permissions;

async fn respond(
    context: serenity::client::Context,
    message: serenity::model::channel::Message,
    response: CommandResult,
) {
    match response {
        CommandResult::Help(prefix) => help(context, message, prefix).await,
        CommandResult::RollHelp => {}
        CommandResult::Info => {}
        CommandResult::SetCommandPrefix(prefix) => {
            set_command_prefix(context, message, prefix).await
        }
        CommandResult::GetCommandPrefix(prefix) => {
            get_command_prefix(context, message, prefix).await
        }
        CommandResult::AddRollPrefix(result) => add_roll_prefix(context, message, result).await,
        CommandResult::RemoveRollPrefix(result) => {
            remove_roll_prefix(context, message, result).await
        }
        CommandResult::ListRollPrefix(prefixes) => {
            list_roll_prefix(context, message, prefixes).await
        }
        CommandResult::AddAlias => add_alias(context, message).await,
        CommandResult::RemoveAlias(result) => remove_alias(context, message, result).await,
        CommandResult::ListAliases(aliases) => list_aliases(context, message, aliases).await,
        CommandResult::Roll(res, expr) => roll(context, message, res, expr).await,
        CommandResult::InsufficentPermission => insufficent_permissions(context, message).await,
    }
}
