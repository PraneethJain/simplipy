use rustpython_parser::ast::bigint::BigInt;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Rem, Sub};

pub type LocalEnv = BTreeMap<String, usize>;
pub type Env = Vec<LocalEnv>;
pub type Closure = (u64, Env);
pub type Stack = Vec<Closure>;
pub type Store = Vec<StorableValue>;

#[derive(Clone, Debug, PartialEq)]
pub enum StorableValue {
    Bottom,
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    Closure(usize, Env),
}

impl Add for StorableValue {
    type Output = Option<StorableValue>;

    fn add(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => Some(StorableValue::Int(a + b)),
            (StorableValue::Float(a), StorableValue::Float(b)) => Some(StorableValue::Float(a + b)),
            (StorableValue::String(a), StorableValue::String(b)) => {
                Some(StorableValue::String(a + &b))
            }

            _ => None,
        }
    }
}

impl Sub for StorableValue {
    type Output = Option<StorableValue>;

    fn sub(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => Some(StorableValue::Int(a - b)),
            (StorableValue::Float(a), StorableValue::Float(b)) => Some(StorableValue::Float(a - b)),
            _ => None,
        }
    }
}

impl Mul for StorableValue {
    type Output = Option<StorableValue>;

    fn mul(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => Some(StorableValue::Int(a * b)),
            (StorableValue::Float(a), StorableValue::Float(b)) => Some(StorableValue::Float(a * b)),
            _ => None,
        }
    }
}

impl Div for StorableValue {
    type Output = Option<StorableValue>;

    fn div(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => {
                if b != BigInt::from(0) {
                    Some(StorableValue::Int(a / b))
                } else {
                    None
                }
            }
            (StorableValue::Float(a), StorableValue::Float(b)) => {
                if b != 0.0 {
                    Some(StorableValue::Float(a / b))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Rem for StorableValue {
    type Output = Option<StorableValue>;

    fn rem(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => {
                if b != BigInt::from(0) {
                    Some(StorableValue::Int(a % b))
                } else {
                    None
                }
            }
            (StorableValue::Float(a), StorableValue::Float(b)) => {
                if b != 0.0 {
                    Some(StorableValue::Float(a % b))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl StorableValue {
    pub fn floordiv(self, other: StorableValue) -> Option<StorableValue> {
        match (self, other) {
            (StorableValue::Int(a), StorableValue::Int(b)) => {
                if b != BigInt::from(0) {
                    Some(StorableValue::Int(a / b))
                } else {
                    None
                }
            }
            (StorableValue::Float(a), StorableValue::Float(b)) => {
                if b != 0.0 {
                    Some(StorableValue::Float((a / b).floor()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
