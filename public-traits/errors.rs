// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#![allow(dead_code)]

use std; // To refer to std::result::Result.

use std::collections::BTreeSet;
use std::error::Error;

use rusqlite;
use failure::{
    Backtrace,
    Context,
    Fail,
};
+use std::fmt;
use uuid;

use edn;

use core_traits::{Attribute, ValueType};

use db_traits::errors::DbError;
use query_algebrizer_traits::errors::AlgebrizerError;
use query_projector_traits::errors::ProjectorError;
use query_pull_traits::errors::PullError;
use sql_traits::errors::SQLError;

#[cfg(feature = "syncable")]
use tolstoy_traits::errors::TolstoyError;

#[cfg(feature = "syncable")]
use hyper;

#[cfg(feature = "syncable")]
use serde_json;

pub type Result<T> = std::result::Result<T, MentatError>;

#[derive(Debug)]
pub struct MentatError(Box<Context<MentatErrorKind>>);

impl Fail for MentatError {
    #[inline]
    fn cause(&self) -> Option<&Fail> {
        self.0.cause()
    }

    #[inline]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for MentatError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl MentatError {
    #[inline]
    pub fn kind(&self) -> &MentatErrorKind {
        &*self.0.get_context()
    }
}

impl From<MentatErrorKind> for MentatError {
    #[inline]
    fn from(kind: MentatErrorKind) -> MentatError {
        MentatError(Box::new(Context::new(kind)))
    }
}

impl From<Context<MentatErrorKind>> for MentatError {
    #[inline]
    fn from(inner: Context<MentatErrorKind>) -> MentatError {
        MentatError(Box::new(inner))
    }
}

#[derive(Debug, Fail)]
pub enum MentatErrorKind {
    #[fail(display = "bad uuid {}", _0)]
    BadUuid(String),

    #[fail(display = "path {} already exists", _0)]
    PathAlreadyExists(String),

    #[fail(display = "variables {:?} unbound at query execution time", _0)]
    UnboundVariables(BTreeSet<String>),

    #[fail(display = "invalid argument name: '{}'", _0)]
    InvalidArgumentName(String),

    #[fail(display = "unknown attribute: '{}'", _0)]
    UnknownAttribute(String),

    #[fail(display = "invalid vocabulary version")]
    InvalidVocabularyVersion,

    #[fail(
        display = "vocabulary {}/version {} already has attribute {}, and the requested definition differs",
        _0, _1, _2
    )]
    ConflictingAttributeDefinitions(String, u32, String, Attribute, Attribute),

    #[fail(
        display = "existing vocabulary {} too new: wanted version {}, got version {}",
        _0, _1, _2
    )]
    ExistingVocabularyTooNew(String, u32, u32),

    #[fail(display = "core schema: wanted version {}, got version {:?}", _0, _1)]
    UnexpectedCoreSchema(u32, Option<u32>),

    #[fail(display = "Lost the transact() race!")]
    UnexpectedLostTransactRace,

    #[fail(display = "missing core attribute {}", _0)]
    MissingCoreVocabulary(edn::query::Keyword),

    #[fail(display = "schema changed since query was prepared")]
    PreparedQuerySchemaMismatch,

    #[fail(
        display = "provided value of type {} doesn't match attribute value type {}",
        _0, _1
    )]
    ValueTypeMismatch(ValueType, ValueType),

    #[fail(display = "{}", _0)]
    IoError(#[cause] std::io::Error),

    /// We're just not done yet.  Message that the feature is recognized but not yet
    /// implemented.
    #[fail(display = "not yet implemented: {}", _0)]
    NotYetImplemented(String),

    // It would be better to capture the underlying `rusqlite::Error`, but that type doesn't
    // implement many useful traits, including `Clone`, `Eq`, and `PartialEq`.
    #[fail(display = "SQL error: {}, cause: {}", _0, _1)]
    RusqliteError(String, String),

    #[fail(display = "{}", _0)]
    EdnParseError(#[cause] edn::ParseError),

    #[fail(display = "{}", _0)]
    DbError(#[cause] DbError),

    #[fail(display = "{}", _0)]
    AlgebrizerError(#[cause] AlgebrizerError),

    #[fail(display = "{}", _0)]
    ProjectorError(#[cause] ProjectorError),

    #[fail(display = "{}", _0)]
    PullError(#[cause] PullError),

    #[fail(display = "{}", _0)]
    SQLError(#[cause] SQLError),

    #[fail(display = "{}", _0)]
    UuidError(#[cause] uuid::Error),

    #[cfg(feature = "syncable")]
    #[fail(display = "{}", _0)]
    TolstoyError(#[cause] TolstoyError),

    #[cfg(feature = "syncable")]
    #[fail(display = "{}", _0)]
    NetworkError(#[cause] hyper::Error),

    #[cfg(feature = "syncable")]
    #[fail(display = "{}", _0)]
    UriError(#[cause] http::uri::InvalidUri),

    #[cfg(feature = "syncable")]
    #[fail(display = "{}", _0)]
    SerializationError(#[cause] serde_json::Error),
}

impl From<std::io::Error> for MentatErrorKind {
    fn from(error: std::io::Error) -> MentatErrorKind {
        MentatErrorKind::IoError(error)
    }
}

impl From<rusqlite::Error> for MentatErrorKind {
    fn from(error: rusqlite::Error) -> MentatErrorKind {
        MentatErrorKind::RusqliteError(error.to_string())
    }
}

impl From<edn::ParseError> for MentatErrorKind {
    fn from(error: edn::ParseError) -> MentatErrorKind {
        MentatErrorKind::EdnParseError(error)
    }
}

impl From<mentat_db::DbError> for MentatErrorKind {
    fn from(error: mentat_db::DbError) -> MentatErrorKind {
        MentatErrorKind::DbError(error)
    }
}

impl From<mentat_query_algebrizer::AlgebrizerError> for MentatErrorKind {
    fn from(error: mentat_query_algebrizer::AlgebrizerError) -> MentatErrorKind {
        MentatErrorKind::AlgebrizerError(error)
    }
}

impl From<mentat_query_projector::ProjectorError> for MentatErrorKind {
    fn from(error: mentat_query_projector::ProjectorError) -> MentatErrorKind {
        MentatErrorKind::ProjectorError(error)
    }
}

impl From<mentat_query_pull::PullError> for MentatErrorKind {
    fn from(error: mentat_query_pull::PullError) -> MentatErrorKind {
        MentatErrorKind::PullError(error)
    }
}

impl From<mentat_sql::SQLError> for MentatErrorKind {
    fn from(error: mentat_sql::SQLError) -> MentatErrorKind {
        MentatErrorKind::SQLError(error)
    }
}

#[cfg(feature = "syncable")]
impl From<mentat_tolstoy::TolstoyError> for MentatErrorKind {
    fn from(error: mentat_tolstoy::TolstoyError) -> MentatErrorKind {
        MentatErrorKind::TolstoyError(error)
    }
}

// XXX reduce dupe if this isn't completely throwaway


impl From<std::io::Error> for MentatError {
    fn from(error: std::io::Error) -> Self {
        MentatError::from(error).into()
    }
}

impl From<rusqlite::Error> for MentatError {
    fn from(error: rusqlite::Error) -> Self {
        let cause = match error.source() {
            Some(e) => e.to_string(),
            None => "".to_string(),
        };
        MentatError::from(error).into()
    }
}

impl From<uuid::Error> for MentatError {
    fn from(error: uuid::Error) -> Self {
        MentatError::from(error).into()
    }
}

impl From<edn::ParseError> for MentatError {
    fn from(error: edn::ParseError) -> Self {
        MentatError:from(error).into()
    }
}

impl From<DbError> for MentatError {
    fn from(error: DbError) -> Self {
        MentatError::from(error).into()
    }
}

impl From<AlgebrizerError> for MentatError {
    fn from(error: AlgebrizerError) -> Self {
        MentatError::from(error).into()
    }
}

impl From<ProjectorError> for MentatError {
    fn from(error: ProjectorError) -> Self {
        MentatError::from(error).into()
    }
}

impl From<PullError> for MentatError {
    fn from(error: PullError) -> Self {
        MentatError::from(error).into()
    }
}

impl From<SQLError> for MentatError {
    fn from(error: SQLError) -> Self {
        MentatError::from(error).into()
    }
}

#[cfg(feature = "syncable")]
impl From<TolstoyError> for MentatError {
    fn from(error: TolstoyError) -> Self {
        MentatError::from(error).into()
    }
}

#[cfg(feature = "syncable")]
impl From<serde_json::Error> for MentatError {
    fn from(error: serde_json::Error) -> Self {
        MentatError::from(error).into()
    }
}

#[cfg(feature = "syncable")]
impl From<hyper::Error> for MentatError {
    fn from(error: hyper::Error) -> Self {
        MentatError::from(error).into()
    }
}

#[cfg(feature = "syncable")]
impl From<http::uri::InvalidUri> for MentatError {
    fn from(error: http::uri::InvalidUri) -> Self {
        MentatError::from(error).into()
    }
}
