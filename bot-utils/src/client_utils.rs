pub use robins_dice_roll::dice_roll::EvaluationErrors;

pub mod commands;
pub mod rolls;
pub mod storage;

use rolls::RollExecutor;
use std::{future::Future, sync::Arc};
pub use storage::ClientId;
use storage::{GlobalStorage, StorageHandle};
use tokio::task::JoinHandle;

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

#[derive(Clone)]
pub struct ClientUtils<Id: ClientId> {
    roll: Arc<RollExecutor>,
    store: StorageHandle<Id>,
}

impl<Id: storage::ClientId> ClientUtils<Id> {
    pub async fn eval<F: Future<Output = bool>, Fn: FnOnce() -> F>(
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
                        self.store.add_alias(id, alias, expression).await.unwrap();
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
                    CommandResult::Roll(self.roll.roll(expr).await, roll_str)
                }
                commands::Command::Roll(expr) => {
                    let roll_str = format!("{}", &expr);
                    CommandResult::Roll(self.roll.roll(expr).await, roll_str)
                }
            }),
            None => None,
        }
    }
}
pub struct ClientUtilsBuilder {
    pub(crate) rolls: Arc<RollExecutor>,
    pub(crate) storage: Arc<GlobalStorage>,
    pub(crate) join_handles: Vec<JoinHandle<()>>,
}

use std::convert::TryInto;
use toml::{map::Map, Value};

impl ClientUtilsBuilder {
    pub fn get<Id: ClientId, S: ToString>(
        &mut self,
        client_type: S,
        channel_size: usize,
        cache_size: usize,
    ) -> ClientUtils<Id> {
        let (storage, join) =
            StorageHandle::new(client_type, self.storage.clone(), channel_size, cache_size);
        self.join_handles.push(join);
        ClientUtils {
            roll: self.rolls.clone(),
            store: storage,
        }
    }
    pub fn get_from_config<Id: ClientId>(&mut self, config: ClientUtilsConfig) -> ClientUtils<Id> {
        self.get(config.client_type, config.channel_size, config.cache_size)
    }
    pub async fn wait(self) {
        let handles = self.join_handles;
        drop(self.storage);
        drop(self.rolls);
        for handle in handles.into_iter() {
            handle.await.unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientUtilsConfig {
    pub channel_size: usize,
    pub cache_size: usize,
    pub client_type: String,
}

impl ClientUtilsConfig {
    pub fn from_config<S: ToString>(
        client_type: S,
        config: &mut Map<String, Value>,
    ) -> ClientUtilsConfig {
        let client = client_type.to_string();
        let channel_size: usize = match config
            .get("queue_size")
            .and_then(|v| v.as_integer())
            .and_then(|i| i.try_into().ok())
        {
            Some(i) => i,
            None => {
                log::warn!(
                    "Unable to read queue_size for {}, using default of 64",
                    &client
                );
                config.insert("queue_size".to_string(), Value::from(64));
                64
            }
        };
        let cache_size: usize = match config
            .get("cache_size")
            .and_then(|v| v.as_integer())
            .and_then(|i| i.try_into().ok())
        {
            Some(i) => i,
            None => {
                log::warn!(
                    "Unable to read cache_size for {}, using default of 1024",
                    &client
                );
                config.insert("cache_size".to_string(), Value::from(1024));
                1024
            }
        };
        ClientUtilsConfig {
            channel_size,
            cache_size,
            client_type: client,
        }
    }
}
