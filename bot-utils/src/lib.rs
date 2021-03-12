/*
Copyright 2021 Robin Marchart

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

pub mod commands;
pub mod rolls;
pub mod storage;

use async_trait::async_trait;
use std::path::Path;
use storage::StorageHandle;

pub struct ClientUtils<'g, Id: storage::ClientId> {
    global: &'g GlobalUtils,
    store: StorageHandle<Id>,
}

pub enum CommandResult {
    Help,
    RollHelp,
    Info,
    SetCommandPrefix,
    GetCommandPrefix(String),
    AddRollPrefix(Result<(), ()>),
    RemoveRollPrefix(Result<(), ()>),
    ListRollPrefix(Vec<String>),
    AddAlias,
    RemoveAlias(Result<(), ()>),
    ListAlias(Vec<(String, String)>),
    Roll(
        Result<Vec<(i64, Vec<i64>)>, robins_dice_roll::dice_roll::EvaluationErrors>,
        String,
    ),
}

impl<'g, Id: storage::ClientId> ClientUtils<'g, Id> {
    pub async fn new(global: &'g GlobalUtils, name: &str) -> std::io::Result<ClientUtils<'g, Id>> {
        let store: StorageHandle<Id> =
            storage::StorageHandle::new(global.base_path.join(name).into_boxed_path()).await?;
        Ok(ClientUtils { global, store })
    }
    pub async fn eval(&self, id: Id, message: &str) -> Option<CommandResult> {
        match commands::parse(message, id.clone(), &self.store).await {
            Some(command) => Some(match command {
                commands::Command::Help => CommandResult::Help,
                commands::Command::RollHelp => CommandResult::RollHelp,
                commands::Command::Info => CommandResult::Info,
                commands::Command::SetCommandPrefix(prefix) => {
                    self.store.set_command_prefix(id, prefix).await;
                    CommandResult::SetCommandPrefix
                }
                commands::Command::GetCommandPrefix => {
                    CommandResult::GetCommandPrefix(self.store.get_command_prefix(id).await)
                }
                commands::Command::AddRollPrefix(prefix) => {
                    CommandResult::AddRollPrefix(self.store.add_roll_prefix(id, prefix).await)
                }
                commands::Command::RemoveRollPrefix(prefix) => {
                    CommandResult::RemoveRollPrefix(self.store.remove_roll_prefix(id, prefix).await)
                }
                commands::Command::ListRollPrefix => {
                    CommandResult::ListRollPrefix(self.store.get_roll_prefixes(id).await)
                }
                commands::Command::AddAlias(alias, expression) => {
                    self.store.set_alias(id, alias, expression).await;
                    CommandResult::AddAlias
                }
                commands::Command::RemoveAlias(alias) => {
                    CommandResult::RemoveAlias(self.store.remove_alias(id, alias).await)
                }
                commands::Command::ListAlias => CommandResult::ListAlias(
                    self.store
                        .get_all_alias(id)
                        .await
                        .into_iter()
                        .map(|(key, value)| (key, value.to_string()))
                        .collect(),
                ),
                commands::Command::AliasRoll(expr) => {
                    let roll_str = format!("{}", &expr);
                    CommandResult::Roll(self.global.roller.roll(expr).await, roll_str)
                }
                commands::Command::Roll(expr) => {
                    let roll_str = format!("{}", &expr);
                    CommandResult::Roll(self.global.roller.roll(expr).await, roll_str)
                }
            }),
            None => None,
        }
    }
}

pub struct GlobalUtils {
    roller: rolls::RollExecutor,
    base_path: Box<Path>,
}

#[async_trait]
pub trait Bot {
    async fn run(&self, utils: &GlobalUtils);
}
