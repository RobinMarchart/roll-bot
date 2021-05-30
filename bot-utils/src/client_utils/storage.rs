/*
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
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::hash_map::RandomState, hash::BuildHasher, sync::Arc};
use std::{collections::HashMap, fmt};
use std::{hash::Hash, usize};
use tokio::{
    sync::{mpsc, oneshot},
    task::spawn,
};
mod schema;
use super::VersionedRollExpr;
use cached::{Cached, SizedCache};
use parking_lot::{Mutex, MutexGuard, RwLock};
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
        pub(crate) command_prefix: Option<String>,
        pub(crate) roll_prefix: Option<String>,
        pub(crate) aliases: Option<String>,
        pub(crate) roll_info: Option<bool>,
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
struct Client<'s, Id: ClientId> {
    client_type: &'s str,
    client_id: Id,
}

#[derive(Debug, Clone)]
struct ClientInformation {
    source: ClientConfig,
    roll_prefix: Vec<String>,
    aliases: HashMap<String, Arc<VersionedRollExpr>>,
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
    fn get_cmd_prefix_mut(&mut self) -> &mut String {
        self.command_prefix_changed = true;
        &mut self.source.command_prefix
    }
    fn get_roll_prefix(&self) -> &[String] {
        &self.roll_prefix
    }
    fn get_roll_prefix_mut(&mut self) -> &mut Vec<String> {
        self.roll_prefix_changed = true;
        &mut self.roll_prefix
    }
    fn get_aliases(&self) -> &HashMap<String, Arc<VersionedRollExpr>> {
        &self.aliases
    }
    fn get_aliases_mut(&mut self) -> &mut HashMap<String, Arc<VersionedRollExpr>> {
        self.aliases_changed = true;
        &mut self.aliases
    }
    fn get_roll_info(&self) -> bool {
        self.source.roll_info
    }
    fn get_roll_info_mut(&mut self) -> &mut bool {
        self.roll_info_changed = true;
        &mut self.source.roll_info
    }
}

#[derive(Debug)]
enum StorageOps {
    GetCommandPrefix(oneshot::Sender<String>),
    SetCommandPrefix(String, oneshot::Sender<()>),
    GetRollPrefixes(oneshot::Sender<Vec<String>>),
    AddRollPrefix(String, oneshot::Sender<Result<(), ()>>),
    RemoveRollPrefix(String, oneshot::Sender<Result<(), ()>>),
    GetAllAlias(oneshot::Sender<HashMap<String, Arc<VersionedRollExpr>>>),
    GetAlias(String, oneshot::Sender<Option<Arc<VersionedRollExpr>>>),
    AddAlias(String, VersionedRollExpr, oneshot::Sender<Result<(), ()>>),
    RemoveAlias(String, oneshot::Sender<Result<(), ()>>),
    GetRollInfo(oneshot::Sender<bool>),
    SetRollInfo(bool, oneshot::Sender<()>),
    Get(
        Vec<String>,
        oneshot::Sender<(String, Vec<String>, Vec<Arc<VersionedRollExpr>>, bool)>,
    ),
}

pub(crate) struct GlobalStorage {
    db_submit: mpsc::Sender<Box<dyn Send + FnOnce(&SqliteConnection)>>,
}

impl GlobalStorage {
    pub(crate) fn new(
        db_url: String,
        channel_size: usize,
    ) -> diesel::ConnectionResult<(GlobalStorage, std::thread::JoinHandle<()>)> {
        let (sender, mut receiver) = mpsc::channel(channel_size);
        Ok((
            GlobalStorage { db_submit: sender },
            std::thread::Builder::new()
                .name("db_worker".to_string())
                .spawn(move || loop {
                    let db = SqliteConnection::establish(&db_url).unwrap();
                    match receiver.blocking_recv() {
                        Some(f) => f(&db),
                        None => {
                            break log::info!("db worker queue closed");
                        }
                    }
                })
                .unwrap(),
        ))
    }
    async fn get<Id: ClientId>(
        &self,
        client_id: String,
        c_id: Id,
        sender: mpsc::UnboundedSender<(Id, ClientConfig)>,
    ) {
        match self
            .db_submit
            .send(Box::from(move |db: &SqliteConnection| {
                use schema::client_config::dsl::*;
                sender
                    .send((
                        c_id,
                        match client_config.find(&client_id).first(db) {
                            Ok(v) => v,
                            Err(err) => {
                                log::info!("Error getting {} from db: {}", &client_id, err);
                                let conf = ClientConfig::new(client_id);
                                match diesel::insert_into(client_config).values(&conf).execute(db) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::warn!("{}", err);
                                    }
                                };
                                conf
                            }
                        },
                    ))
                    .unwrap()
            }))
            .await
        {
            Ok(_) => {}
            Err(_) => panic!("unable to submit to db worker queue"),
        };
    }

    async fn set(&self, config: &mut ClientInformation) {
        let change = ClientConfigChangeset {
            command_prefix: if config.command_prefix_changed {
                config.command_prefix_changed = false;
                Some(config.source.command_prefix.to_string())
            } else {
                None
            },
            aliases: if config.aliases_changed {
                config.aliases_changed = false;
                Some(serde_json::to_string(&config.aliases).unwrap_or_else(|_| "{}".to_string()))
            } else {
                None
            },
            roll_prefix: if config.roll_prefix_changed {
                config.roll_prefix_changed = false;
                Some(
                    serde_json::to_string(&config.roll_prefix).unwrap_or_else(|_| "[]".to_string()),
                )
            } else {
                None
            },
            roll_info: if config.roll_info_changed {
                config.roll_info_changed = false;
                Some(config.source.roll_info)
            } else {
                None
            },
        };
        let id_clone = config.source.id.to_string();
        match self
            .db_submit
            .send(Box::new(move |db| {
                diesel::update(schema::client_config::dsl::client_config.find(&id_clone))
                    .set(change)
                    .execute(db)
                    .unwrap();
            }))
            .await
        {
            Ok(_) => {}
            Err(_) => panic!("unable to submit to db queue"),
        };
    }
}

struct ClientStorage<Id: ClientId, HB:BuildHasher+Default = RandomState> {
    client_type: String,
    db_cache: Arc<Vec<Mutex<SizedCache<Id, Arc<Mutex<ClientInformation>>>>>>,
    query_cache: Arc<RwLock<HashMap<Id, Mutex<Vec<StorageOps>>>>>,
    global: Arc<GlobalStorage>,
    hash_builder: HB,
}

fn run_cmd(client: &mut ClientInformation, op: StorageOps) -> bool {
    match op {
        StorageOps::GetCommandPrefix(channel) => {
            channel.send(client.get_cmd_prefix().to_owned()).unwrap();
            false
        }
        StorageOps::SetCommandPrefix(prefix, channel) => {
            *client.get_cmd_prefix_mut() = prefix;
            channel.send(()).unwrap();
            true
        }
        StorageOps::GetRollPrefixes(channel) => {
            channel.send(client.get_roll_prefix().to_owned()).unwrap();
            false
        }
        StorageOps::AddRollPrefix(prefix, channel) => {
            channel
                .send(if client.get_roll_prefix().contains(&prefix) {
                    Err(())
                } else {
                    client.get_roll_prefix_mut().push(prefix);
                    Ok(())
                })
                .unwrap();
            true
        }
        StorageOps::RemoveRollPrefix(prefix, channel) => {
            channel
                .send(
                    client
                        .get_roll_prefix()
                        .iter()
                        .position(|p| p == &prefix)
                        .map(|p| {
                            client.get_roll_prefix_mut().remove(p);
                        })
                        .ok_or(()),
                )
                .unwrap();
            true
        }
        StorageOps::GetAllAlias(channel) => {
            channel.send(client.get_aliases().to_owned()).unwrap();
            false
        }
        StorageOps::GetAlias(name, channel) => {
            channel
                .send(client.get_aliases().get(&name).map(|a| a.to_owned()))
                .unwrap();
            false
        }
        StorageOps::AddAlias(alias, expr, channel) => {
            let expression = Arc::from(expr);
            channel
                .send(
                    match client.get_aliases_mut().insert(alias, expression.clone()) {
                        Some(old) => {
                            if old == expression {
                                Err(())
                            } else {
                                Ok(())
                            }
                        }
                        None => Ok(()),
                    },
                )
                .unwrap();
            true
        }
        StorageOps::RemoveAlias(alias, channel) => {
            channel
                .send(
                    client
                        .get_aliases_mut()
                        .remove(&alias)
                        .map(|_| ())
                        .ok_or(()),
                )
                .unwrap();
            true
        }
        StorageOps::Get(aliases, channel) => {
            channel
                .send((
                    client.get_cmd_prefix().to_owned(),
                    client.get_roll_prefix().to_owned(),
                    {
                        let a = client.get_aliases();
                        aliases
                            .iter()
                            .filter_map(|alias| a.get(alias).map(|a| a.to_owned()))
                            .collect()
                    },
                    client.get_roll_info(),
                ))
                .unwrap();
            false
        }
        StorageOps::GetRollInfo(channel) => {
            channel.send(client.get_roll_info().to_owned()).unwrap();
            false
        }
        StorageOps::SetRollInfo(new, channel) => {
            *client.get_roll_info_mut() = new;
            channel.send(()).unwrap();
            true
        }
    }
}

impl<Id: ClientId,HB:BuildHasher+Default> ClientStorage<Id,HB> {
    fn new<S: ToString>(
        client_type: S,
        global: Arc<GlobalStorage>,
        cache_buckets: usize,
        cache_size: usize,
    ) -> Self {
        ClientStorage {
            client_type: client_type.to_string(),
            db_cache: Arc::from({
                let mut v = Vec::with_capacity(cache_buckets);
                let c = SizedCache::with_size(cache_size);
                for _ in 0..cache_buckets {
                    v.push(Mutex::new(c.clone()))
                }
                v
            }),
            query_cache: Arc::from(RwLock::new(HashMap::new())),
            global,
            hash_builder:HB::default()
        }
    }

    async fn run_cmd(&self, id: Id, op: StorageOps) {
        let bucket=self.db_cache.get(id.h)
    }

    async fn run(mut self) {
        let (loaded_sender, mut loaded_receiver) = mpsc::unbounded_channel::<(Id, ClientConfig)>();
        loop {
            tokio::select! {
                            biased;
                            rcv = loaded_receiver.recv() =>{
                                match rcv{
                                    Some((id, config)) => {
            let mut info = ClientInformation::new(config);
                                if self
                                    .query_cache
                                    .remove(&id)
                                    .into_iter()
                                    .flat_map(|v| v.into_iter())
                                    .map(|op| run_cmd(&mut info, op))
                                    .reduce(|r1, r2| r1 | r2)
                                    .unwrap_or(false)
                                {
                                    self.global.set(&mut info).await;
                                }
                                self.db_cache.cache_set(id, info);
                                    }
                                    None=> panic!("db receiver closed unexpectedly")
                                }
                            }
                            rcv = self.receiver.recv() =>{
                                match rcv{
                                    Some((id, op)) => match self.db_cache.cache_get_mut(&id) {
                    Some(info) => {
                        if run_cmd(info, op) {
                            self.global.set(info).await;
                        }
                    }
                    None => match self.query_cache.get_mut(&id) {
                        Some(queue) => queue.push(op),
                        None => {
                            self.query_cache.insert(id.clone(), vec![op]);
                            let id_clone = id.clone();
                            let sender_clone = loaded_sender.clone();
                            self.global
                                .get(
                                    serde_json::to_string(&Client {
                                        client_type: &self.client_type,
                                        client_id: id_clone,
                                    })
                                    .unwrap(),
                                    id,
                                    sender_clone,
                                )
                                .await;
                        }
                    }
                },
                None => break,
                                }
                            }
                        };
        }
        drop(loaded_sender);
        loop {
            let rcv = loaded_receiver.recv().await;
            match rcv {
                Some((id, config)) => {
                    let mut info = ClientInformation::new(config);
                    if self
                        .query_cache
                        .remove(&id)
                        .into_iter()
                        .flat_map(|v| v.into_iter())
                        .map(|op| run_cmd(&mut info, op))
                        .reduce(|r1, r2| r1 | r2)
                        .unwrap_or(false)
                    {
                        self.global.set(&mut info).await;
                    }
                    self.db_cache.cache_set(id, info);
                    continue;
                }
                None => break,
            }
        }
    }
}

#[derive(Clone)]
pub struct StorageHandle<Id: ClientId> {
    sender: mpsc::Sender<(Id, StorageOps)>,
}

impl<Id: ClientId> StorageHandle<Id> {
    pub(crate) fn new<S: ToString>(
        client_type: S,
        global: Arc<GlobalStorage>,
        channel_size: usize,
        cache_size: usize,
    ) -> (StorageHandle<Id>, tokio::task::JoinHandle<()>) {
        let (store, sender) = ClientStorage::new(client_type, global, channel_size, cache_size);
        (
            StorageHandle { sender },
            spawn(async move { store.run().await }),
        )
    }

    pub async fn get_command_prefix(&self, id: Id) -> String {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::GetCommandPrefix(sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn set_command_prefix(&self, id: Id, prefix: String) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::SetCommandPrefix(prefix, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn add_roll_prefix(&self, id: Id, prefix: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::AddRollPrefix(prefix, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn remove_roll_prefix(&self, id: Id, prefix: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::RemoveRollPrefix(prefix, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_roll_prefixes(&self, id: Id) -> Vec<String> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::GetRollPrefixes(sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn add_alias(
        &self,
        id: Id,
        alias: String,
        expr: VersionedRollExpr,
    ) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::AddAlias(alias, expr, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn remove_alias(&self, id: Id, alias: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::RemoveAlias(alias, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_alias(&self, id: Id, alias: String) -> Option<Arc<VersionedRollExpr>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::GetAlias(alias, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_all_alias(&self, id: Id) -> HashMap<String, Arc<VersionedRollExpr>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::GetAllAlias(sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_roll_info(&self, id: Id) -> bool {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::GetRollInfo(sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn set_roll_info(&self, id: Id, roll_info: bool) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::SetRollInfo(roll_info, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get(
        &self,
        id: Id,
        aliases: Vec<String>,
    ) -> (String, Vec<String>, Vec<Arc<VersionedRollExpr>>, bool) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((id, StorageOps::Get(aliases, sender)))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
}
