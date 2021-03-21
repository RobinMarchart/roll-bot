/*
 *  Copyright 2021 Robin Marchart
 *
 *     Licensed under the Apache License, Version 2.0 (the "License");
 *     you may not use this file except in compliance with the License.
 *     You may obtain a copy of the License at
 *
 *         http://www.apache.org/licenses/LICENSE-2.0
 *
 *     Unless required by applicable law or agreed to in writing, software
 *     distributed under the License is distributed on an "AS IS" BASIS,
 *     WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *     See the License for the specific language governing permissions and
 *     limitations under the License.
 */

use diesel::prelude::*;
use robins_dice_roll::dice_types::Expression;
use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Serialize,
};
use std::hash::Hash;
use std::sync::Arc;
use std::{collections::HashMap, fmt};
use tokio::{
    sync::{mpsc, oneshot},
    task::spawn,
};
mod schema;
use cached::{Cached, SizedCache};
mod cc {
    use super::schema::client_config;
    #[derive(Debug, Queryable, Clone, Identifiable, Insertable)]
    #[table_name = "client_config"]
    pub(crate) struct ClientConfig {
        pub(crate) id: String,
        pub(crate) command_prefix: String,
        pub(crate) roll_prefix: String,
        pub(crate) aliases: String,
        pub(crate) roll_info: bool,
    }
    impl ClientConfig {
        pub(crate) fn new(id: String) -> ClientConfig {
            ClientConfig {
                id,
                command_prefix: "rrb!".to_string(),
                roll_prefix: "[]".to_string(),
                aliases: "{}".to_string(),
                roll_info: false,
            }
        }
    }

    #[derive(Debug, AsChangeset)]
    #[table_name = "client_config"]
    pub(crate) struct ClientConfigChangeset {
        command_prefix: Option<String>,
        roll_prefix: Option<String>,
        aliases: Option<String>,
        roll_info: Option<bool>,
    }
}

use cc::{ClientConfig, ClientConfigChangeset};
pub trait ClientId:
    Serialize + DeserializeOwned + Eq + fmt::Debug + Hash + Clone + Send + Sync + Sized + 'static
{
}

impl<
        Id: Serialize
            + DeserializeOwned
            + Eq
            + fmt::Debug
            + Hash
            + Clone
            + Send
            + Sized
            + Sync
            + 'static,
    > ClientId for Id
{
}

#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone)]
struct Client<Id: ClientId> {
    client_type: Arc<String>,
    client_id: Id,
}

#[derive(Debug, Clone)]
struct ClientInformation {
    source: ClientConfig,
    roll_prefix: Vec<String>,
    aliases: HashMap<String, Arc<Expression>>,
    command_prefix_changed: bool,
    roll_prefix_changed: bool,
    aliases_changed: bool,
    roll_info_changed: bool,
}

impl ClientInformation {
    fn new(source: ClientConfig) -> ClientInformation {
        let mut roll_prefix_changed = false;
        let roll_prefix = match serde_json::from_str(&source.roll_prefix) {
            Ok(p) => p,
            Err(err) => {
                log::warn!(
                    "unable to parse roll prefixes from {}: {}",
                    &source.roll_prefix,
                    err
                );
                roll_prefix_changed = true;
                vec![]
            }
        };
        let mut aliases_changed = false;
        let aliases = match serde_json::from_str(&source.aliases) {
            Ok(a) => a,
            Err(err) => {
                log::warn!("unable to parse aliases from {}: {}", &source.aliases, err);
                aliases_changed = true;
                HashMap::new()
            }
        };
        ClientInformation {
            source,
            roll_prefix,
            aliases,
            command_prefix_changed: false,
            roll_prefix_changed,
            aliases_changed,
            roll_info_changed: false,
        }
    }

    fn get_cmd_prefix(&self) -> &str {
        &self.source.command_prefix
    }
    fn get_cmd_prefix_mut(&mut self) -> &mut str {
        self.command_prefix_changed = true;
        &mut self.source.command_prefix
    }
    fn get_roll_prefix(&self) -> &[String] {
        &self.roll_prefix
    }
    fn get_roll_prefix_mut(&mut self) -> &mut [String] {
        self.roll_prefix_changed = true;
        &mut self.roll_prefix
    }
    fn get_aliases(&self) -> &HashMap<String, Arc<Expression>> {
        &self.aliases
    }
    fn get_aliases_mut(&mut self) -> &mut HashMap<String, Arc<Expression>> {
        self.aliases_changed = true;
        &mut self.aliases
    }
    fn get_roll_info(&self) -> bool {
        self.source.roll_info.clone()
    }
    fn get_roll_info_mut(&mut self) -> &mut bool {
        self.roll_info_changed = true;
        &mut self.source.roll_info
    }
}

#[derive(Debug)]
enum StorageOps<Id: ClientId> {
    SetClientInfo(Id, ClientInformation),
    GetCommandPrefix(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        oneshot::Sender<String>,
    ),
    SetCommandPrefix(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        oneshot::Sender<()>,
    ),
    GetRollPrefixes(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        oneshot::Sender<Vec<String>>,
    ),
    AddRollPrefix(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        oneshot::Sender<Result<(), ()>>,
    ),
    RemoveRollPrefix(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        oneshot::Sender<Result<(), ()>>,
    ),
    GetAllAlias(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        oneshot::Sender<HashMap<String, Arc<Expression>>>,
    ),
    GetAlias(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        oneshot::Sender<Option<Arc<Expression>>>,
    ),
    AddAlias(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        Expression,
        oneshot::Sender<()>,
    ),
    RemoveAlias(
        Id,
        mpsc::UnboundedSender<StorageOps<Id>>,
        String,
        oneshot::Sender<Result<(), ()>>,
    ),
}

pub(crate) struct GlobalStorage {
    db: SqliteConnection,
}

impl GlobalStorage {
    pub(crate) fn new(db_url: &str) -> diesel::ConnectionResult<GlobalStorage> {
        Ok(GlobalStorage {
            db: SqliteConnection::establish(db_url)?,
        })
    }
    fn query(&self, client_id: &str) -> Option<ClientConfig> {
        use schema::client_config::dsl::*;
        match client_config.find(client_id).first(&self.db) {
            Ok(v) => Some(v),
            Err(err) => {
                log::info!("Error getting {} from db: {}", client_id, err);
                None
            }
        }
    }
    fn get(&self, client_id: String) -> ClientConfig {
        self.query(&client_id).unwrap_or_else(|| {
            use schema::client_config::dsl::*;
            let conf = ClientConfig::new(client_id);
            match diesel::insert_into(client_config)
                .values(&conf)
                .execute(&self.db)
            {
                Ok(_) => {}
                Err(err) => {
                    log::warn!("{}", err);
                }
            };
            conf
        })
    }
    fn set(&self, config: ClientInformation) {
        use schema::client_config::dsl::*;
        match diesel::update(schema::client_config::table)
            .set(&ClientConfigChangeset {
                command_prefix: if config.command_prefix_changed {
                    Some(config.source.command_prefix)
                } else {
                    None
                },
                aliases: if config.aliases_changed {
                    Some(serde_json::to_string(&config.aliases).unwrap_or("{}".to_string()))
                } else {
                    None
                },
                roll_prefix: if config.roll_prefix_changed {
                    Some(serde_json::to_string(&config.roll_prefix).unwrap_or("[]".to_string()))
                } else {
                    None
                },
                roll_info: if config.roll_info_changed {
                    Some(config.source.roll_info)
                } else {
                    None
                },
            })
            .execute(&self.db)
        {
            Ok(_) => {}
            Err(err) => {
                log::warn!("{}", err);
            }
        };
    }
}

pub struct StorageHandle<Id: ClientId> {
    client_type: Arc<String>,
    db_cache: SizedCache<Id, ClientInformation>,
    query_cache: HashMap<Id, Vec<StorageOps<Id>>>,
    global: Arc<GlobalStorage>,
}
