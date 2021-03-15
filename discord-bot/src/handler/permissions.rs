use serenity::{client::Context, model::channel::Message};

pub(crate) async fn insufficent_permissions(context: Context, message: Message) {
    if let Err(err) = Message::react(&message, &context, '❌').await {
        log::warn!("unable to add reaction to message {}: {}", message.id, err);
    }
}
