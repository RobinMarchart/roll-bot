use super::{BotWrapper, JoinChain, ResultChain};
use crate::{
    bot_manager::StopListener,
    bots::{BotBuilder, ClientUtilsBuilder},
};
use std::sync::{Arc, Mutex};
use tokio::task::{spawn, JoinError, JoinHandle};

pub trait BotBuilderWrapper {
    type Bot: BotWrapper;
    type Result: ResultChain<JoinError, Output = Self::Bot>;
    type Output: JoinChain<Output = Self::Result>;
    fn build<S: StopListener>(self, utils: Arc<Mutex<ClientUtilsBuilder>>, stop: S)
        -> Self::Output;
}

impl<BB> BotBuilderWrapper for BB
where
    BB: BotBuilder,
{
    type Output = JoinHandle<BB::B>;

    fn build<S: StopListener>(
        self,
        utils: Arc<Mutex<ClientUtilsBuilder>>,
        stop: S,
    ) -> Self::Output {
        spawn(async move { self.build(utils, stop).await.unwrap() })
    }

    type Bot = BB::B;

    type Result = Result<BB::B, JoinError>;
}
