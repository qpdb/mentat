// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use failure::{ Backtrace, Context, Fail, };
use std::fmt;

#[derive(Debug)]
pub struct SQLError(Box<Context<SQLErrorKind>>);

impl Fail for SQLError {
    #[inline]
    fn cause(&self) -> Option<&dyn Fail> {
        self.0.cause()
    }

    #[inline]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for SQLError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl SQLError {
    #[inline]
    pub fn kind(&self) -> &SQLErrorKind {
        &*self.0.get_context()
    }
}

impl From<SQLErrorKind> for SQLError {
    #[inline]
    fn from(kind: SQLErrorKind) -> SQLError {
        SQLError(Box::new(Context::new(kind)))
    }
}

impl From<Context<SQLErrorKind>> for SQLError {
    #[inline]
    fn from(inner: Context<SQLErrorKind>) -> SQLError {
        SQLError(Box::new(inner))
    }
}

#[derive(Debug, Fail)]
pub enum SQLErrorKind {
    #[fail(display = "invalid parameter name: {}", _0)]
    InvalidParameterName(String),

    #[fail(display = "parameter name could be generated: '{}'", _0)]
    BindParamCouldBeGenerated(String),
}

pub type BuildQueryResult = Result<(), SQLError>;
