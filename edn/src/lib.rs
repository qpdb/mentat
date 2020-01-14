// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#![allow(ellipsis_inclusive_range_patterns)]

extern crate chrono;
extern crate itertools;
extern crate num;
extern crate ordered_float;
extern crate pretty;
extern crate uuid;

#[cfg(feature = "serde_support")]
extern crate serde;

#[cfg(feature = "serde_support")]
#[macro_use]
extern crate serde_derive;

pub mod entities;
pub mod intern_set;
pub use intern_set::InternSet;
// Intentionally not pub.
pub mod matcher;
mod namespaceable_name;
pub mod pretty_print;
pub mod query;
pub mod symbols;
pub mod types;
pub mod utils;
pub mod value_rc;
pub use value_rc::{Cloned, FromRc, ValueRc};

pub mod parse {
    include!(concat!(env!("OUT_DIR"), "/edn.rs"));
}

// Re-export the types we use.
pub use chrono::{DateTime, Utc};
pub use num::BigInt;
pub use ordered_float::OrderedFloat;
pub use uuid::Uuid;

// Export from our modules.
pub use parse::ParseError;
pub use types::{
    FromMicros, FromMillis, Span, SpannedValue, ToMicros, ToMillis, Value, ValueAndSpan,
};

pub use symbols::{Keyword, NamespacedSymbol, PlainSymbol};
