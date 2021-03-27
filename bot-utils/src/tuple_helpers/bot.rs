use super::JoinChain;
use crate::bots::Bot;
use tokio::task::{spawn, JoinHandle};

pub trait BotWrapper: Send + 'static {
    type Output: JoinChain;
    fn run(self) -> Self::Output;
}

impl<B> BotWrapper for B
where
    B: Bot + 'static,
{
    type Output = JoinHandle<()>;

    fn run(self) -> Self::Output {
        spawn(async move { self.run().await })
    }
}
