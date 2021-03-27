use super::BotBuilderWrapper;
use crate::bots::BotConfig;
use std::collections::HashMap;
use toml::Value;

pub trait BotConfigWrapper {
    type Output: BotBuilderWrapper;
    fn config(self, config: &mut HashMap<String, Value>) -> Self::Output;
}

impl<BC> BotConfigWrapper for BC
where
    BC: BotConfig,
{
    type Output = BC::Builder;

    fn config(self, config: &mut HashMap<String, Value>) -> Self::Output {
        self.config(config)
    }
}
