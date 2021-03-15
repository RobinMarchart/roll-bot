pub(crate) async fn set_command_prefix(
    context: serenity::client::Context,
    message: Message,
    prefix: std::string::String,
) {
    if let Err(err) = Message::react(&message, &context, '✅').await {
        log::warn!("unable to react to message {}: {}", message.id, err)
    }
    if let Some(guild) = message.guild_id {
        let mut nickname = format!("[{}] Robins Roll Bot", &prefix);
        if nickname.len() > 32 {
            nickname = format!("[{}] Roll Bot", &prefix);
            if nickname.len() > 32 {
                nickname = format!("[{}] Roll", &prefix);
                if nickname.len() > 32 {
                    nickname = format!("[{}]", &prefix);
                    if nickname.len() > 32 {
                        nickname = "Robins Roll Bot".to_string();
                    }
                }
            }
        }
        if let Some(err) = guild.edit_nickname(&context, Some(&nickname)).await.err() {
            log::warn!("Unable to change nickname in {}: {}", guild, err);
        }
    }
}
use serenity::model::channel::Message;

pub(crate) async fn get_command_prefix(
    context: serenity::client::Context,
    message: Message,
    prefix: std::string::String,
) {
    if let Err(err) = Message::react(&message, &context, '✅').await {
        log::warn!("unable to react to message {}: {}", message.id, err)
    }
    if let Err(err) = Message::reply(&message, &context, format!("`{}`", prefix)).await {
        log::warn!("Unable to reply to message {}: {}", message.id, err)
    }
}
