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
use std::path::PathBuf;
use std::sync::Arc;
use storage::StorageHandle;

pub struct ClientUtils<Id: storage::ClientId> {
    global: Arc<GlobalUtils>,
    store: StorageHandle<Id>,
}

pub use robins_dice_roll::dice_roll::EvaluationErrors;

#[derive(Debug, PartialEq, Eq)]
pub enum CommandResult {
    Help(String),
    RollHelp,
    Info,
    SetCommandPrefix(String),
    GetCommandPrefix(String),
    AddRollPrefix(Result<(), ()>),
    RemoveRollPrefix(Result<(), ()>),
    ListRollPrefix(Vec<String>),
    AddAlias,
    RemoveAlias(Result<(), ()>),
    ListAliases(Vec<(String, String)>),
    Roll(Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>, String),
    InsufficentPermission,
}

impl<Id: storage::ClientId> ClientUtils<Id> {
    pub async fn new(global: Arc<GlobalUtils>, name: &str) -> std::io::Result<ClientUtils<Id>> {
        let store: StorageHandle<Id> =
            storage::StorageHandle::new(global.base_path.join(name).into_boxed_path()).await?;
        Ok(ClientUtils { global, store })
    }
    pub async fn eval<F: std::future::Future<Output = bool>, Fn: std::ops::FnOnce() -> F>(
        &self,
        id: Id,
        message: &str,
        check_permission: Fn,
    ) -> Option<CommandResult> {
        match commands::parse_logging(message, id.clone(), &self.store).await {
            Some(command) => Some(match command.0 {
                commands::Command::Help => CommandResult::Help(command.1),
                commands::Command::RollHelp => CommandResult::RollHelp,
                commands::Command::Info => CommandResult::Info,
                commands::Command::SetCommandPrefix(prefix) => {
                    if check_permission().await {
                        self.store.set_command_prefix(id, prefix.clone()).await;
                        CommandResult::SetCommandPrefix(prefix)
                    } else {
                        CommandResult::InsufficentPermission
                    }
                }
                commands::Command::GetCommandPrefix => CommandResult::GetCommandPrefix(command.1),
                commands::Command::AddRollPrefix(prefix) => {
                    if check_permission().await {
                        CommandResult::AddRollPrefix(self.store.add_roll_prefix(id, prefix).await)
                    } else {
                        CommandResult::InsufficentPermission
                    }
                }
                commands::Command::RemoveRollPrefix(prefix) => {
                    if check_permission().await {
                        CommandResult::RemoveRollPrefix(
                            self.store.remove_roll_prefix(id, prefix).await,
                        )
                    } else {
                        CommandResult::InsufficentPermission
                    }
                }
                commands::Command::ListRollPrefix => {
                    CommandResult::ListRollPrefix(self.store.get_roll_prefixes(id).await)
                }
                commands::Command::AddAlias(alias, expression) => {
                    if check_permission().await {
                        self.store.set_alias(id, alias, expression).await;
                        CommandResult::AddAlias
                    } else {
                        CommandResult::InsufficentPermission
                    }
                }
                commands::Command::RemoveAlias(alias) => {
                    if check_permission().await {
                        CommandResult::RemoveAlias(self.store.remove_alias(id, alias).await)
                    } else {
                        CommandResult::InsufficentPermission
                    }
                }
                commands::Command::ListAliases => CommandResult::ListAliases(
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

impl GlobalUtils {
    pub async fn new(
        path: PathBuf,
        roll_timeout: std::time::Duration,
        roll_workers: u32,
        rng_reseed: std::time::Duration,
    ) -> GlobalUtils {
        GlobalUtils {
            roller: rolls::RollExecutor::new(roll_workers, roll_timeout, rng_reseed).await,
            base_path: path,
        }
    }
}

pub struct GlobalUtils {
    pub roller: rolls::RollExecutor,
    base_path: PathBuf,
}

#[async_trait]
pub trait Bot {
    async fn run(&self, utils: Arc<GlobalUtils>);
    fn config(&mut self, config: &mut std::collections::HashMap<String, toml::Value>) -> bool;
}
