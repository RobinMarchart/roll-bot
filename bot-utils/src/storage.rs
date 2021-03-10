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

use robins_dice_roll::dice_types::Expression;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{digest::generic_array::GenericArray, Digest, Sha256};
use std::hash::Hash;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::{collections::HashMap, fmt};
use tokio::{
    fs,
    sync::{mpsc, oneshot},
    task::spawn,
};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClientInformation {
    command_prefix: String,
    roll_prefix: Vec<String>,
    alias: HashMap<String, Arc<Expression>>,
}

enum ClientInformationWrapper<Id: ClientId> {
    Available(ClientInformation, mpsc::Sender<(Id, ClientInformation)>),
    Waiting(Vec<StorageOps<Id>>),
}

impl ClientInformation {
    fn new() -> ClientInformation {
        ClientInformation {
            command_prefix: "rrb!".to_string(),
            roll_prefix: vec![],
            alias: HashMap::new(),
        }
    }
}

struct Storage<Id: ClientId> {
    cache: HashMap<Id, ClientInformationWrapper<Id>>,
    store_path: Box<Path>,
    receiver: mpsc::UnboundedReceiver<StorageOps<Id>>,
    sender: mpsc::UnboundedSender<StorageOps<Id>>,
    write_handler: mpsc::Sender<(Id, ClientInformation)>,
    write_handler_counter: u8,
}

#[derive(Debug)]
enum StorageOps<Id: ClientId> {
    SetClientInfo(Id, ClientInformation),
    GetCommandPrefix(Id, oneshot::Sender<String>),
    SetCommandPrefix(Id, String, oneshot::Sender<()>),
    GetRollPrefixes(Id, oneshot::Sender<Vec<String>>),
    AddRollPrefix(Id, String, oneshot::Sender<Result<(), ()>>),
    RemoveRollPrefix(Id, String, oneshot::Sender<Result<(), ()>>),
    GetAllAlias(Id, oneshot::Sender<HashMap<String, Arc<Expression>>>),
    GetAlias(Id, String, oneshot::Sender<Option<Arc<Expression>>>),
    AddAlias(Id, String, Expression, oneshot::Sender<()>),
    RemoveAlias(Id, String, oneshot::Sender<Result<(), ()>>),
}

pub struct StorageHandle<Id: ClientId> {
    channel: mpsc::UnboundedSender<StorageOps<Id>>,
}

fn hash_id<Id: Serialize>(id: &Id) -> GenericArray<u8, <Sha256 as Digest>::OutputSize> {
    Sha256::digest(&serde_cbor::to_vec(id).expect("failed serializing id"))
}

impl<Id: ClientId> Storage<Id> {
    async fn new(
        store_path: Box<Path>,
    ) -> std::io::Result<(Storage<Id>, mpsc::UnboundedSender<StorageOps<Id>>)> {
        tokio::fs::create_dir_all(store_path.as_ref()).await?;
        let (sender, receiver) = mpsc::unbounded_channel();
        let store_path_clone = store_path.clone();
        let store = Storage::<Id> {
            cache: HashMap::new(),
            store_path,
            sender,
            receiver,
            write_handler: create_write_handler(store_path_clone),
            write_handler_counter: 0,
        };
        let input_sender = store.sender.clone();
        Ok((store, input_sender))
    }

    fn load(&mut self, id: Id, op: StorageOps<Id>) {
        self.cache
            .insert(id.clone(), ClientInformationWrapper::Waiting(vec![op]));
        let id_clone = id.clone();
        let path_clone = self.store_path.clone();
        let result_channel = self.sender.clone();

        spawn(async move {
            let id: Id = id_clone;
            let hashed_id = hex::encode(hash_id(&id));
            let path = Box::new(path_clone.deref().join(hashed_id + ".cbor"));
            let client_info = match fs::read(path.clone().deref()).await {
                Ok(content) => match serde_cbor::from_slice(&content) {
                    Ok(info) => info,
                    Err(e) => {
                        log::warn!("Parsing client information for {:?} from {:?} resulted in Error {}.\n Using default values",&id,path.deref(),e);
                        ClientInformation::new()
                    }
                },
                Err(e) => {
                    log::warn!("Unable to open client information for {:?} from {:?}: {}.\n Using default values",&id,path.deref(),e);
                    ClientInformation::new()
                }
            };
            result_channel
                .send(StorageOps::SetClientInfo(id, client_info))
                .unwrap()
        });
    }

    async fn handle(mut self) {
        loop {
            match self.receiver.recv().await {
                Some(operation) => match operation {
                    StorageOps::GetCommandPrefix(id, channel) => match self.cache.get_mut(&id) {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, _) => {
                                channel.send(data.command_prefix.clone()).unwrap()
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::GetCommandPrefix(id, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::GetCommandPrefix(id_clone, channel));
                        }
                    },
                    StorageOps::SetCommandPrefix(id, prefix, channel) => {
                        match self.cache.get_mut(&id) {
                            Some(value) => match value {
                                ClientInformationWrapper::Available(data, write) => {
                                    data.command_prefix = prefix;
                                    write.send((id, data.to_owned())).await.unwrap();
                                    channel.send(()).unwrap();
                                }
                                ClientInformationWrapper::Waiting(vec) => {
                                    vec.push(StorageOps::SetCommandPrefix(id, prefix, channel))
                                }
                            },
                            None => {
                                let id_clone = id.clone();
                                self.load(
                                    id,
                                    StorageOps::SetCommandPrefix(id_clone, prefix, channel),
                                )
                            }
                        }
                    }
                    StorageOps::GetRollPrefixes(id, channel) => match self.cache.get_mut(&id) {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, _) => {
                                channel.send(data.roll_prefix.clone()).unwrap()
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::GetRollPrefixes(id, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::GetRollPrefixes(id_clone, channel))
                        }
                    },
                    StorageOps::RemoveRollPrefix(id, prefix, channel) => {
                        match self.cache.get_mut(&id) {
                            Some(value) => match value {
                                ClientInformationWrapper::Available(data, write) => {
                                    for index in 0..data.roll_prefix.len() {
                                        if data.roll_prefix.get(index).unwrap() == &prefix {
                                            data.roll_prefix.remove(index);
                                            write.send((id, data.clone())).await.unwrap();
                                            channel.send(Ok(())).unwrap();
                                            return;
                                        }
                                    }
                                    channel.send(Err(())).unwrap()
                                }
                                ClientInformationWrapper::Waiting(vec) => {
                                    vec.push(StorageOps::RemoveRollPrefix(id, prefix, channel))
                                }
                            },
                            None => {
                                let id_clone = id.clone();
                                self.load(
                                    id,
                                    StorageOps::RemoveRollPrefix(id_clone, prefix, channel),
                                )
                            }
                        }
                    }
                    StorageOps::AddRollPrefix(id, prefix, channel) => match self.cache.get_mut(&id)
                    {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, write) => {
                                for p in data.roll_prefix.iter() {
                                    if p == prefix.as_str() {
                                        channel.send(Err(())).unwrap();
                                        return;
                                    }
                                }
                                data.roll_prefix.push(prefix);
                                write.send((id, data.clone())).await.unwrap();
                                channel.send(Ok(())).unwrap();
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::AddRollPrefix(id, prefix, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::AddRollPrefix(id_clone, prefix, channel))
                        }
                    },
                    StorageOps::GetAllAlias(id, channel) => match self.cache.get_mut(&id) {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, _) => {
                                channel.send(data.alias.clone()).unwrap()
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::GetAllAlias(id, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::GetAllAlias(id_clone, channel))
                        }
                    },
                    StorageOps::GetAlias(id, alias, channel) => match self.cache.get_mut(&id) {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, _) => channel
                                .send(data.alias.get(&alias).map(|a| a.clone()))
                                .unwrap(),
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::GetAlias(id, alias, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::GetAlias(id_clone, alias, channel))
                        }
                    },
                    StorageOps::AddAlias(id, alias, expr, channel) => match self.cache.get_mut(&id)
                    {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, write) => {
                                data.alias.insert(alias, Arc::new(expr));
                                write.send((id, data.clone())).await.unwrap();
                                channel.send(()).unwrap()
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::AddAlias(id, alias, expr, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::AddAlias(id_clone, alias, expr, channel))
                        }
                    },
                    StorageOps::RemoveAlias(id, alias, channel) => match self.cache.get_mut(&id) {
                        Some(value) => match value {
                            ClientInformationWrapper::Available(data, write) => {
                                let result = data.alias.remove(&alias);
                                write.send((id, data.clone())).await.unwrap();
                                channel
                                    .send(match result {
                                        Some(_) => Ok(()),
                                        None => Err(()),
                                    })
                                    .unwrap()
                            }
                            ClientInformationWrapper::Waiting(vec) => {
                                vec.push(StorageOps::RemoveAlias(id, alias, channel))
                            }
                        },
                        None => {
                            let id_clone = id.clone();
                            self.load(id, StorageOps::RemoveAlias(id_clone, alias, channel))
                        }
                    },
                    StorageOps::SetClientInfo(id, new) => {
                        self.write_handler_counter = self.write_handler_counter.wrapping_add(1);
                        if let Some(ClientInformationWrapper::Waiting(vec)) = self.cache.insert(
                            id,
                            ClientInformationWrapper::Available(
                                new,
                                if self.write_handler_counter == 0 {
                                    self.write_handler =
                                        create_write_handler(self.store_path.clone());
                                    self.write_handler.clone()
                                } else {
                                    self.write_handler.clone()
                                },
                            ),
                        ) {
                            for a in vec {
                                self.sender.send(a).unwrap();
                            }
                        }
                    }
                },
                None => {
                    break;
                }
            }
        }
    }
}

fn create_write_handler<Id: ClientId>(
    base_path: Box<Path>,
) -> mpsc::Sender<(Id, ClientInformation)> {
    let (sender, mut receiver) = mpsc::channel::<(Id, ClientInformation)>(32);
    spawn(async move {
        loop {
            match receiver.recv().await {
                Some((id, value)) => {
                    let path = base_path.join(hex::encode(hash_id(&id)) + ".cbor");
                    match serde_cbor::to_vec(&value) {
                        Ok(val) => match fs::write(path, val).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::warn!("Failed to write config change for {:?}: {}", id, e)
                            }
                        },
                        Err(e) => {
                            log::warn!(
                                "Failed to serialize config for {:?}: {}\nConfig: {:?}",
                                id,
                                e,
                                value
                            )
                        }
                    }
                }
                None => break,
            }
        }
    });
    sender
}

impl<Id: ClientId> StorageHandle<Id> {
    pub async fn new(base_path: Box<Path>) -> std::io::Result<StorageHandle<Id>> {
        let (store, sender) = Storage::new(base_path).await?;
        spawn(async move {
            store.handle().await;
        });
        Ok(StorageHandle { channel: sender })
    }
    pub async fn get_command_prefix(&self, id: Id) -> String {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::GetCommandPrefix(id, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn set_command_prefix(&self, id: Id, prefix: String) {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::SetCommandPrefix(id, prefix, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_roll_prefixes(&self, id: Id) -> Vec<String> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::GetRollPrefixes(id, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn add_roll_prefix(&self, id: Id, prefix: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::AddRollPrefix(id, prefix, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn remove_roll_prefix(&self, id: Id, prefix: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::RemoveRollPrefix(id, prefix, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_alias(&self, id: Id, alias: String) -> Option<Arc<Expression>> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::GetAlias(id, alias, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn set_alias(&self, id: Id, alias: String, expr: Expression) {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::AddAlias(id, alias, expr, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn remove_alias(&self, id: Id, alias: String) -> Result<(), ()> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::RemoveAlias(id, alias, sender))
            .unwrap();
        receiver.await.unwrap()
    }
    pub async fn get_all_alias(&self, id: Id) -> HashMap<String, Arc<Expression>> {
        let (sender, receiver) = oneshot::channel();
        self.channel
            .send(StorageOps::GetAllAlias(id, sender))
            .unwrap();
        receiver.await.unwrap()
    }
}
