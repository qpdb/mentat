// Copyright 2016 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

extern crate chrono;
extern crate itertools;
extern crate num;
extern crate ordered_float;
extern crate peg;
extern crate pretty;
extern crate uuid;

#[cfg(feature = "serde_support")]
extern crate serde;

#[cfg(feature = "serde_support")]
#[macro_use]
extern crate serde_derive;

pub mod entities;
pub mod intern_set;
pub use crate::intern_set::InternSet;
// Intentionally not pub.
pub mod matcher;
mod namespaceable_name;
pub mod pretty_print;
pub mod query;
pub mod symbols;
pub mod types;
pub mod utils;
pub mod value_rc;
pub use crate::value_rc::{Cloned, FromRc, ValueRc};

// Re-export the types we use.
pub use chrono::{DateTime, Utc};
pub use num::BigInt;
pub use ordered_float::OrderedFloat;
pub use uuid::Uuid;

// Export from our modules.
pub use crate::types::{
    FromMicros, FromMillis, Span, SpannedValue, ToMicros, ToMillis, Value, ValueAndSpan,
};

pub use crate::symbols::{Keyword, NamespacedSymbol, PlainSymbol};

use std::collections::{BTreeMap, BTreeSet, LinkedList};
use std::f64::{INFINITY, NAN, NEG_INFINITY};
use std::iter::FromIterator;

use chrono::TimeZone;

use crate::entities::*;
use crate::query::FromValue;

// Goal: Be able to parse https://github.com/edn-format/edn
// Also extensible to help parse http://docs.datomic.com/query.html

// Debugging hint: test using `cargo test --features peg/trace -- --nocapture`
// to trace where the parser is failing

// TODO: Support tagged elements
// TODO: Support discard

pub type ParseError = peg::error::ParseError<peg::str::LineCol>;

peg::parser!(pub grammar parse() for str {

    pub rule nil() -> SpannedValue = "nil" { SpannedValue::Nil }
    pub rule nan() -> SpannedValue = "#f" whitespace()+ "NaN" { SpannedValue::Float(OrderedFloat(NAN)) }

    pub rule infinity() -> SpannedValue = "#f" whitespace()+ s:$(sign()) "Infinity"
        { SpannedValue::Float(OrderedFloat(if s == "+" { INFINITY } else { NEG_INFINITY })) }

    pub rule boolean() -> SpannedValue
        = "true"  { SpannedValue::Boolean(true) }
        / "false" { SpannedValue::Boolean(false) }

    rule digit() = ['0'..='9']
    rule alphanumeric() = ['0'..='9' | 'a'..='z' | 'A'..='Z']
    rule octaldigit() = ['0'..='7']
    rule validbase() = ['3']['0'..='6'] / ['1' | '2']['0'..='9'] / ['2'..='9']
    rule hex() = ['0'..='9' | 'a'..='f' | 'A'..='F']
    rule sign() = ['+' | '-']

    pub rule raw_bigint() -> BigInt = b:$( sign()? digit()+ ) "N"
        { b.parse::<BigInt>().unwrap() }
    pub rule raw_octalinteger() -> i64 = "0" i:$( octaldigit()+ )
        { i64::from_str_radix(i, 8).unwrap() }
    pub rule raw_hexinteger() -> i64 = "0x" i:$( hex()+ )
        { i64::from_str_radix(i, 16).unwrap() }
    pub rule raw_basedinteger() -> i64 = b:$( validbase() ) "r" i:$( alphanumeric()+ )
        { i64::from_str_radix(i, b.parse::<u32>().unwrap()).unwrap() }
    pub rule raw_integer() -> i64 = i:$( sign()? digit()+ ) !("." / (['e' | 'E']))
        { i.parse::<i64>().unwrap() }
    pub rule raw_float() -> OrderedFloat<f64> = f:$(sign()? digit()+ ("." digit()+)? (['e' | 'E'] sign()? digit()+)?)
        { OrderedFloat(f.parse::<f64>().unwrap()) }

    pub rule bigint() -> SpannedValue = v:raw_bigint() { SpannedValue::BigInteger(v) }
    pub rule octalinteger() -> SpannedValue = v:raw_octalinteger() { SpannedValue::Integer(v) }
    pub rule hexinteger() -> SpannedValue = v:raw_hexinteger() { SpannedValue::Integer(v) }
    pub rule basedinteger() -> SpannedValue = v:raw_basedinteger() { SpannedValue::Integer(v) }
    pub rule integer() -> SpannedValue = v:raw_integer() { SpannedValue::Integer(v) }
    pub rule float() -> SpannedValue = v:raw_float() { SpannedValue::Float(v) }

    rule number() -> SpannedValue = ( bigint() / basedinteger() / hexinteger() / octalinteger() / integer() / float() )

    // TODO: standalone characters: \<char>, \newline, \return, \space and \tab.
    // rule string_standalone_chars() ->
    rule string_special_char() -> &'input str = "\\" c:$(['\\' | '"' | 'n' | 't' | 'r']) { c }
    rule string_normal_chars() -> &'input str = c:$((!['\"' | '\\'][_])+) { c }

    // This is what we need to do in order to unescape. We can't just match the entire string slice:
    // we get a Vec<&str> from rust-peg, where some parts might be unescaped special characters and
    // we join it together to form an output string.
    // E.g., input = r#"\"foo\\\\bar\""#
    //      output = [quote, "foo", backslash, "bar", quote]
    //      result = r#""foo\\bar""#
    // For the typical case, string_normal_chars will match multiple, leading to a single-element vec.
    pub rule raw_text() -> String = "\"" t:((string_special_char() / string_normal_chars())*) "\""
        {  t.join(&"") }

    pub rule text() -> SpannedValue
        = v:raw_text() { SpannedValue::Text(v) }

    // RFC 3339 timestamps. #inst "1985-04-12T23:20:50.52Z"
    // We accept an arbitrary depth of decimals.
    // TODO: Note that we discard the timezone information -- all times are translated to UTC.  Should we?
    rule inst_string() -> DateTime<Utc> =
        "#inst" whitespace()+ "\"" d:$( ['0'..='9']*<4> "-" ['0'..='2']['0'..='9'] "-" ['0'..='3']['0'..='9']
                "T"
                ['0'..='2']['0'..='9'] ":" ['0'..='5']['0'..='9'] ":" ['0'..='6']['0'..='9']
                ("." ['0'..='9']+)?
                ("Z" / (("+" / "-") ['0'..='2']['0'..='9'] ":" ['0'..='5']['0'..='9']))
            )
        "\"" {?
            DateTime::parse_from_rfc3339(d)
                .map(|t| t.with_timezone(&Utc))
                .map_err(|_| "invalid datetime")        // TODO Oh, rustpeg.
        }

    rule inst_micros() -> DateTime<Utc> =
        "#instmicros" whitespace()+ d:$( digit()+ ) {
            let micros = d.parse::<i64>().unwrap();
            let seconds: i64 = micros / 1_000_000;
            let nanos: u32 = ((micros % 1_000_000).abs() as u32) * 1000;
            Utc.timestamp(seconds, nanos)
        }

    rule inst_millis() -> DateTime<Utc> =
        "#instmillis" whitespace()+ d:$( digit()+ ) {
            let millis = d.parse::<i64>().unwrap();
            let seconds: i64 = millis / 1000;
            let nanos: u32 = ((millis % 1000).abs() as u32) * 1_000_000;
            Utc.timestamp(seconds, nanos)
        }

    rule inst() -> SpannedValue = t:(inst_millis() / inst_micros() / inst_string())
        { SpannedValue::Instant(t) }

    rule uuid_string() -> Uuid =
        "\"" u:$( ['a'..='f' | '0'..='9']*<8> "-" ['a'..='f' | '0'..='9']*<4> "-" ['a'..='f' | '0'..='9']*<4> "-" ['a'..='f' | '0'..='9']*<4> "-" ['a'..='f' | '0'..='9']*<12> ) "\"" {
            Uuid::parse_str(u).expect("this is a valid UUID string")
        }

    pub rule uuid() -> SpannedValue = "#uuid" whitespace()+ u:uuid_string()
        { SpannedValue::Uuid(u) }

    rule namespace_divider() = "."
    rule namespace_separator() = "/"

    // TODO: Be more picky here.
    // Keywords follow the rules of symbols, except they can (and must) begin with :
    // e.g. :fred or :my/fred. See https://github.com/edn-format/edn#keywords
    rule symbol_char_initial() = ['a'..='z' | 'A'..='Z' | '0'..='9' | '*' | '!' | '_' | '?' | '$' | '%' | '&' | '=' | '<' | '>']
    rule symbol_char_subsequent() = ['+' | 'a'..='z' | 'A'..='Z' | '0'..='9' | '*' | '!' | '_' | '?' | '$' | '%' | '&' | '=' | '<' | '>' | '-']

    rule symbol_namespace() = symbol_char_initial() symbol_char_subsequent()* (namespace_divider() symbol_char_subsequent()+)*
    rule symbol_name() = ( symbol_char_initial()+ symbol_char_subsequent()* )
    rule plain_symbol_name() = symbol_name() / "..." / "."

    rule keyword_prefix() = ":"

    pub rule symbol() -> SpannedValue =
        ns:( sns:$(symbol_namespace()) namespace_separator() { sns })?
        n:$(plain_symbol_name())
        { SpannedValue::from_symbol(ns, n) }
        / expected!("symbol")

    pub rule keyword() -> SpannedValue =
        keyword_prefix()
        ns:( sns:$(symbol_namespace()) namespace_separator() { sns })?
        n:$(symbol_name())
        { SpannedValue::from_keyword(ns, n) }
        / expected!("keyword")

    pub rule list() -> SpannedValue = "(" __ v:(value())* __ ")"
        { SpannedValue::List(LinkedList::from_iter(v)) }

    pub rule vector() -> SpannedValue = "[" __ v:(value())* __ "]"
        { SpannedValue::Vector(v) }

    pub rule set() -> SpannedValue = "#{" __ v:(value())* __ "}"
        { SpannedValue::Set(BTreeSet::from_iter(v)) }

    pub rule pair() -> (ValueAndSpan, ValueAndSpan) =
        k:(value()) v:(value()) {
            (k, v)
        }

    pub rule map() -> SpannedValue = "{" __ v:(pair())* __ "}"
        { SpannedValue::Map(BTreeMap::from_iter(v)) }

    // Note: It's important that float comes before integer or the parser assumes that floats are integers and fails to parse.
    pub rule value() -> ValueAndSpan =
        __ start:position!() v:(nil() / nan() / infinity() / boolean() / number() / inst() / uuid() / text() / keyword() / symbol() / list() / vector() / map() / set()) end:position!() __ {
            ValueAndSpan {
                inner: v,
                span: Span::new(start, end)
            }
        }
        / expected!("value")

    rule atom() -> ValueAndSpan
        = v:value() {? if v.is_atom() { Ok(v) } else { Err("expected atom") } }

    // Clojure (and thus EDN) regards commas as whitespace, and thus the two-element vectors [1 2] and
    // [1,,,,2] are equivalent, as are the maps {:a 1, :b 2} and {:a 1 :b 2}.
    rule whitespace() = quiet!{[' ' | '\r' | '\n' | '\t' | ',']}
    rule comment() = quiet!{";" (!['\r' | '\n'][_])* ['\r' | '\n']?}

    rule __() = (whitespace() / comment())*

    // Transaction entity parser starts here.

    pub rule op() -> OpType
        = ":db/add"     { OpType::Add }
        / ":db/retract" { OpType::Retract }

    rule raw_keyword() -> Keyword =
        keyword_prefix()
        ns:( sns:$(symbol_namespace()) namespace_separator() { sns })?
        n:$(symbol_name()) {
            match ns {
                Some(ns) => Keyword::namespaced(ns, n),
                None => Keyword::plain(n),
            }
        }
        / expected!("keyword")

    rule raw_forward_keyword() -> Keyword
        = v:raw_keyword() {? if v.is_forward() { Ok(v) } else { Err("expected :forward or :forward/keyword") } }

    rule raw_backward_keyword() -> Keyword
        = v:raw_keyword() {? if v.is_backward() { Ok(v) } else { Err("expected :_backward or :backward/_keyword") } }

    rule raw_namespaced_keyword() -> Keyword
        = keyword_prefix() ns:$(symbol_namespace()) namespace_separator() n:$(symbol_name()) { Keyword::namespaced(ns, n) }
        / expected!("namespaced keyword")

    rule raw_forward_namespaced_keyword() -> Keyword
        = v:raw_namespaced_keyword() {? if v.is_forward() { Ok(v) } else { Err("expected namespaced :forward/keyword") } }

    rule raw_backward_namespaced_keyword() -> Keyword
        = v:raw_namespaced_keyword() {? if v.is_backward() { Ok(v) } else { Err("expected namespaced :backward/_keyword") } }

    rule entid() -> EntidOrIdent
        = v:( raw_basedinteger() / raw_hexinteger() / raw_octalinteger() / raw_integer() ) { EntidOrIdent::Entid(v) }
        / v:raw_namespaced_keyword() { EntidOrIdent::Ident(v) }
        / expected!("entid")

    rule forward_entid() -> EntidOrIdent
        = v:( raw_basedinteger() / raw_hexinteger() / raw_octalinteger() / raw_integer() ) { EntidOrIdent::Entid(v) }
        / v:raw_forward_namespaced_keyword() { EntidOrIdent::Ident(v) }
        / expected!("forward entid")

    rule backward_entid() -> EntidOrIdent
        = v:raw_backward_namespaced_keyword() { EntidOrIdent::Ident(v.to_reversed()) }
        / expected!("backward entid")

    rule lookup_ref() -> LookupRef<ValueAndSpan>
        = "(" __ "lookup-ref" __ a:(entid()) __ v:(value()) __ ")" { LookupRef { a: AttributePlace::Entid(a), v } }
        / expected!("lookup-ref")

    rule tx_function() -> TxFunction
        = "(" __ n:$(symbol_name()) __ ")" { TxFunction { op: PlainSymbol::plain(n) } }

    rule entity_place() -> EntityPlace<ValueAndSpan>
        = v:raw_text() { EntityPlace::TempId(TempId::External(v).into()) }
        / v:entid() { EntityPlace::Entid(v) }
        / v:lookup_ref() { EntityPlace::LookupRef(v) }
        / v:tx_function() { EntityPlace::TxFunction(v) }

    rule value_place_pair() -> (EntidOrIdent, ValuePlace<ValueAndSpan>)
        = k:(entid()) __ v:(value_place()) { (k, v) }

    rule map_notation() -> MapNotation<ValueAndSpan>
        = "{" __ kvs:(value_place_pair()*) __ "}" { kvs.into_iter().collect() }

    rule value_place() -> ValuePlace<ValueAndSpan>
        = __ v:lookup_ref() __ { ValuePlace::LookupRef(v) }
        / __ v:tx_function() __ { ValuePlace::TxFunction(v) }
        / __ "[" __ vs:(value_place()*) __ "]" __ { ValuePlace::Vector(vs) }
        / __ v:map_notation() __ { ValuePlace::MapNotation(v) }
        / __ v:atom() __ { ValuePlace::Atom(v) }

    pub rule entity() -> Entity<ValueAndSpan>
        = __ "[" __ op:(op()) __ e:(entity_place()) __ a:(forward_entid())  __ v:(value_place()) __  "]" __ { Entity::AddOrRetract { op, e, a: AttributePlace::Entid(a), v } }
        / __ "[" __ op:(op()) __ e:(value_place())  __ a:(backward_entid()) __ v:(entity_place()) __ "]" __ { Entity::AddOrRetract { op, e: v, a: AttributePlace::Entid(a), v: e } }
        / __ map:map_notation() __ { Entity::MapNotation(map) }
        / expected!("entity")

    pub rule entities() -> Vec<Entity<ValueAndSpan>>
        = __ "[" __ es:(entity()*) __ "]" __ { es }

    // Query parser starts here.
    //
    // We expect every rule except the `raw_*` rules to eat whitespace
    // (with `__`) at its start and finish.  That means that every string
    // pattern (say "[") should be bracketed on either side with either a
    // whitespace-eating rule or an explicit whitespace eating `__`.

    rule query_function() -> query::QueryFunction
        = __ n:$(symbol_name()) __ {? query::QueryFunction::from_symbol(&PlainSymbol::plain(n)).ok_or("expected query function") }

    rule fn_arg() -> query::FnArg
        = v:value() {? query::FnArg::from_value(&v).ok_or("expected query function argument") }
        / __ "[" args:fn_arg()+ "]" __ { query::FnArg::Vector(args) }

    rule find_elem() -> query::Element
        = __ v:variable() __ { query::Element::Variable(v) }
        / __ "(" __ "the" v:variable() ")" __ { query::Element::Corresponding(v) }
        / __ "(" __ "pull" var:variable() "[" patterns:pull_attribute()+ "]" __ ")" __ { query::Element::Pull(query::Pull { var, patterns }) }
        / __ "(" func:query_function() args:fn_arg()* ")" __ { query::Element::Aggregate(query::Aggregate { func, args }) }

    rule find_spec() -> query::FindSpec
        = f:find_elem() "." __ { query::FindSpec::FindScalar(f) }
        / fs:find_elem()+ { query::FindSpec::FindRel(fs) }
        / __ "[" f:find_elem() __ "..." __ "]" __ { query::FindSpec::FindColl(f) }
        / __ "[" fs:find_elem()+ "]" __ { query::FindSpec::FindTuple(fs) }

    rule pull_attribute() -> query::PullAttributeSpec
        = __ "*" __ { query::PullAttributeSpec::Wildcard }
        / __ k:raw_forward_namespaced_keyword() __ alias:(":as" __ alias:raw_forward_keyword() __ { alias })? {
            let attribute = query::PullConcreteAttribute::Ident(::std::rc::Rc::new(k));
            let alias = alias.map(::std::rc::Rc::new);
            query::PullAttributeSpec::Attribute(
                query::NamedPullAttribute {
                    attribute,
                    alias,
                })
        }

    rule limit() -> query::Limit
        = __ v:variable() __ { query::Limit::Variable(v) }
        / __ n:(raw_octalinteger() / raw_hexinteger() / raw_basedinteger() / raw_integer()) __ {?
            if n > 0 {
                Ok(query::Limit::Fixed(n as u64))
            } else {
                Err("expected positive integer")
            }
        }

    rule order() -> query::Order
        = __ "(" __ "asc" v:variable() ")" __ { query::Order(query::Direction::Ascending, v) }
        / __ "(" __ "desc" v:variable() ")" __ { query::Order(query::Direction::Descending, v) }
        / v:variable() { query::Order(query::Direction::Ascending, v) }


    rule pattern_value_place() -> query::PatternValuePlace
        = v:value() {? query::PatternValuePlace::from_value(&v).ok_or("expected pattern_value_place") }

    rule pattern_non_value_place() -> query::PatternNonValuePlace
        = v:value() {? query::PatternNonValuePlace::from_value(&v).ok_or("expected pattern_non_value_place") }

    rule pattern() -> query::WhereClause
        = __ "["
          src:src_var()?
          e:pattern_non_value_place()
          a:pattern_non_value_place()
          v:pattern_value_place()?
          tx:pattern_non_value_place()?
        "]" __
        {?
            let v = v.unwrap_or(query::PatternValuePlace::Placeholder);
            let tx = tx.unwrap_or(query::PatternNonValuePlace::Placeholder);

            // Pattern::new takes care of reversal of reversed
            // attributes: [?x :foo/_bar ?y] turns into
            // [?y :foo/bar ?x].
            //
            // This is a bit messy: the inner conversion to a Pattern can
            // fail if the input is something like
            //
            // ```edn
            // [?x :foo/_reversed 23.4]
            // ```
            //
            // because
            //
            // ```edn
            // [23.4 :foo/reversed ?x]
            // ```
            //
            // is nonsense. That leaves us with a nested optional, which we unwrap here.
            query::Pattern::new(src, e, a, v, tx)
                .map(query::WhereClause::Pattern)
                .ok_or("expected pattern")
        }

    // TODO: This shouldn't be checked at parse time.
    rule rule_vars() -> BTreeSet<query::Variable>
        = vs:variable()+ {?
            let given = vs.len();
            let set: BTreeSet<query::Variable> = vs.into_iter().collect();
            if given != set.len() {
                Err("expected unique variables")
            } else {
                Ok(set)
            }
        }

    rule or_pattern_clause() -> query::OrWhereClause
        = clause:where_clause() { query::OrWhereClause::Clause(clause) }

    rule or_and_clause() -> query::OrWhereClause
        = __ "(" __ "and" clauses:where_clause()+ ")" __ { query::OrWhereClause::And(clauses) }

    rule or_where_clause() -> query::OrWhereClause
        = or_pattern_clause()
        / or_and_clause()

    rule or_clause() -> query::WhereClause
        = __ "(" __ "or" clauses:or_where_clause()+ ")" __ {
             query::WhereClause::OrJoin(query::OrJoin::new(query::UnifyVars::Implicit, clauses))
        }

    rule or_join_clause() -> query::WhereClause
        = __ "(" __ "or-join" __ "[" vars:rule_vars() "]" clauses:or_where_clause()+ ")" __ {
             query::WhereClause::OrJoin(query::OrJoin::new(query::UnifyVars::Explicit(vars), clauses))
        }

    rule not_clause() -> query::WhereClause
        = __ "(" __ "not" clauses:where_clause()+ ")" __ {
             query::WhereClause::NotJoin(query::NotJoin::new(query::UnifyVars::Implicit, clauses))
        }

    rule not_join_clause() -> query::WhereClause
        = __ "(" __ "not-join" __ "[" vars:rule_vars() "]" clauses:where_clause()+ ")" __ {
             query::WhereClause::NotJoin(query::NotJoin::new(query::UnifyVars::Explicit(vars), clauses))
        }

    rule type_annotation() -> query::WhereClause
        = __ "[" __ "(" __ "type" var:variable() __ ty:raw_keyword() __ ")" __ "]" __ {
            query::WhereClause::TypeAnnotation(
                query::TypeAnnotation {
                    value_type: ty,
                    variable: var,
                })
        }

    rule pred() -> query::WhereClause
        = __ "[" __ "(" func:query_function() args:fn_arg()* ")" __ "]" __ {
            query::WhereClause::Pred(
                query::Predicate {
                    operator: func.0,
                    args,
                })
        }

    pub rule where_fn() -> query::WhereClause
        = __ "[" __ "(" func:query_function() args:fn_arg()* ")" __ binding:binding() "]" __ {
            query::WhereClause::WhereFn(
                query::WhereFn {
                    operator: func.0,
                    args,
                    binding,
                })
        }

    rule where_clause() -> query::WhereClause
        // Right now we only support patterns and predicates. See #239 for more.
        = pattern()
        / or_join_clause()
        / or_clause()
        / not_join_clause()
        / not_clause()
        / type_annotation()
        / pred()
        / where_fn()

    rule query_part() -> query::QueryPart
        = __ ":find" fs:find_spec() { query::QueryPart::FindSpec(fs) }
        / __ ":in" in_vars:variable()+ { query::QueryPart::InVars(in_vars) }
        / __ ":limit" l:limit() { query::QueryPart::Limit(l) }
        / __ ":order" os:order()+ { query::QueryPart::Order(os) }
        / __ ":where" ws:where_clause()+ { query::QueryPart::WhereClauses(ws) }
        / __ ":with" with_vars:variable()+ { query::QueryPart::WithVars(with_vars) }

    pub rule parse_query() -> query::ParsedQuery
        = __ "[" qps:query_part()+ "]" __ {? query::ParsedQuery::from_parts(qps) }

    rule variable() -> query::Variable
        = v:value() {? query::Variable::from_value(&v).ok_or("expected variable") }

    rule src_var() -> query::SrcVar
        = v:value() {? query::SrcVar::from_value(&v).ok_or("expected src_var") }

    rule variable_or_placeholder() -> query::VariableOrPlaceholder
        = v:variable() { query::VariableOrPlaceholder::Variable(v) }
        / __ "_" __ { query::VariableOrPlaceholder::Placeholder }

    rule binding() -> query::Binding
        = __ "[" __ "[" vs:variable_or_placeholder()+ "]" __ "]" __ { query::Binding::BindRel(vs) }
        / __ "[" v:variable() "..." __ "]" __ { query::Binding::BindColl(v) }
        / __ "[" vs:variable_or_placeholder()+ "]" __ { query::Binding::BindTuple(vs) }
        / v:variable() { query::Binding::BindScalar(v) }

});
