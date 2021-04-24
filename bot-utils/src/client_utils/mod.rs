pub use robins_dice_roll::dice_roll::{EvaluationErrors, ExpressionEvaluate};

pub mod commands;
pub mod rolls;
pub mod storage;

use rolls::RollExecutor;
use serde::{Deserialize, Serialize};
use std::{future::Future, sync::Arc};
pub use storage::ClientId;
use storage::{GlobalStorage, StorageHandle};
use tokio::task::JoinHandle;

use robins_dice_roll::dice_types::{Expression, LabeledExpression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionedRollExpr {
    V1(Expression),
    V2(LabeledExpression),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RollExprResult {
    pub roll: Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>,
    pub text: String,
    pub label: Option<String>,
}

impl std::fmt::Display for VersionedRollExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionedRollExpr::V1(e) => {
                write!(f, "{}", e)
            }
            VersionedRollExpr::V2(e) => {
                write!(f, "{}", e)
            }
        }
    }
}

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
    Roll(Vec<RollExprResult>, bool),
    GetRollInfo(bool),
    SetRollInfo,
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
            Some((command, command_prefix, roll_info)) => Some(match command {
                commands::Command::Help => CommandResult::Help(command_prefix),
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
                commands::Command::GetCommandPrefix => {
                    CommandResult::GetCommandPrefix(command_prefix)
                }
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
                commands::Command::AliasRoll(expressions) => {
                    let mut rolls = Vec::with_capacity(expressions.len());
                    for expr in expressions {
                        rolls.push(self.roll.roll(expr).await);
                    }
                    CommandResult::Roll(rolls, roll_info)
                }
                commands::Command::Roll(expr) => {
                    CommandResult::Roll(vec![self.roll.roll(expr).await], roll_info)
                }
                commands::Command::SetRollInfo(new) => {
                    self.store.set_roll_info(id, new).await;
                    CommandResult::SetRollInfo
                }
                commands::Command::GetRollInfo => CommandResult::GetRollInfo(roll_info),
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
        log::info!("all client utils finished")
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
