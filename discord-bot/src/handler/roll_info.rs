use serenity::{client::Context, model::channel::Message};

pub(crate) async fn set_roll_info(context: Context, message: Message) {
    if let Err(err) = Message::react(&message, &context, '✅').await {
        log::warn!("unable to react to message {}: {}", message.id, err)
    }
}

pub(crate) async fn get_roll_info(context: Context, message: Message, roll_info: bool) {
    if let Err(err) = Message::reply(
        &message,
        &context,
        &format!(
            "extra roll info is set to `{}`",
            if roll_info { "on" } else { "off" }
        ),
    )
    .await
    {
        log::warn!("Unable to reply to message: {}", err)
    }
}
