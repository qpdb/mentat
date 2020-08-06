// Copyright 2018 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

// TODO https://github.com/mozilla/mentat/issues/569
// extern crate hyper_tls;

#[macro_use]
extern crate core_traits;

pub mod bootstrap;
pub mod metadata;
pub use crate::metadata::{PartitionsTable, SyncMetadata};
mod datoms;
pub mod debug;
pub mod remote_client;
pub use crate::remote_client::RemoteClient;
pub mod schema;
pub mod syncer;
pub use crate::syncer::{SyncFollowup, SyncReport, SyncResult, Syncer};
pub mod logger;
pub mod tx_mapper;
mod tx_uploader;
pub use crate::tx_mapper::TxMapper;
pub mod tx_processor;
pub mod types;
pub use crate::types::{GlobalTransactionLog, Tx, TxPart};
