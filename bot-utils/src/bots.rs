pub use crate::bot_manager::StopListener;
pub use crate::client_utils::{ClientId, ClientUtils, ClientUtilsBuilder};
pub use async_trait::async_trait;
use std::sync::{Arc, Mutex};
pub use toml::{map::Map, Value};

#[async_trait]
pub trait Bot: Send + 'static {
    async fn run(self);
}

#[async_trait]
pub trait BotBuilder: Send + 'static {
    type B: Bot + 'static;
    async fn build<S: StopListener>(
        self,
        utils: Arc<Mutex<ClientUtilsBuilder>>,
        stop: S,
    ) -> Self::B;
}

pub trait BotConfig {
    type Builder: BotBuilder;
    fn config(self, config: &mut Map<String, Value>) -> Self::Builder;
}
