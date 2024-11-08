use rustpython_parser::ast::bigint::BigInt;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Sub};

pub type Stack = Vec<Context>;
pub type Store = Vec<StorableValue>;
pub type Env = BTreeMap<String, usize>;

#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Lexical(usize, Option<Env>),
    Class(usize, Env),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub metadata: ObjectMetadata,
    pub env_addr: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMetadata {
    pub class: Option<usize>,
    pub mro: Option<Vec<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StorableValue {
    Bottom,
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    DefinitionClosure(usize, Option<Env>, Vec<String>),
    Env(Env),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub lineno: usize,
    pub global_env: Env,
    pub local_env: Option<Env>,
    pub stack: Stack,
    pub store: Store,
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
            | StorableValue::DefinitionClosure(_, _, _)
            | StorableValue::Env(_)
            | StorableValue::Object(_) => None,
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
            | StorableValue::DefinitionClosure(_, _, _)
            | StorableValue::Env(_)
            | StorableValue::Object(_) => None,
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

    pub fn as_object(&self) -> Option<&Object> {
        if let StorableValue::Object(object) = self {
            Some(object)
        } else {
            None
        }
    }

    pub fn as_env(&self) -> Option<&Env> {
        if let StorableValue::Env(env) = self {
            Some(env)
        } else {
            None
        }
    }

    pub fn as_mut_object(&mut self) -> Option<&mut Object> {
        if let StorableValue::Object(object) = self {
            Some(object)
        } else {
            None
        }
    }

    pub fn as_mut_env(&mut self) -> Option<&mut Env> {
        if let StorableValue::Env(env) = self {
            Some(env)
        } else {
            None
        }
    }
}
