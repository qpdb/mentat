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

use db_traits::errors::DbError;

use failure::{ Backtrace, Context, Fail, };

use core_traits::Entid;

use std::fmt;

pub type Result<T> = std::result::Result<T, PullError>;

#[derive(Debug)]
pub struct PullErrorKind(Box<Context<PullErrorKind>>);

impl Fail for PullError {
    #[inline]
    fn cause(&self) -> Option<&dyn Fail> {
        self.0.cause()
    }

    #[inline]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for PullError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PullError {
    #[inline]
    pub fn kind(&self) -> &PullErrorKind {
        &*self.0.get_context()
    }
}

impl From<PullErrorKind> for PullError {
    #[inline]
    fn from(kind: PullErrorKind) -> PullError {
        PullErrorKind(Box::new(Context::new(kind)))
    }
}

#[derive(Debug, Fail)]
pub enum PullError {
    #[fail(display = "attribute {:?} has no name", _0)]
    UnnamedAttribute(Entid),

    #[fail(display = ":db/id repeated")]
    RepeatedDbId,

    #[fail(display = "{}", _0)]
    DbError(#[cause] DbError),
}

impl From<DbError> for PullErrorKind {
    fn from(error: DbError) -> PullErrorKind {
        PullErrorKind::DbError(error)
    }
}
impl From<DbError> for PullError {
    fn from(error: DbError) -> PullError {
        PullErrorKind::from(error).into()
    }
}
