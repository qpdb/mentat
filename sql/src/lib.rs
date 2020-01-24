// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

extern crate failure;

extern crate ordered_float;
extern crate rusqlite;

extern crate core_traits;
extern crate mentat_core;
extern crate sql_traits;

use std::rc::Rc;
use std::collections::HashMap;

use ordered_float::OrderedFloat;

use core_traits::TypedValue;

use sql_traits::errors::{BuildQueryResult, SQLErrorKind};

use mentat_core::{ToMicros, ValueRc};

/// We want to accumulate values that will later be substituted into a SQL statement execution.
/// This struct encapsulates the generated string and the _initial_ argument list.
/// Additional user-supplied argument bindings, with their placeholders accumulated via
/// `push_bind_param`, will be appended to this argument list.
pub struct SQLQuery {
    pub sql: String,

    /// These will eventually perhaps be rusqlite `ToSql` instances.
    pub args: Vec<(String, Rc<rusqlite::types::Value>)>,
}

/// Gratefully based on Diesel's QueryBuilder trait:
/// https://github.com/diesel-rs/diesel/blob/4885f61b8205f7f3c2cfa03837ed6714831abe6b/diesel/src/query_builder/mod.rs#L56
pub trait QueryBuilder {
    fn push_sql(&mut self, sql: &str);
    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult;
    fn push_typed_value(&mut self, value: &TypedValue) -> BuildQueryResult;
    fn push_bind_param(&mut self, name: &str) -> BuildQueryResult;
    fn finish(self) -> SQLQuery;
}

pub trait QueryFragment {
    fn push_sql(&self, out: &mut dyn QueryBuilder) -> BuildQueryResult;
}

impl QueryFragment for Box<dyn QueryFragment> {
    fn push_sql(&self, out: &mut dyn QueryBuilder) -> BuildQueryResult {
        QueryFragment::push_sql(&**self, out)
    }
}

impl<'a> QueryFragment for &'a dyn QueryFragment {
    fn push_sql(&self, out: &mut dyn QueryBuilder) -> BuildQueryResult {
        QueryFragment::push_sql(&**self, out)
    }
}

impl QueryFragment for () {
    fn push_sql(&self, _out: &mut dyn QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

/// A QueryBuilder that implements SQLite's specific escaping rules.
pub struct SQLiteQueryBuilder {
    pub sql: String,

    arg_prefix: String,
    arg_counter: i64,

    // We can't just use an InternSet on the rusqlite::types::Value instances, because that
    // includes f64, so it's not Hash or Eq.
    // Instead, we track the byte and String arguments separately, mapping them to their argument name,
    // in order to dedupe. We'll add these to the regular argument vector later.
    byte_args: HashMap<Vec<u8>, String>, // From value to argument name.
    string_args: HashMap<ValueRc<String>, String>, // From value to argument name.
    args: Vec<(String, Rc<rusqlite::types::Value>)>, // (arg, value).
}

impl SQLiteQueryBuilder {
    pub fn new() -> Self {
        SQLiteQueryBuilder::with_prefix("$v".to_string())
    }

    pub fn with_prefix(prefix: String) -> Self {
        SQLiteQueryBuilder {
            sql: String::new(),
            arg_prefix: prefix,
            arg_counter: 0,

            byte_args: HashMap::default(),
            string_args: HashMap::default(),
            args: vec![],
        }
    }

    fn next_argument_name(&mut self) -> String {
        let arg = format!("{}{}", self.arg_prefix, self.arg_counter);
        self.arg_counter = self.arg_counter + 1;
        arg
    }

    fn push_static_arg(&mut self, val: Rc<rusqlite::types::Value>) {
        // TODO: intern these, too.
        let arg = self.next_argument_name();
        self.push_named_arg(arg.as_str());
        self.args.push((arg, val));
    }

    fn push_named_arg(&mut self, arg: &str) {
        self.push_sql(arg);
    }
}

impl QueryBuilder for SQLiteQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        self.push_sql("`");
        self.push_sql(&identifier.replace("`", "``"));
        self.push_sql("`");
        Ok(())
    }

    fn push_typed_value(&mut self, value: &TypedValue) -> BuildQueryResult {
        use TypedValue::*;
        match value {
            &Ref(entid) => self.push_sql(entid.to_string().as_str()),
            &Boolean(v) => self.push_sql(if v { "1" } else { "0" }),
            &Long(v) => self.push_sql(v.to_string().as_str()),
            &Double(OrderedFloat(v)) => {
                // Rust's floats print without a trailing '.' in some cases.
                // https://github.com/rust-lang/rust/issues/30967
                // We format with 'e' -- scientific notation -- so that SQLite treats them as
                // floats and not integers. This is most noticeable for fulltext scores, which
                // will currently (2017-06) always be 0, and need to round-trip as doubles.
                self.push_sql(format!("{:e}", v).as_str());
            }
            &Instant(dt) => {
                self.push_sql(format!("{}", dt.to_micros()).as_str()); // TODO: argument instead?
            }
            &Uuid(ref u) => {
                let bytes = u.as_bytes();
                if let Some(arg) = self.byte_args.get(bytes.as_ref()).cloned() {
                    // Why, borrow checker, why?!
                    self.push_named_arg(arg.as_str());
                } else {
                    let arg = self.next_argument_name();
                    self.push_named_arg(arg.as_str());
                    self.byte_args.insert(bytes.clone().to_vec(), arg);
                }
            }
            // These are both `Rc`. Unfortunately, we can't use that fact when
            // turning these into rusqlite Values.
            // However, we can check to see whether there's an existing var that matches…
            &String(ref s) => {
                if let Some(arg) = self.string_args.get(s).cloned() {
                    self.push_named_arg(arg.as_str());
                } else {
                    let arg = self.next_argument_name();
                    self.push_named_arg(arg.as_str());
                    self.string_args.insert(s.clone(), arg);
                }
            }
            &Keyword(ref s) => {
                // TODO: intern.
                let v = Rc::new(rusqlite::types::Value::Text(s.as_ref().to_string()));
                self.push_static_arg(v);
            }
        }
        Ok(())
    }

    /// Our bind parameters will be interleaved with pushed `TypedValue` instances. That means we
    /// need to use named parameters, not positional parameters.
    /// The `name` argument to this method is expected to be alphanumeric. If not, this method
    /// returns an `InvalidParameterName` error result.
    /// Callers should make sure that the name doesn't overlap with generated parameter names. If
    /// it does, `BindParamCouldBeGenerated` is the error.
    fn push_bind_param(&mut self, name: &str) -> BuildQueryResult {
        // Do some validation first.
        // This is not free, but it's probably worth it for now.
        if !name.chars().all(|c| char::is_alphanumeric(c) || c == '_') {
            return Err(SQLErrorKind::InvalidParameterName(name.to_string()));
        }

        if name.starts_with(self.arg_prefix.as_str())
            && name
                .chars()
                .skip(self.arg_prefix.len())
                .all(char::is_numeric)
        {
            return Err(SQLErrorKind::BindParamCouldBeGenerated(name.to_string()));
        }

        self.push_sql("$");
        self.push_sql(name);
        Ok(())
    }

    fn finish(self) -> SQLQuery {
        // We collected string and byte arguments into separate maps so that we could
        // dedupe them. Now we need to turn them into rusqlite Values.
        let mut args = self.args;
        let string_args = self.string_args.into_iter().map(|(val, arg)| {
            (
                arg,
                Rc::new(rusqlite::types::Value::Text(val.as_ref().clone())),
            )
        });
        let byte_args = self
            .byte_args
            .into_iter()
            .map(|(val, arg)| (arg, Rc::new(rusqlite::types::Value::Blob(val))));

        args.extend(string_args);
        args.extend(byte_args);

        // Get the args in the right order -- $v0, $v1…
        args.sort_by(|&(ref k1, _), &(ref k2, _)| k1.cmp(k2));
        SQLQuery {
            sql: self.sql,
            args: args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_arg(s: &str) -> Rc<rusqlite::types::Value> {
        Rc::new(rusqlite::types::Value::Text(s.to_string()))
    }

    #[test]
    fn test_sql() {
        let mut s = SQLiteQueryBuilder::new();
        s.push_sql("SELECT ");
        s.push_identifier("foo").unwrap();
        s.push_sql(" WHERE ");
        s.push_identifier("bar").unwrap();
        s.push_sql(" = ");
        s.push_static_arg(string_arg("frobnicate"));
        s.push_sql(" OR ");
        s.push_static_arg(string_arg("swoogle"));
        s.push_sql(" OR ");
        s.push_identifier("bar").unwrap();
        s.push_sql(" = ");
        s.push_typed_value(&TypedValue::Double(1.0.into())).unwrap();
        let q = s.finish();

        assert_eq!(
            q.sql.as_str(),
            "SELECT `foo` WHERE `bar` = $v0 OR $v1 OR `bar` = 1e0"
        );
        assert_eq!(
            q.args,
            vec![
                ("$v0".to_string(), string_arg("frobnicate")),
                ("$v1".to_string(), string_arg("swoogle"))
            ]
        );
    }
}
