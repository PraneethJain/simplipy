use rustpython_parser::ast::bigint::BigInt;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Rem, Sub};

pub type Env = Vec<FlatEnv>;
pub type Stack = Vec<ApplicationClosure>;
pub type Store = Vec<StorableValue>;
pub type ClassEnvs = Vec<(usize, FlatEnv)>;

#[derive(Debug, Clone, PartialEq)]
pub struct FlatEnv {
    pub mapping: BTreeMap<String, usize>,
    pub func_name: String,
}

impl FlatEnv {
    pub fn new(mapping: BTreeMap<String, usize>, func_name: String) -> Self {
        Self { mapping, func_name }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationClosure(pub usize, pub Env, pub ClassEnvs);
#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionClosure(pub usize, pub Env);

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub class: Option<usize>,
    pub flat_env_addr: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StorableValue {
    Bottom,
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    DefinitionClosure(DefinitionClosure),
    FlatEnv(FlatEnv),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub lineno: usize,
    pub env: Env,
    pub stack: Stack,
    pub store: Store,
    pub class_envs: ClassEnvs,
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

    pub fn bool(self) -> Option<bool> {
        if let StorableValue::Bool(bool_val) = self {
            Some(bool_val)
        } else {
            None
        }
    }

    pub fn closure(self) -> Option<DefinitionClosure> {
        if let StorableValue::DefinitionClosure(closure) = self {
            Some(closure)
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

    pub fn as_flat_env(&self) -> Option<&FlatEnv> {
        if let StorableValue::FlatEnv(flat_env) = self {
            Some(flat_env)
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

    pub fn as_mut_flat_env(&mut self) -> Option<&mut FlatEnv> {
        if let StorableValue::FlatEnv(flat_env) = self {
            Some(flat_env)
        } else {
            None
        }
    }
}
