use bot_utils::EvaluationErrors;
use serenity::{client::Context, model::channel::Message};

pub(crate) async fn roll(
    context: Context,
    message: Message,
    res: Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>,
    expr: String,
) {
    let mes = match res {
        Ok(r) => {
            format!(
                "{} => [{}]",
                expr,
                r.iter()
                    .map(|result| format!("`{}`", result.0))
                    .reduce(|r1, r2| format!("{}, {}", r1, r2))
                    .unwrap_or(" ".to_string())
            )
        }
        Err(e) => match e {
            bot_utils::EvaluationErrors::DivideByZero => "*Division by 0 detected*".to_string(),
            bot_utils::EvaluationErrors::Timeout => "*Timeout*".to_string(),
            bot_utils::EvaluationErrors::Overflow => "*Overflow detected*".to_string(),
        },
    };
    if let Err(err) = Message::reply(&message, &context, mes).await {
        log::warn!("Unable to reply to message: {}", err)
    }
}
