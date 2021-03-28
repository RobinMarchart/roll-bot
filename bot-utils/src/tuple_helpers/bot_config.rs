use super::BotBuilderWrapper;
use crate::bots::BotConfig;
use toml::{map::Map, Value};

pub trait BotConfigWrapper {
    type Output: BotBuilderWrapper;
    fn config(self, config: &mut Map<String, Value>) -> Self::Output;
}

impl<BC> BotConfigWrapper for BC
where
    BC: BotConfig,
{
    type Output = BC::Builder;

    fn config(self, config: &mut Map<String, Value>) -> Self::Output {
        self.config(config)
    }
}
