// Copyright 2018 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use std; // To refer to std::result::Result.

use rusqlite;

use core_traits::ValueTypeSet;
use db_traits::errors::DbError;
use edn::query::PlainSymbol;
use failure::{ Backtrace, Context, Fail, };
use std::fmt;
use query_pull_traits::errors::PullError;

use aggregates::SimpleAggregationOp;

pub type Result<T> = std::result::Result<T, ProjectorError>;

#[derive(Debug)]
pub struct ProjectorError(Box<Context<ProjectorErrorKind>>);

impl Fail for ProjectorError {
    #[inline]
    fn cause(&self) -> Option<&Fail> {
        self.0.cause()
    }

    #[inline]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for ProjectorError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl ProjectorError {
    #[inline]
    pub fn kind(&self) -> &ProjectorErrorKind {
        &*self.0.get_context()
    }
}

impl From<ProjectorErrorKind> for ProjectorError {
    #[inline]
    fn from(kind: ProjectorErrorKind) -> ProjectorError {
        ProjectorError(Box::new(Context::new(kind)))
    }
}

impl From<Context<ProjectorErrorKind>> for ProjectorError {
    #[inline]
    fn from(inner: Context<ProjectorErrorKind>) -> ProjectorError {
        ProjectorError(Box::new(inner))
    }
}


#[derive(Debug, Fail)]
pub enum ProjectorErrorKind {
    /// We're just not done yet.  Message that the feature is recognized but not yet
    /// implemented.
    #[fail(display = "not yet implemented: {}", _0)]
    NotYetImplemented(String),

    #[fail(display = "no possible types for value provided to {:?}", _0)]
    CannotProjectImpossibleBinding(SimpleAggregationOp),

    #[fail(
        display = "cannot apply projection operation {:?} to types {:?}",
        _0, _1
    )]
    CannotApplyAggregateOperationToTypes(SimpleAggregationOp, ValueTypeSet),

    #[fail(display = "invalid projection: {}", _0)]
    InvalidProjection(String),

    #[fail(display = "cannot project unbound variable {:?}", _0)]
    UnboundVariable(PlainSymbol),

    #[fail(display = "cannot find type for variable {:?}", _0)]
    NoTypeAvailableForVariable(PlainSymbol),

    #[fail(display = "expected {}, got {}", _0, _1)]
    UnexpectedResultsType(&'static str, &'static str),

    #[fail(
        display = "expected tuple of length {}, got tuple of length {}",
        _0, _1
    )]
    UnexpectedResultsTupleLength(usize, usize),

    #[fail(display = "min/max expressions: {} (max 1), corresponding: {}", _0, _1)]
    AmbiguousAggregates(usize, usize),

    // It would be better to capture the underlying `rusqlite::Error`, but that type doesn't
    // implement many useful traits, including `Clone`, `Eq`, and `PartialEq`.
    #[fail(display = "SQL error: {}", _0)]
    RusqliteError(String),

    #[fail(display = "{}", _0)]
    DbError(#[cause] DbError),

    #[fail(display = "{}", _0)]
    PullError(#[cause] PullError),
}

impl From<rusqlite::Error> for ProjectorErrorKind {
    fn from(error: rusqlite::Error) -> ProjectorErrorKind {
        ProjectorErrorKind::from(error).into()
    }
}

impl From<mentat_db::DbError> for ProjectorErrorKind {
    fn from(error: mentat_db::DbError) -> ProjectorErrorKind {
        ProjectorErrorKind::from(error).into()
    }
}

impl From<mentat_query_pull::PullError> for ProjectorErrorKind {
    fn from(error: mentat_query_pull::PullError) -> ProjectorErrorKind {
        ProjectorErrorKind::from(error).into()
    }
}

impl From<rusqlite::Error> for ProjectorError {
    fn from(error: rusqlite::Error) -> ProjectorError {
        ProjectorError::RusqliteError(error.to_string())
    }
}

impl From<DbError> for ProjectorError {
    fn from(error: DbError) -> ProjectorError {
        ProjectorError::DbError(error)
    }
}

impl From<PullError> for ProjectorError {
    fn from(error: PullError) -> ProjectorError {
        ProjectorError::PullError(error)
    }
}
