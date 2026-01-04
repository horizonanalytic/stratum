//! Runtime values for the Stratum virtual machine

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use regex::Regex as CompiledRegex;

use super::Chunk;

/// Database connection types supported by Stratum
#[derive(Clone)]
pub enum DbConnectionKind {
    /// SQLite connection
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
    /// PostgreSQL connection
    Postgres(Arc<Mutex<postgres::Client>>),
    /// MySQL connection
    MySql(Arc<Mutex<mysql::Conn>>),
    /// DuckDB connection
    DuckDb(Arc<Mutex<duckdb::Connection>>),
}

/// Database connection wrapper with metadata
#[derive(Clone)]
pub struct DbConnection {
    /// The underlying connection
    pub kind: DbConnectionKind,
    /// Database version string
    pub version: String,
    /// Connection identifier (for display purposes)
    pub identifier: String,
}

impl DbConnection {
    /// Create a new SQLite connection
    pub fn sqlite(conn: rusqlite::Connection, path: &str) -> Result<Self, String> {
        let version = conn
            .query_row("SELECT sqlite_version()", [], |row| row.get::<_, String>(0))
            .map_err(|e| format!("failed to get SQLite version: {e}"))?;
        Ok(Self {
            kind: DbConnectionKind::Sqlite(Arc::new(Mutex::new(conn))),
            version: format!("SQLite {version}"),
            identifier: path.to_string(),
        })
    }

    /// Create a new PostgreSQL connection
    pub fn postgres(client: postgres::Client) -> Result<Self, String> {
        // Version will be set after query
        Ok(Self {
            kind: DbConnectionKind::Postgres(Arc::new(Mutex::new(client))),
            version: String::new(),
            identifier: String::new(),
        })
    }

    /// Create a new MySQL connection
    pub fn mysql(conn: mysql::Conn, url: &str) -> Result<Self, String> {
        Ok(Self {
            kind: DbConnectionKind::MySql(Arc::new(Mutex::new(conn))),
            version: String::new(),
            identifier: url.to_string(),
        })
    }

    /// Create a new DuckDB connection
    pub fn duckdb(conn: duckdb::Connection, path: &str) -> Result<Self, String> {
        let version = conn
            .query_row("SELECT version()", [], |row| row.get::<_, String>(0))
            .map_err(|e| format!("failed to get DuckDB version: {e}"))?;
        Ok(Self {
            kind: DbConnectionKind::DuckDb(Arc::new(Mutex::new(conn))),
            version: format!("DuckDB {version}"),
            identifier: path.to_string(),
        })
    }

    /// Get the database type name
    #[must_use]
    pub fn db_type(&self) -> &'static str {
        match &self.kind {
            DbConnectionKind::Sqlite(_) => "SQLite",
            DbConnectionKind::Postgres(_) => "PostgreSQL",
            DbConnectionKind::MySql(_) => "MySQL",
            DbConnectionKind::DuckDb(_) => "DuckDB",
        }
    }
}

impl fmt::Debug for DbConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbConnection")
            .field("type", &self.db_type())
            .field("version", &self.version)
            .field("identifier", &self.identifier)
            .finish()
    }
}

/// A runtime value in the Stratum VM
#[derive(Clone)]
pub enum Value {
    /// Null value
    Null,

    /// Boolean value
    Bool(bool),

    /// 64-bit signed integer
    Int(i64),

    /// 64-bit floating-point number
    Float(f64),

    /// String (reference-counted)
    String(Rc<String>),

    /// List/array (reference-counted, mutable)
    List(Rc<RefCell<Vec<Value>>>),

    /// Map/dictionary (reference-counted, mutable)
    Map(Rc<RefCell<HashMap<HashableValue, Value>>>),

    /// Function (user-defined)
    Function(Rc<Function>),

    /// Closure (function with captured variables)
    Closure(Rc<Closure>),

    /// Native/built-in function
    NativeFunction(NativeFunction),

    /// Struct instance
    Struct(Rc<RefCell<StructInstance>>),

    /// Enum variant instance
    EnumVariant(Rc<EnumVariantInstance>),

    /// Range (start..end)
    Range(Rc<Range>),

    /// Iterator (for for-loops)
    Iterator(Rc<RefCell<Box<dyn Iterator<Item = Value>>>>),

    /// Bound method (method + receiver)
    BoundMethod(Rc<BoundMethod>),

    /// Native namespace module (File, Dir, Path, Env, Args, Shell)
    NativeNamespace(&'static str),

    /// Compiled regular expression
    Regex(Rc<CompiledRegex>),

    /// Database connection
    DbConnection(Arc<DbConnection>),
}

/// A hashable wrapper for values that can be used as map keys
#[derive(Clone, Debug)]
pub enum HashableValue {
    Null,
    Bool(bool),
    Int(i64),
    String(Rc<String>),
}

impl PartialEq for HashableValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HashableValue::Null, HashableValue::Null) => true,
            (HashableValue::Bool(a), HashableValue::Bool(b)) => a == b,
            (HashableValue::Int(a), HashableValue::Int(b)) => a == b,
            (HashableValue::String(a), HashableValue::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for HashableValue {}

impl Hash for HashableValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            HashableValue::Null => {}
            HashableValue::Bool(b) => b.hash(state),
            HashableValue::Int(i) => i.hash(state),
            HashableValue::String(s) => s.hash(state),
        }
    }
}

impl TryFrom<Value> for HashableValue {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(HashableValue::Null),
            Value::Bool(b) => Ok(HashableValue::Bool(b)),
            Value::Int(i) => Ok(HashableValue::Int(i)),
            Value::String(s) => Ok(HashableValue::String(s)),
            _ => Err("Only null, bool, int, and string can be used as map keys"),
        }
    }
}

impl From<HashableValue> for Value {
    fn from(h: HashableValue) -> Self {
        match h {
            HashableValue::Null => Value::Null,
            HashableValue::Bool(b) => Value::Bool(b),
            HashableValue::Int(i) => Value::Int(i),
            HashableValue::String(s) => Value::String(s),
        }
    }
}

/// A user-defined function
#[derive(Clone)]
pub struct Function {
    /// Function name (for debugging/stack traces)
    pub name: String,

    /// Number of parameters
    pub arity: u8,

    /// Number of upvalues captured
    pub upvalue_count: u16,

    /// Bytecode chunk containing the function's code
    pub chunk: Chunk,
}

impl Function {
    /// Create a new function
    #[must_use]
    pub fn new(name: String, arity: u8) -> Self {
        Self {
            name,
            arity,
            upvalue_count: 0,
            chunk: Chunk::new(),
        }
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("upvalue_count", &self.upvalue_count)
            .finish()
    }
}

/// A closure (function + captured variables)
#[derive(Clone)]
pub struct Closure {
    /// The underlying function
    pub function: Rc<Function>,

    /// Captured upvalues
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl Closure {
    /// Create a new closure
    #[must_use]
    pub fn new(function: Rc<Function>) -> Self {
        Self {
            function,
            upvalues: Vec::new(),
        }
    }
}

impl fmt::Debug for Closure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Closure")
            .field("function", &self.function.name)
            .field("upvalue_count", &self.upvalues.len())
            .finish()
    }
}

/// An upvalue (captured variable from enclosing scope)
#[derive(Clone, Debug)]
pub enum Upvalue {
    /// The variable is still on the stack
    Open(usize),

    /// The variable has been closed (moved to heap)
    Closed(Value),
}

/// A native/built-in function
#[derive(Clone)]
pub struct NativeFunction {
    /// Function name
    pub name: &'static str,

    /// Number of parameters (-1 for variadic)
    pub arity: i8,

    /// The native function pointer
    pub function: fn(&[Value]) -> Result<Value, String>,
}

impl NativeFunction {
    /// Create a new native function
    #[must_use]
    pub const fn new(
        name: &'static str,
        arity: i8,
        function: fn(&[Value]) -> Result<Value, String>,
    ) -> Self {
        Self {
            name,
            arity,
            function,
        }
    }
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .finish()
    }
}

/// A struct instance
#[derive(Clone, Debug)]
pub struct StructInstance {
    /// The struct type name
    pub type_name: String,

    /// Field values by name
    pub fields: HashMap<String, Value>,
}

impl StructInstance {
    /// Create a new struct instance
    #[must_use]
    pub fn new(type_name: String) -> Self {
        Self {
            type_name,
            fields: HashMap::new(),
        }
    }
}

/// An enum variant instance
#[derive(Clone, Debug)]
pub struct EnumVariantInstance {
    /// The enum type name
    pub enum_name: String,

    /// The variant name
    pub variant_name: String,

    /// Associated data (if any)
    pub data: Option<Value>,
}

impl EnumVariantInstance {
    /// Create a new enum variant
    #[must_use]
    pub fn new(enum_name: String, variant_name: String, data: Option<Value>) -> Self {
        Self {
            enum_name,
            variant_name,
            data,
        }
    }
}

/// A range value
#[derive(Clone, Debug)]
pub struct Range {
    /// Start of the range
    pub start: i64,

    /// End of the range
    pub end: i64,

    /// Whether the range is inclusive (..=)
    pub inclusive: bool,
}

impl Range {
    /// Create a new exclusive range
    #[must_use]
    pub const fn exclusive(start: i64, end: i64) -> Self {
        Self {
            start,
            end,
            inclusive: false,
        }
    }

    /// Create a new inclusive range
    #[must_use]
    pub const fn inclusive(start: i64, end: i64) -> Self {
        Self {
            start,
            end,
            inclusive: true,
        }
    }

    /// Returns true if the value is within the range
    #[must_use]
    pub const fn contains(&self, value: i64) -> bool {
        if self.inclusive {
            value >= self.start && value <= self.end
        } else {
            value >= self.start && value < self.end
        }
    }
}

/// A bound method (method + receiver)
#[derive(Clone)]
pub struct BoundMethod {
    /// The receiver object
    pub receiver: Value,

    /// The method closure
    pub method: Rc<Closure>,
}

impl BoundMethod {
    /// Create a new bound method
    #[must_use]
    pub fn new(receiver: Value, method: Rc<Closure>) -> Self {
        Self { receiver, method }
    }
}

impl fmt::Debug for BoundMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundMethod")
            .field("method", &self.method.function.name)
            .finish()
    }
}

impl Value {
    /// Returns true if this value is considered "truthy"
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            // All other values are truthy (including 0 and empty collections)
            _ => true,
        }
    }

    /// Returns true if this value is null
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns the type name of this value (for error messages)
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Function(_) => "Function",
            Value::Closure(_) => "Function",
            Value::NativeFunction(_) => "Function",
            Value::Struct(_) => "Struct",
            Value::EnumVariant(_) => "EnumVariant",
            Value::Range(_) => "Range",
            Value::Iterator(_) => "Iterator",
            Value::BoundMethod(_) => "Method",
            Value::NativeNamespace(name) => name,
            Value::Regex(_) => "Regex",
            Value::DbConnection(_) => "DbConnection",
        }
    }

    /// Create a string value
    #[must_use]
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Rc::new(s.into()))
    }

    /// Create an empty list
    #[must_use]
    pub fn empty_list() -> Self {
        Value::List(Rc::new(RefCell::new(Vec::new())))
    }

    /// Create a list from values
    #[must_use]
    pub fn list(values: Vec<Value>) -> Self {
        Value::List(Rc::new(RefCell::new(values)))
    }

    /// Create an empty map
    #[must_use]
    pub fn empty_map() -> Self {
        Value::Map(Rc::new(RefCell::new(HashMap::new())))
    }

    /// Create a regex value from a compiled regex
    #[must_use]
    pub fn regex(re: CompiledRegex) -> Self {
        Value::Regex(Rc::new(re))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            (Value::Struct(a), Value::Struct(b)) => Rc::ptr_eq(a, b),
            (Value::EnumVariant(a), Value::EnumVariant(b)) => {
                a.enum_name == b.enum_name
                    && a.variant_name == b.variant_name
                    && a.data == b.data
            }
            (Value::Range(a), Value::Range(b)) => {
                a.start == b.start && a.end == b.end && a.inclusive == b.inclusive
            }
            (Value::NativeNamespace(a), Value::NativeNamespace(b)) => a == b,
            (Value::Regex(a), Value::Regex(b)) => a.as_str() == b.as_str(),
            (Value::DbConnection(a), Value::DbConnection(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s:?}"),
            Value::List(l) => write!(f, "{:?}", l.borrow()),
            Value::Map(m) => write!(f, "{:?}", m.borrow()),
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Closure(c) => write!(f, "<fn {}>", c.function.name),
            Value::NativeFunction(n) => write!(f, "<native fn {}>", n.name),
            Value::Struct(s) => {
                let s = s.borrow();
                write!(f, "{} {{ ", s.type_name)?;
                for (i, (k, v)) in s.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v:?}")?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant(e) => {
                if let Some(data) = &e.data {
                    write!(f, "{}.{}({:?})", e.enum_name, e.variant_name, data)
                } else {
                    write!(f, "{}.{}", e.enum_name, e.variant_name)
                }
            }
            Value::Range(r) => {
                if r.inclusive {
                    write!(f, "{}..={}", r.start, r.end)
                } else {
                    write!(f, "{}..{}", r.start, r.end)
                }
            }
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::BoundMethod(m) => write!(f, "<method {}>", m.method.function.name),
            Value::NativeNamespace(name) => write!(f, "<module {name}>"),
            Value::Regex(r) => write!(f, "<regex {}>", r.as_str()),
            Value::DbConnection(c) => write!(f, "<db {} ({})>", c.db_type(), c.version),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    // Display keys properly
                    match k {
                        HashableValue::String(s) => write!(f, "{s:?}: {v}")?,
                        _ => write!(f, "{:?}: {v}", Value::from(k.clone()))?,
                    }
                }
                write!(f, "}}")
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Closure(c) => write!(f, "<fn {}>", c.function.name),
            Value::NativeFunction(n) => write!(f, "<native fn {}>", n.name),
            Value::Struct(s) => {
                let s = s.borrow();
                write!(f, "{} {{ ", s.type_name)?;
                for (i, (k, v)) in s.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant(e) => {
                if let Some(data) = &e.data {
                    write!(f, "{}({data})", e.variant_name)
                } else {
                    write!(f, "{}", e.variant_name)
                }
            }
            Value::Range(r) => {
                if r.inclusive {
                    write!(f, "{}..={}", r.start, r.end)
                } else {
                    write!(f, "{}..{}", r.start, r.end)
                }
            }
            Value::Iterator(_) => write!(f, "<iterator>"),
            Value::BoundMethod(m) => write!(f, "<method {}>", m.method.function.name),
            Value::NativeNamespace(name) => write!(f, "<module {name}>"),
            Value::Regex(r) => write!(f, "<regex {}>", r.as_str()),
            Value::DbConnection(c) => write!(f, "<db {} ({})>", c.db_type(), c.version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_truthiness() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(Value::Int(0).is_truthy()); // 0 is truthy in Stratum
        assert!(Value::Int(42).is_truthy());
        assert!(Value::string("").is_truthy()); // Empty string is truthy
        assert!(Value::string("hello").is_truthy());
    }

    #[test]
    fn value_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::string("hello"), Value::string("hello"));
    }

    #[test]
    fn hashable_value() {
        let key = HashableValue::try_from(Value::string("test")).unwrap();
        assert!(matches!(key, HashableValue::String(_)));

        let list = Value::empty_list();
        assert!(HashableValue::try_from(list).is_err());
    }

    #[test]
    fn value_type_name() {
        assert_eq!(Value::Null.type_name(), "Null");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Float(3.14).type_name(), "Float");
        assert_eq!(Value::string("hi").type_name(), "String");
        assert_eq!(Value::empty_list().type_name(), "List");
    }
}
