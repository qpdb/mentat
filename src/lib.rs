// Copyright 2016 Mozilla
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

pub use edn;

#[macro_use]
extern crate core_traits;

pub use core_traits::{
    now, Attribute, Binding, Entid, KnownEntid, StructuredMap, TypedValue, ValueType,
};

pub use mentat_core::{DateTime, HasSchema, Keyword, Schema, TxReport, Utc, Uuid};

pub use edn::query::FindSpec;

pub use mentat_db::{
    new_connection, AttributeSet, TxObserver, CORE_SCHEMA_VERSION, DB_SCHEMA_CORE,
};

#[cfg(feature = "sqlcipher")]
pub use mentat_db::{change_encryption_key, new_connection_with_key};

/// Produce the appropriate `Variable` for the provided valid ?-prefixed name.
/// This lives here because we can't re-export macros:
/// https://github.com/rust-lang/rust/issues/29638.
#[macro_export]
macro_rules! var {
    ( ? $var:ident ) => {
        $crate::Variable::from_valid_name(concat!("?", stringify!($var)))
    };
}

/// Produce the appropriate `Keyword` for the provided namespace and name.
/// This lives here because we can't re-export macros:
/// https://github.com/rust-lang/rust/issues/29638.
#[macro_export]
macro_rules! kw {
    ( : $ns:ident$(. $nss:ident)+ / $nn:ident$(. $nns:ident)+ ) => {
        $crate::Keyword::namespaced(
            concat!(stringify!($ns) $(, ".", stringify!($nss))*),
            concat!(stringify!($nn) $(, ".", stringify!($nns))*),
        )
    };

    ( : $ns:ident$(. $nss:ident)+ / $nn:ident ) => {
        $crate::Keyword::namespaced(
            concat!(stringify!($ns) $(, ".", stringify!($nss))*),
            stringify!($nn)
        )
    };

    ( : $ns:ident / $nn:ident$(. $nns:ident)+ ) => {
        $crate::Keyword::namespaced(
            stringify!($ns),
            concat!(stringify!($nn) $(, ".", stringify!($nns))*),
        )
    };

    ( : $ns:ident / $nn:ident ) => {
        $crate::Keyword::namespaced(
            stringify!($ns),
            stringify!($nn)
        )
    };

    ( : $n:ident ) => {
        $crate::Keyword::plain(
            stringify!($n)
        )
    };
}

pub use public_traits::errors;
pub use public_traits::errors::{MentatError, Result};

pub use edn::{FromMicros, FromMillis, ParseError, ToMicros, ToMillis};
pub use mentat_query_projector::BindingTuple;
pub use query_algebrizer_traits::errors::AlgebrizerError;
pub use query_projector_traits::errors::ProjectorError;
pub use query_pull_traits::errors::PullError;
pub use sql_traits::errors::SQLError;

pub use mentat_transaction::Metadata;

pub use mentat_transaction::entity_builder;
pub use mentat_transaction::query;

pub use mentat_transaction::query::{
    q_once, IntoResult, PlainSymbol, QueryExecutionResult, QueryExplanation, QueryInputs,
    QueryOutput, QueryPlanStep, QueryResults, RelResult, Variable,
};

pub mod conn;
pub mod query_builder;
pub mod store;
pub mod vocabulary;

#[cfg(feature = "syncable")]
mod sync;

#[cfg(feature = "syncable")]
pub use sync::Syncable;

#[cfg(feature = "syncable")]
pub use mentat_tolstoy::SyncReport;

pub use query_builder::QueryBuilder;

pub use conn::Conn;

pub use mentat_transaction::{CacheAction, CacheDirection, InProgress, Pullable, Queryable};

pub use store::Store;

#[cfg(test)]
mod tests {
    use super::*;
    use edn::symbols::Keyword;

    #[test]
    fn can_import_edn() {
        assert_eq!(":foo", &Keyword::plain("foo").to_string());
    }

    #[test]
    fn test_kw() {
        assert_eq!(kw!(:foo/bar), Keyword::namespaced("foo", "bar"));
        assert_eq!(
            kw!(:org.mozilla.foo/bar_baz),
            Keyword::namespaced("org.mozilla.foo", "bar_baz")
        );
        assert_eq!(
            kw!(:_foo_/_bar_._baz_),
            Keyword::namespaced("_foo_", "_bar_._baz_")
        );
        assert_eq!(
            kw!(:_org_._mozilla_._foo_/_bar_._baz_),
            Keyword::namespaced("_org_._mozilla_._foo_", "_bar_._baz_")
        );
    }

    #[test]
    fn test_var() {
        let foo_baz = var!(?foo_baz);
        let vu = var!(?vü);
        assert_eq!(foo_baz, Variable::from_valid_name("?foo_baz"));
        assert_eq!(vu, Variable::from_valid_name("?vü"));
        assert_eq!(foo_baz.as_str(), "?foo_baz");
    }
}
