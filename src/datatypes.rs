use rustpython_parser::ast::bigint::BigInt;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Sub};

pub type Stack = Vec<LexicalContext>;
pub type EnvId = usize;
pub type Env = BTreeMap<String, StorableValue>;
pub type Envs = BTreeMap<EnvId, Env>;
pub type Parent = BTreeMap<EnvId, EnvId>;
pub type LexicalContext = (usize, EnvId);

#[derive(Clone, Debug, PartialEq)]
pub enum StorableValue {
    Bottom,
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    DefinitionClosure(usize, EnvId, Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub envs: Envs,
    pub parent: Parent,
    pub stack: Stack,
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

impl Not for StorableValue {
    type Output = Option<StorableValue>;

    fn not(self) -> Option<StorableValue> {
        match self {
            StorableValue::Bool(b) => Some(StorableValue::Bool(!b)),
            StorableValue::Bottom
            | StorableValue::None
            | StorableValue::Int(_)
            | StorableValue::Float(_)
            | StorableValue::String(_)
            | StorableValue::DefinitionClosure(_, _, _) => None,
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

impl Neg for StorableValue {
    type Output = Option<StorableValue>;

    fn neg(self) -> Option<StorableValue> {
        match self {
            StorableValue::Int(a) => Some(StorableValue::Int(-a)),
            StorableValue::Float(f) => Some(StorableValue::Float(-f)),
            StorableValue::Bottom
            | StorableValue::None
            | StorableValue::Bool(_)
            | StorableValue::String(_)
            | StorableValue::DefinitionClosure(_, _, _) => None,
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

    pub fn bool(self) -> Option<bool> {
        if let StorableValue::Bool(bool_val) = self {
            Some(bool_val)
        } else {
            None
        }
    }
}
