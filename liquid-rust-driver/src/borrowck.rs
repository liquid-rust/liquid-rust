// This code is adapted from the
// [Polonius](https://github.com/rust-lang-nursery/polonius/blob/master/src/facts.rs)
// and [Prusti](https://github.com/viperproject/prusti-dev/blob/master/prusti-interface/src/environment/borrowck/facts.rs)
// source codes. Since the latter is licensed under MPL, its license is
// reproduced below:
//
// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module defines utilities for interacting with nll facts,
//! collected during Rust compilation.

use csv::ReaderBuilder;
use hashconsing::{HConsign, HashConsign};
use lazy_static::lazy_static;
use liquid_rust_common::index::newtype_index;
use polonius_engine;
use regex::Regex;
use rustc_index::vec::Idx;
use rustc_middle::mir;
use serde::de::{Deserialize, DeserializeOwned, Deserializer};
use std::{path::Path, str::FromStr};

/// Macro for declaring index types for referencing interned facts.
macro_rules! index_type {
    ($typ:ident, $debug_str:ident) => {
        newtype_index! {
            pub struct $typ {
                DEBUG_FORMAT = concat!(stringify!($debug_str), "_{}")
            }
        }

        impl InternTo<$typ> for String {
            fn intern(_interner: &mut Interner, from: String) -> $typ {
                from.parse().unwrap()
            }
        }

        impl polonius_engine::Atom for $typ {
            fn index(self) -> usize {
                self.into()
            }
        }
    };
}

// A unique identifier of a loan.
index_type!(Loan, L);
// A unique identifier of a region.
index_type!(Region, R);
// A unique identifier of a variable.
index_type!(Variable, V);
// A unique identifier of a path.
index_type!(Place, X);

/// From https://github.com/rust-lang/polonius/blob/master/src/intern.rs:
/// When we load facts out of the table, they are essentially random
/// strings. We create an intern table to map those to integers.
pub struct Interner {
    pub points: HConsign<Point>,
}

impl Interner {
    pub fn new() -> Self {
        Self {
            points: HConsign::empty(),
        }
    }
}

pub trait InternTo<To> {
    fn intern(interner: &mut Interner, input: Self) -> To;
}

impl<A, FromA, B, FromB> InternTo<(A, B)> for (FromA, FromB)
where
    FromA: InternTo<A>,
    FromB: InternTo<B>,
{
    fn intern(tables: &mut Interner, input: (FromA, FromB)) -> (A, B) {
        let (from_a, from_b) = input;
        (FromA::intern(tables, from_a), FromB::intern(tables, from_b))
    }
}

impl<A, FromA, B, FromB, C, FromC> InternTo<(A, B, C)> for (FromA, FromB, FromC)
where
    FromA: InternTo<A>,
    FromB: InternTo<B>,
    FromC: InternTo<C>,
{
    fn intern(tables: &mut Interner, input: (FromA, FromB, FromC)) -> (A, B, C) {
        let (from_a, from_b, from_c) = input;
        (
            FromA::intern(tables, from_a),
            FromB::intern(tables, from_b),
            FromC::intern(tables, from_c),
        )
    }
}

impl<A, FromA, B, FromB, C, FromC, D, FromD> InternTo<(A, B, C, D)> for (FromA, FromB, FromC, FromD)
where
    FromA: InternTo<A>,
    FromB: InternTo<B>,
    FromC: InternTo<C>,
    FromD: InternTo<D>,
{
    fn intern(tables: &mut Interner, input: (FromA, FromB, FromC, FromD)) -> (A, B, C, D) {
        let (from_a, from_b, from_c, from_d) = input;
        (
            FromA::intern(tables, from_a),
            FromB::intern(tables, from_b),
            FromC::intern(tables, from_c),
            FromD::intern(tables, from_d),
        )
    }
}

/// The type of the point. Either the start of a statement or in the
/// middle of it.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum PointType {
    Start,
    Mid,
}

impl std::cmp::PartialOrd for PointType {
    fn partial_cmp(&self, other: &PointType) -> Option<std::cmp::Ordering> {
        let res = match (self, other) {
            (PointType::Start, PointType::Start) => std::cmp::Ordering::Equal,
            (PointType::Start, PointType::Mid) => std::cmp::Ordering::Less,
            (PointType::Mid, PointType::Start) => std::cmp::Ordering::Greater,
            (PointType::Mid, PointType::Mid) => std::cmp::Ordering::Equal,
        };
        Some(res)
    }
}

impl std::cmp::Ord for PointType {
    fn cmp(&self, other: &PointType) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug)]
pub struct UnknownPointTypeError(String);

impl FromStr for PointType {
    type Err = UnknownPointTypeError;

    fn from_str(point_type: &str) -> Result<Self, Self::Err> {
        match point_type {
            "Start" => Ok(PointType::Start),
            "Mid" => Ok(PointType::Mid),
            _ => Err(UnknownPointTypeError(String::from(point_type))),
        }
    }
}

/// A program point used in the borrow checker analysis.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone)]
pub struct Point {
    pub location: mir::Location,
    pub typ: PointType,
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}:{:?}:{:?}",
            self.location.block, self.location.statement_index, self.typ
        )
    }
}

impl FromStr for Point {
    type Err = ();

    fn from_str(point: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"^(?P<type>Mid|Start)\(bb(?P<bb>\d+)\[(?P<stmt>\d+)\]\)$").unwrap();
        }
        let caps = RE.captures(point).unwrap();
        let point_type: PointType = caps["type"].parse().unwrap();
        let basic_block: usize = caps["bb"].parse().unwrap();
        let statement_index: usize = caps["stmt"].parse().unwrap();
        Ok(Self {
            location: mir::Location {
                block: mir::BasicBlock::new(basic_block),
                statement_index,
            },
            typ: point_type,
        })
    }
}

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(FromStr::from_str(&s).unwrap())
    }
}

newtype_index! {
    pub struct PointIndex {
        DEBUG_FORMAT = "pi_{}"
    }
}

impl InternTo<PointIndex> for String {
    fn intern(interner: &mut Interner, input: Self) -> PointIndex {
        let p = input.parse().unwrap();
        PointIndex::new(interner.points.mk(p).uid() as usize)
    }
}

impl polonius_engine::Atom for PointIndex {
    fn index(self) -> usize {
        self.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LrFactTypes;

impl polonius_engine::FactTypes for LrFactTypes {
    type Origin = Region;
    type Loan = Loan;
    type Point = PointIndex;
    type Variable = Variable;
    type Path = Place;
}

pub type AllFacts = polonius_engine::AllFacts<LrFactTypes>;

impl FromStr for Region {
    type Err = ();

    fn from_str(region: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^'_#(?P<id>\d+)r$").unwrap();
        }
        println!("{}", region);
        let caps = RE.captures(region).unwrap();
        let id: usize = caps["id"].parse().unwrap();
        Ok(Self::new(id))
    }
}

impl FromStr for Loan {
    type Err = ();

    fn from_str(loan: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^bw(?P<id>\d+)$").unwrap();
        }
        let caps = RE.captures(loan).unwrap();
        let id: usize = caps["id"].parse().unwrap();
        Ok(Self::new(id))
    }
}

impl FromStr for Variable {
    type Err = ();

    fn from_str(variable: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(variable[1..].parse().unwrap()))
    }
}

impl FromStr for Place {
    type Err = ();

    fn from_str(place: &str) -> Result<Self, Self::Err> {
        assert_eq!(place.chars().nth(0).unwrap(), 'm');
        assert_eq!(place.chars().nth(1).unwrap(), 'p');
        Ok(Self::new(place[2..].parse().unwrap()))
    }
}

pub struct FactLoader {
    pub interner: Interner,
    pub facts: AllFacts,
}

impl FactLoader {
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            facts: AllFacts::default(),
        }
    }

    pub fn load_facts<T: DeserializeOwned + InternTo<U>, U>(
        &mut self,
        facts_dir: &Path,
        facts_type: &str,
    ) -> Vec<U> {
        let filename = format!("{}.facts", facts_type);
        let facts_file = facts_dir.join(&filename);
        let mut reader = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(&facts_file)
            .unwrap_or_else(|err| panic!("failed to read file {:?} with err: {}", facts_file, err));

        let mut res: Vec<U> = vec![];

        for row in reader.deserialize() {
            let row: T = row.unwrap();
            res.push(InternTo::intern(&mut self.interner, row))
        }

        res
    }

    pub fn load_all_facts(&mut self, facts_dir: &Path) {
        macro_rules! add {
            ($cols:ty, $fact_type:ident) => {
                let f = self.load_facts::<$cols, _>(facts_dir, stringify!($fact_type));
                self.facts.$fact_type.extend(f);
            };
            ($cols:ty, $res:ty, $fact_type:ident) => {
                let f = self.load_facts::<$cols, $res>(facts_dir, stringify!($fact_type));
                self.facts.$fact_type.extend(f);
            };
        }

        add!(String, Region, universal_region);

        add!((String, String), cfg_edge);
        add!((String, String), killed);
        add!((String, String), invalidates);
        add!((String, String), var_used_at);
        add!((String, String), var_defined_at);
        add!((String, String), var_dropped_at);
        add!((String, String), use_of_var_derefs_origin);
        add!((String, String), drop_of_var_derefs_origin);
        add!((String, String), child_path);
        add!((String, String), path_is_var);
        add!((String, String), path_assigned_at_base);
        add!((String, String), path_moved_at_base);
        add!((String, String), path_accessed_at_base);
        add!((String, String), known_subset);
        add!((String, String), placeholder);

        add!((String, String, String), borrow_region);
        add!((String, String, String), outlives);
    }
}

pub fn load_facts(facts_dir: &Path) -> FactLoader {
    let mut fl = FactLoader::new();
    fl.load_all_facts(facts_dir);
    fl
}

