use async_trait::async_trait;
use bot_utils::client_utils::ClientUtils;
use serenity::{
    model::{
        channel::Message,
        id::{GuildId, UserId},
    },
    prelude::EventHandler,
};

pub(crate) struct DiscordBotHandler {
    pub(crate) guild_utils: ClientUtils<GuildId>,
    pub(crate) dm_utils: ClientUtils<UserId>,
    pub(crate) invite_url: String,
}
#[async_trait]
impl EventHandler for DiscordBotHandler {
    async fn message(&self, ctx: serenity::client::Context, message: Message) {
        if message.author.bot {
        } else if let Some(guild) = message.guild_id {
            if let Some(response) = self
                .guild_utils
                .eval(guild.clone(), &message.content, || {
                    check_priviledged_access(&ctx, &message)
                })
                .await
            {
                respond(ctx, message, response).await;
            }
        } else {
            if let Some(response) = self
                .dm_utils
                .eval(message.author.id.clone(), &message.content, || {
                    std::future::ready(true)
                })
                .await
            {
                respond(ctx, message, response).await;
            }
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

use bot_utils::client_utils::CommandResult;

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
