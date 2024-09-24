use rustpython_parser::ast::bigint::BigInt;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Rem, Sub};

pub type Env = Vec<LocalEnv>;
pub type Stack = Vec<Closure>;
pub type Store = Vec<StorableValue>;

#[derive(Debug, Clone, PartialEq)]
pub struct LocalEnv {
    pub mapping: BTreeMap<String, usize>,
    pub func_name: String,
}

impl LocalEnv {
    pub fn new(mapping: BTreeMap<String, usize>, func_name: String) -> Self {
        Self { mapping, func_name }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Closure {
    Function(usize, Env),
    Return(usize, Env),
    Except(usize, Env),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StorableValue {
    Bottom,
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    Closure(Closure),
}

impl PartialOrd for StorableValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        match (self, other) {
            (StorableValue::Bottom, StorableValue::Bottom) => Some(Ordering::Equal),
            (StorableValue::None, StorableValue::None) => Some(Ordering::Equal),
            (StorableValue::Bool(a), StorableValue::Bool(b)) => a.partial_cmp(b),
            (StorableValue::Int(a), StorableValue::Int(b)) => a.partial_cmp(b),
            (StorableValue::Float(a), StorableValue::Float(b)) => a.partial_cmp(b),
            (StorableValue::String(a), StorableValue::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
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

    pub fn as_bool(self) -> Option<bool> {
        if let StorableValue::Bool(bool_val) = self {
            Some(bool_val)
        } else {
            None
        }
    }

    pub fn as_closure(self) -> Option<Closure> {
        if let StorableValue::Closure(closure) = self {
            Some(closure)
        } else {
            None
        }
    }
}
