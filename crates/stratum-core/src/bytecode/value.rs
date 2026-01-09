//! Runtime values for the Stratum virtual machine

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use image::{DynamicImage, GenericImageView};
use regex::Regex as CompiledRegex;
use tokio::net::{TcpListener as TokioTcpListener, TcpStream as TokioTcpStream, UdpSocket as TokioUdpSocket};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use futures_util::stream::{SplitSink, SplitStream};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use super::Chunk;
use crate::ast::ExecutionMode;
use crate::data::{AggSpec, Cube, CubeBuilder, CubeQuery, DataFrame, GroupedDataFrame, JoinSpec, Rolling, Series, SqlContext};

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

/// TCP stream wrapper for Stratum
/// Wraps a tokio TcpStream with metadata about the connection
#[derive(Debug)]
pub struct TcpStreamWrapper {
    /// The underlying async TCP stream
    pub stream: Arc<tokio::sync::Mutex<TokioTcpStream>>,
    /// Local address
    pub local_addr: String,
    /// Remote (peer) address
    pub peer_addr: String,
}

impl Clone for TcpStreamWrapper {
    fn clone(&self) -> Self {
        Self {
            stream: Arc::clone(&self.stream),
            local_addr: self.local_addr.clone(),
            peer_addr: self.peer_addr.clone(),
        }
    }
}

impl TcpStreamWrapper {
    /// Create a new TCP stream wrapper from a tokio stream
    pub fn new(stream: TokioTcpStream) -> Result<Self, String> {
        let local_addr = stream
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let peer_addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(Self {
            stream: Arc::new(tokio::sync::Mutex::new(stream)),
            local_addr,
            peer_addr,
        })
    }
}

/// TCP listener wrapper for Stratum
/// Wraps a tokio TcpListener for accepting connections
#[derive(Debug)]
pub struct TcpListenerWrapper {
    /// The underlying async TCP listener
    pub listener: Arc<tokio::sync::Mutex<TokioTcpListener>>,
    /// Local address
    pub local_addr: String,
}

impl Clone for TcpListenerWrapper {
    fn clone(&self) -> Self {
        Self {
            listener: Arc::clone(&self.listener),
            local_addr: self.local_addr.clone(),
        }
    }
}

impl TcpListenerWrapper {
    /// Create a new TCP listener wrapper from a tokio listener
    pub fn new(listener: TokioTcpListener) -> Result<Self, String> {
        let local_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(Self {
            listener: Arc::new(tokio::sync::Mutex::new(listener)),
            local_addr,
        })
    }
}

/// UDP socket wrapper for Stratum
/// Wraps a tokio UdpSocket
#[derive(Debug)]
pub struct UdpSocketWrapper {
    /// The underlying async UDP socket
    pub socket: Arc<tokio::sync::Mutex<TokioUdpSocket>>,
    /// Local address
    pub local_addr: String,
}

impl Clone for UdpSocketWrapper {
    fn clone(&self) -> Self {
        Self {
            socket: Arc::clone(&self.socket),
            local_addr: self.local_addr.clone(),
        }
    }
}

impl UdpSocketWrapper {
    /// Create a new UDP socket wrapper from a tokio socket
    pub fn new(socket: TokioUdpSocket) -> Result<Self, String> {
        let local_addr = socket
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(Self {
            socket: Arc::new(tokio::sync::Mutex::new(socket)),
            local_addr,
        })
    }
}

/// WebSocket connection wrapper for Stratum
/// Wraps a tokio-tungstenite WebSocket stream with metadata
#[derive(Debug)]
pub struct WebSocketWrapper {
    /// The write half of the WebSocket (for sending messages)
    pub sink: Arc<tokio::sync::Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TokioTcpStream>>, WsMessage>>>,
    /// The read half of the WebSocket (for receiving messages)
    pub stream: Arc<tokio::sync::Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TokioTcpStream>>>>>,
    /// Remote URL
    pub url: String,
    /// Connection state
    pub closed: Arc<std::sync::atomic::AtomicBool>,
}

impl Clone for WebSocketWrapper {
    fn clone(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
            stream: Arc::clone(&self.stream),
            url: self.url.clone(),
            closed: Arc::clone(&self.closed),
        }
    }
}

impl WebSocketWrapper {
    /// Create a new WebSocket wrapper from split streams
    #[allow(clippy::type_complexity)]
    pub fn new(
        sink: SplitSink<WebSocketStream<MaybeTlsStream<TokioTcpStream>>, WsMessage>,
        stream: SplitStream<WebSocketStream<MaybeTlsStream<TokioTcpStream>>>,
        url: String,
    ) -> Self {
        Self {
            sink: Arc::new(tokio::sync::Mutex::new(sink)),
            stream: Arc::new(tokio::sync::Mutex::new(stream)),
            url,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Check if the WebSocket is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Mark the WebSocket as closed
    pub fn set_closed(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// WebSocket server (listener) wrapper for Stratum
/// Wraps a TCP listener that accepts WebSocket upgrade requests
#[derive(Debug)]
pub struct WebSocketServerWrapper {
    /// The underlying TCP listener
    pub listener: Arc<tokio::sync::Mutex<TokioTcpListener>>,
    /// Local address
    pub local_addr: String,
}

impl Clone for WebSocketServerWrapper {
    fn clone(&self) -> Self {
        Self {
            listener: Arc::clone(&self.listener),
            local_addr: self.local_addr.clone(),
        }
    }
}

impl WebSocketServerWrapper {
    /// Create a new WebSocket server wrapper from a TCP listener
    pub fn new(listener: TokioTcpListener) -> Result<Self, String> {
        let local_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(Self {
            listener: Arc::new(tokio::sync::Mutex::new(listener)),
            local_addr,
        })
    }
}

/// WebSocket server connection wrapper for Stratum
/// Wraps an accepted WebSocket connection from a server
#[derive(Debug)]
pub struct WebSocketServerConnWrapper {
    /// The write half of the WebSocket (for sending messages)
    pub sink: Arc<tokio::sync::Mutex<SplitSink<WebSocketStream<TokioTcpStream>, WsMessage>>>,
    /// The read half of the WebSocket (for receiving messages)
    pub stream: Arc<tokio::sync::Mutex<SplitStream<WebSocketStream<TokioTcpStream>>>>,
    /// Remote peer address
    pub peer_addr: String,
    /// Local address
    pub local_addr: String,
    /// Connection state
    pub closed: Arc<std::sync::atomic::AtomicBool>,
}

impl Clone for WebSocketServerConnWrapper {
    fn clone(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
            stream: Arc::clone(&self.stream),
            peer_addr: self.peer_addr.clone(),
            local_addr: self.local_addr.clone(),
            closed: Arc::clone(&self.closed),
        }
    }
}

impl WebSocketServerConnWrapper {
    /// Create a new WebSocket server connection wrapper from split streams
    #[allow(clippy::type_complexity)]
    pub fn new(
        sink: SplitSink<WebSocketStream<TokioTcpStream>, WsMessage>,
        stream: SplitStream<WebSocketStream<TokioTcpStream>>,
        peer_addr: String,
        local_addr: String,
    ) -> Self {
        Self {
            sink: Arc::new(tokio::sync::Mutex::new(sink)),
            stream: Arc::new(tokio::sync::Mutex::new(stream)),
            peer_addr,
            local_addr,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Check if the WebSocket is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Mark the WebSocket as closed
    pub fn set_closed(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// XML document wrapper for Stratum
/// Wraps a parsed XML document stored as a string (for thread safety)
/// along with metadata about the document
#[derive(Clone)]
pub struct XmlDocumentWrapper {
    /// The XML content as a string (re-parsed on demand for XPath queries)
    pub content: String,
    /// Root element name (cached for quick access)
    pub root_name: String,
}

impl fmt::Debug for XmlDocumentWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XmlDocument")
            .field("root", &self.root_name)
            .field("size", &self.content.len())
            .finish()
    }
}

impl XmlDocumentWrapper {
    /// Create a new XML document wrapper from parsed content
    #[must_use]
    pub fn new(content: String, root_name: String) -> Self {
        Self { content, root_name }
    }
}

/// Image wrapper for Stratum
/// Wraps a dynamic image with metadata
#[derive(Clone)]
pub struct ImageWrapper {
    /// The underlying image data
    pub image: Arc<DynamicImage>,
    /// Original file path (if loaded from file)
    pub source_path: Option<String>,
    /// Image format (for saving)
    pub format: Option<String>,
}

impl fmt::Debug for ImageWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.image.width())
            .field("height", &self.image.height())
            .field("source", &self.source_path)
            .finish()
    }
}

impl ImageWrapper {
    /// Create a new image wrapper
    #[must_use]
    pub fn new(image: DynamicImage, source_path: Option<String>, format: Option<String>) -> Self {
        Self {
            image: Arc::new(image),
            source_path,
            format,
        }
    }

    /// Get the image dimensions
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Get the image width
    #[must_use]
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    /// Get the image height
    #[must_use]
    pub fn height(&self) -> u32 {
        self.image.height()
    }
}

/// A weak reference to a container value
///
/// Weak references do not prevent garbage collection of the referenced value.
/// Use `upgrade()` to attempt to get a strong reference, which returns `None`
/// if the value has been collected.
#[derive(Clone)]
pub enum WeakRefValue {
    /// Weak reference to a list
    List(Weak<RefCell<Vec<Value>>>),
    /// Weak reference to a map
    Map(Weak<RefCell<HashMap<HashableValue, Value>>>),
    /// Weak reference to a set
    Set(Weak<RefCell<HashSet<HashableValue>>>),
    /// Weak reference to a struct instance
    Struct(Weak<RefCell<StructInstance>>),
}

impl WeakRefValue {
    /// Attempt to upgrade the weak reference to a strong reference.
    /// Returns `Some(Value)` if the referenced value is still alive,
    /// or `None` if it has been collected.
    #[must_use]
    pub fn upgrade(&self) -> Option<Value> {
        match self {
            WeakRefValue::List(weak) => weak.upgrade().map(Value::List),
            WeakRefValue::Map(weak) => weak.upgrade().map(Value::Map),
            WeakRefValue::Set(weak) => weak.upgrade().map(Value::Set),
            WeakRefValue::Struct(weak) => weak.upgrade().map(Value::Struct),
        }
    }

    /// Check if the referenced value is still alive.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        match self {
            WeakRefValue::List(weak) => weak.strong_count() > 0,
            WeakRefValue::Map(weak) => weak.strong_count() > 0,
            WeakRefValue::Set(weak) => weak.strong_count() > 0,
            WeakRefValue::Struct(weak) => weak.strong_count() > 0,
        }
    }

    /// Get the type name of the referenced value.
    #[must_use]
    pub fn target_type_name(&self) -> &'static str {
        match self {
            WeakRefValue::List(_) => "List",
            WeakRefValue::Map(_) => "Map",
            WeakRefValue::Set(_) => "Set",
            WeakRefValue::Struct(_) => "Struct",
        }
    }

    /// Get the raw pointer for identity comparison.
    #[must_use]
    pub fn ptr(&self) -> usize {
        match self {
            WeakRefValue::List(weak) => weak.as_ptr() as usize,
            WeakRefValue::Map(weak) => weak.as_ptr() as usize,
            WeakRefValue::Set(weak) => weak.as_ptr() as usize,
            WeakRefValue::Struct(weak) => weak.as_ptr() as usize,
        }
    }
}

impl fmt::Debug for WeakRefValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alive = if self.is_alive() { "alive" } else { "dead" };
        write!(f, "<weak {} ({})>", self.target_type_name(), alive)
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

    /// Set (reference-counted, mutable)
    Set(Rc<RefCell<HashSet<HashableValue>>>),

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

    /// TCP stream (connected socket)
    TcpStream(Arc<TcpStreamWrapper>),

    /// TCP listener (server socket)
    TcpListener(Arc<TcpListenerWrapper>),

    /// UDP socket
    UdpSocket(Arc<UdpSocketWrapper>),

    /// WebSocket client connection
    WebSocket(Arc<WebSocketWrapper>),

    /// WebSocket server (listener)
    WebSocketServer(Arc<WebSocketServerWrapper>),

    /// WebSocket server connection (accepted from server)
    WebSocketServerConn(Arc<WebSocketServerConnWrapper>),

    /// Future (async computation result)
    Future(Rc<RefCell<FutureState>>),

    /// Coroutine (suspended async function state)
    Coroutine(Rc<RefCell<CoroutineState>>),

    /// DataFrame (Arrow-backed columnar data)
    DataFrame(Arc<DataFrame>),

    /// Series (single column of data)
    Series(Arc<Series>),

    /// Rolling window for Series (for computing windowed aggregations)
    Rolling(Arc<Rolling>),

    /// GroupedDataFrame (DataFrame partitioned by key columns)
    GroupedDataFrame(Arc<GroupedDataFrame>),

    /// Aggregation specification (for builder pattern aggregations)
    AggSpec(Arc<AggSpec>),

    /// Join specification (for builder pattern joins)
    JoinSpec(Arc<JoinSpec>),

    /// SQL context for multi-table queries
    SqlContext(Arc<Mutex<SqlContext>>),

    /// OLAP Cube for multi-dimensional analytical processing
    Cube(Arc<Cube>),

    /// OLAP Cube builder for fluent construction
    CubeBuilder(Arc<Mutex<Option<CubeBuilder>>>),

    /// OLAP Cube query for lazy OLAP operations (slice, dice, drill_down, roll_up)
    CubeQuery(Arc<Mutex<Option<CubeQuery>>>),

    /// GUI element (opaque container for stratum-gui types)
    /// Stored as a type-erased Arc to allow cross-crate use
    GuiElement(Arc<dyn GuiValue>),

    /// State binding for reactive GUI updates (&state.field)
    /// Contains the dotted path to the bound field
    StateBinding(String),

    /// Test expectation (from Test.expect(value))
    Expectation(Rc<RefCell<ExpectationState>>),

    /// XML document (parsed XML with XPath support)
    XmlDocument(Arc<XmlDocumentWrapper>),

    /// Image (loaded image for processing)
    Image(Arc<ImageWrapper>),

    /// Weak reference to a container value
    /// Does not prevent garbage collection of the referenced value
    WeakRef(WeakRefValue),
}

/// Trait for GUI values that can be stored in the VM.
/// Implemented by stratum-gui's GuiElement type.
pub trait GuiValue: std::fmt::Debug + Send + Sync {
    /// Get the element kind name (for debugging/display)
    fn kind_name(&self) -> &'static str;

    /// Clone the GUI value into a new Arc
    fn clone_boxed(&self) -> Arc<dyn GuiValue>;

    /// Get self as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get self as mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
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

    /// Execution mode for this function (interpret, compile, or JIT)
    pub execution_mode: ExecutionMode,
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
            execution_mode: ExecutionMode::default(),
        }
    }

    /// Create a new function with a specific execution mode
    #[must_use]
    pub fn with_execution_mode(name: String, arity: u8, execution_mode: ExecutionMode) -> Self {
        Self {
            name,
            arity,
            upvalue_count: 0,
            chunk: Chunk::new(),
            execution_mode,
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

/// State for test expectations (Test.expect(value))
#[derive(Clone, Debug)]
pub struct ExpectationState {
    /// The value being tested
    pub actual: Box<Value>,
    /// Whether the expectation has been negated (.not)
    pub negated: bool,
}

impl ExpectationState {
    /// Create a new expectation with a value
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            actual: Box::new(value),
            negated: false,
        }
    }

    /// Create a negated expectation
    #[must_use]
    pub fn negated(value: Value) -> Self {
        Self {
            actual: Box::new(value),
            negated: true,
        }
    }
}

/// Status of a future/promise
#[derive(Clone, Debug, PartialEq)]
pub enum FutureStatus {
    /// Future is still pending (not yet resolved)
    Pending,
    /// Future has completed successfully
    Ready,
    /// Future failed with an error
    Failed(String),
}

/// A future representing an asynchronous computation
#[derive(Clone, Debug)]
pub struct FutureState {
    /// Current status of the future
    pub status: FutureStatus,
    /// Result value (if Ready)
    pub result: Option<Value>,
    /// Kind of async operation (e.g., "sleep", "http_get") - used by executor
    pub kind: Option<String>,
    /// Metadata for the async operation (e.g., duration for sleep, URL for HTTP)
    pub metadata: Option<Value>,
}

impl FutureState {
    /// Create a new pending future
    #[must_use]
    pub fn pending() -> Self {
        Self {
            status: FutureStatus::Pending,
            result: None,
            kind: None,
            metadata: None,
        }
    }

    /// Create a pending future with metadata (for sleep, HTTP, etc.)
    #[must_use]
    pub fn pending_with_metadata(metadata: Value, kind: String) -> Self {
        Self {
            status: FutureStatus::Pending,
            result: None,
            kind: Some(kind),
            metadata: Some(metadata),
        }
    }

    /// Create a resolved future with a value
    #[must_use]
    pub fn ready(value: Value) -> Self {
        Self {
            status: FutureStatus::Ready,
            result: Some(value),
            kind: None,
            metadata: None,
        }
    }

    /// Create a failed future with an error message
    #[must_use]
    pub fn failed(error: String) -> Self {
        Self {
            status: FutureStatus::Failed(error),
            result: None,
            kind: None,
            metadata: None,
        }
    }

    /// Check if the future is pending
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self.status, FutureStatus::Pending)
    }

    /// Check if the future is ready
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.status, FutureStatus::Ready)
    }

    /// Get the kind of async operation
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// Get the metadata value
    #[must_use]
    pub fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }
}

/// Status of a coroutine
#[derive(Clone, Debug, PartialEq)]
pub enum CoroutineStatus {
    /// Coroutine is suspended, waiting for a future
    Suspended,
    /// Coroutine is currently running
    Running,
    /// Coroutine completed successfully with a value
    Completed(Box<Value>),
    /// Coroutine failed with an error
    Failed(String),
}

/// Saved call frame for coroutine suspension
#[derive(Clone, Debug)]
pub struct SavedCallFrame {
    /// The closure being executed
    pub closure: Rc<Closure>,
    /// Instruction pointer
    pub ip: usize,
    /// Stack base offset (relative to coroutine stack)
    pub stack_base: usize,
}

/// Saved exception handler for coroutine suspension
#[derive(Clone, Debug)]
pub struct SavedExceptionHandler {
    /// Frame index (relative to coroutine's frames)
    pub frame_index: usize,
    /// Stack depth (relative to frame's stack_base)
    pub stack_depth: usize,
    /// IP of catch block
    pub catch_ip: usize,
    /// IP of finally block (0 if none)
    pub finally_ip: usize,
}

/// A suspended coroutine state
#[derive(Clone, Debug)]
pub struct CoroutineState {
    /// Saved call frames
    pub frames: Vec<SavedCallFrame>,
    /// Saved value stack
    pub stack: Vec<Value>,
    /// Saved exception handlers
    pub handlers: Vec<SavedExceptionHandler>,
    /// The future being awaited (if suspended)
    pub awaited_future: Option<Value>,
    /// Current status
    pub status: CoroutineStatus,
}

impl CoroutineState {
    /// Create a new suspended coroutine state
    #[must_use]
    pub fn suspended(
        frames: Vec<SavedCallFrame>,
        stack: Vec<Value>,
        handlers: Vec<SavedExceptionHandler>,
        awaited_future: Value,
    ) -> Self {
        Self {
            frames,
            stack,
            handlers,
            awaited_future: Some(awaited_future),
            status: CoroutineStatus::Suspended,
        }
    }

    /// Check if the coroutine is suspended
    #[must_use]
    pub fn is_suspended(&self) -> bool {
        matches!(self.status, CoroutineStatus::Suspended)
    }

    /// Check if the coroutine is completed
    #[must_use]
    pub fn is_completed(&self) -> bool {
        matches!(self.status, CoroutineStatus::Completed(_))
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
            Value::Set(_) => "Set",
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
            Value::TcpStream(_) => "TcpStream",
            Value::TcpListener(_) => "TcpListener",
            Value::UdpSocket(_) => "UdpSocket",
            Value::WebSocket(_) => "WebSocket",
            Value::WebSocketServer(_) => "WebSocketServer",
            Value::WebSocketServerConn(_) => "WebSocketServerConn",
            Value::Future(_) => "Future",
            Value::Coroutine(_) => "Coroutine",
            Value::DataFrame(_) => "DataFrame",
            Value::Series(_) => "Series",
            Value::Rolling(_) => "Rolling",
            Value::GroupedDataFrame(_) => "GroupedDataFrame",
            Value::AggSpec(_) => "AggSpec",
            Value::JoinSpec(_) => "JoinSpec",
            Value::SqlContext(_) => "SqlContext",
            Value::Cube(_) => "Cube",
            Value::CubeBuilder(_) => "CubeBuilder",
            Value::CubeQuery(_) => "CubeQuery",
            Value::GuiElement(e) => e.kind_name(),
            Value::StateBinding(_) => "StateBinding",
            Value::Expectation(_) => "Expectation",
            Value::XmlDocument(_) => "XmlDocument",
            Value::Image(_) => "Image",
            Value::WeakRef(_) => "WeakRef",
        }
    }

    /// Create a pending future
    #[must_use]
    pub fn pending_future() -> Self {
        Value::Future(Rc::new(RefCell::new(FutureState::pending())))
    }

    /// Create a resolved future with a value
    #[must_use]
    pub fn ready_future(value: Value) -> Self {
        Value::Future(Rc::new(RefCell::new(FutureState::ready(value))))
    }

    /// Create a failed future with an error
    #[must_use]
    pub fn failed_future(error: String) -> Self {
        Value::Future(Rc::new(RefCell::new(FutureState::failed(error))))
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

    /// Create an empty set
    #[must_use]
    pub fn empty_set() -> Self {
        Value::Set(Rc::new(RefCell::new(HashSet::new())))
    }

    /// Create a set from hashable values
    #[must_use]
    pub fn set(values: HashSet<HashableValue>) -> Self {
        Value::Set(Rc::new(RefCell::new(values)))
    }

    /// Create a regex value from a compiled regex
    #[must_use]
    pub fn regex(re: CompiledRegex) -> Self {
        Value::Regex(Rc::new(re))
    }

    /// Create an expectation value (for Test.expect())
    #[must_use]
    pub fn expectation(value: Value) -> Self {
        Value::Expectation(Rc::new(RefCell::new(ExpectationState::new(value))))
    }

    /// Create a negated expectation value (for Test.expect().not)
    #[must_use]
    pub fn negated_expectation(value: Value) -> Self {
        Value::Expectation(Rc::new(RefCell::new(ExpectationState::negated(value))))
    }

    /// Create a weak reference from a container value.
    /// Returns `Some(Value::WeakRef(...))` for supported container types,
    /// or `None` for non-container types.
    #[must_use]
    pub fn weak_ref(&self) -> Option<Value> {
        match self {
            Value::List(rc) => Some(Value::WeakRef(WeakRefValue::List(Rc::downgrade(rc)))),
            Value::Map(rc) => Some(Value::WeakRef(WeakRefValue::Map(Rc::downgrade(rc)))),
            Value::Set(rc) => Some(Value::WeakRef(WeakRefValue::Set(Rc::downgrade(rc)))),
            Value::Struct(rc) => Some(Value::WeakRef(WeakRefValue::Struct(Rc::downgrade(rc)))),
            _ => None,
        }
    }

    /// Attempt to upgrade a weak reference to a strong reference.
    /// Returns `Some(Value)` if this is a weak ref and the target is still alive,
    /// `None` if this is a weak ref but the target was collected,
    /// or the original value if it's not a weak ref.
    #[must_use]
    pub fn upgrade_weak(&self) -> Option<Value> {
        match self {
            Value::WeakRef(weak) => weak.upgrade(),
            _ => Some(self.clone()),
        }
    }

    /// Check if this value is a weak reference.
    #[must_use]
    pub fn is_weak_ref(&self) -> bool {
        matches!(self, Value::WeakRef(_))
    }

    /// If this is a weak reference, check if the referenced value is still alive.
    /// Returns `true` for non-weak-ref values.
    #[must_use]
    pub fn is_weak_ref_alive(&self) -> bool {
        match self {
            Value::WeakRef(weak) => weak.is_alive(),
            _ => true,
        }
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
            (Value::Set(a), Value::Set(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
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
            (Value::TcpStream(a), Value::TcpStream(b)) => Arc::ptr_eq(a, b),
            (Value::TcpListener(a), Value::TcpListener(b)) => Arc::ptr_eq(a, b),
            (Value::UdpSocket(a), Value::UdpSocket(b)) => Arc::ptr_eq(a, b),
            (Value::WebSocket(a), Value::WebSocket(b)) => Arc::ptr_eq(a, b),
            (Value::WebSocketServer(a), Value::WebSocketServer(b)) => Arc::ptr_eq(a, b),
            (Value::WebSocketServerConn(a), Value::WebSocketServerConn(b)) => Arc::ptr_eq(a, b),
            (Value::Future(a), Value::Future(b)) => Rc::ptr_eq(a, b),
            (Value::Coroutine(a), Value::Coroutine(b)) => Rc::ptr_eq(a, b),
            (Value::DataFrame(a), Value::DataFrame(b)) => Arc::ptr_eq(a, b),
            (Value::Series(a), Value::Series(b)) => Arc::ptr_eq(a, b),
            (Value::Rolling(a), Value::Rolling(b)) => Arc::ptr_eq(a, b),
            (Value::JoinSpec(a), Value::JoinSpec(b)) => Arc::ptr_eq(a, b),
            (Value::Cube(a), Value::Cube(b)) => Arc::ptr_eq(a, b),
            (Value::CubeBuilder(a), Value::CubeBuilder(b)) => Arc::ptr_eq(a, b),
            (Value::CubeQuery(a), Value::CubeQuery(b)) => Arc::ptr_eq(a, b),
            (Value::GuiElement(a), Value::GuiElement(b)) => Arc::ptr_eq(a, b),
            (Value::StateBinding(a), Value::StateBinding(b)) => a == b,
            (Value::Expectation(a), Value::Expectation(b)) => Rc::ptr_eq(a, b),
            (Value::XmlDocument(a), Value::XmlDocument(b)) => Arc::ptr_eq(a, b),
            (Value::Image(a), Value::Image(b)) => Arc::ptr_eq(a, b),
            (Value::WeakRef(a), Value::WeakRef(b)) => a.ptr() == b.ptr(),
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
            Value::Set(s) => write!(f, "{:?}", s.borrow()),
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
            Value::TcpStream(s) => write!(f, "<tcp stream {} -> {}>", s.local_addr, s.peer_addr),
            Value::TcpListener(l) => write!(f, "<tcp listener {}>", l.local_addr),
            Value::UdpSocket(s) => write!(f, "<udp socket {}>", s.local_addr),
            Value::WebSocket(ws) => write!(f, "<websocket {}>", ws.url),
            Value::WebSocketServer(wss) => write!(f, "<websocket server {}>", wss.local_addr),
            Value::WebSocketServerConn(wsc) => write!(f, "<websocket conn {} -> {}>", wsc.local_addr, wsc.peer_addr),
            Value::Future(fut) => {
                let fut = fut.borrow();
                match &fut.status {
                    FutureStatus::Pending => write!(f, "<future pending>"),
                    FutureStatus::Ready => write!(f, "<future ready: {:?}>", fut.result),
                    FutureStatus::Failed(e) => write!(f, "<future failed: {e}>"),
                }
            }
            Value::Coroutine(coro) => {
                let coro = coro.borrow();
                match &coro.status {
                    CoroutineStatus::Suspended => write!(f, "<coroutine suspended>"),
                    CoroutineStatus::Running => write!(f, "<coroutine running>"),
                    CoroutineStatus::Completed(v) => write!(f, "<coroutine completed: {v:?}>"),
                    CoroutineStatus::Failed(e) => write!(f, "<coroutine failed: {e}>"),
                }
            }
            Value::DataFrame(df) => {
                write!(f, "<DataFrame [{} cols x {} rows]>", df.num_columns(), df.num_rows())
            }
            Value::Series(s) => {
                write!(f, "<Series '{}' [{} rows]>", s.name(), s.len())
            }
            Value::Rolling(r) => {
                write!(f, "<Rolling window={} on '{}'>", r.window_size(), r.series().name())
            }
            Value::GroupedDataFrame(gdf) => {
                write!(
                    f,
                    "<GroupedDataFrame by {:?} ({} groups)>",
                    gdf.group_columns(),
                    gdf.num_groups()
                )
            }
            Value::AggSpec(spec) => {
                write!(
                    f,
                    "<AggSpec {}({:?}) -> {}>",
                    spec.op.name(),
                    spec.column,
                    spec.output_name
                )
            }
            Value::JoinSpec(spec) => {
                write!(
                    f,
                    "<JoinSpec {} on {}.{} = {}.{}>",
                    spec.join_type.name(),
                    "left",
                    spec.left_column,
                    "right",
                    spec.right_column
                )
            }
            Value::SqlContext(ctx) => {
                let tables = ctx.lock().map(|c| c.tables()).unwrap_or_default();
                write!(f, "<SqlContext ({} tables)>", tables.len())
            }
            Value::Cube(cube) => {
                let name = cube.name().unwrap_or("unnamed");
                let dims = cube.dimension_names().len();
                let measures = cube.measure_names().len();
                let rows = cube.row_count();
                write!(f, "<Cube '{}' [{} dims x {} measures x {} rows]>", name, dims, measures, rows)
            }
            Value::CubeBuilder(builder) => {
                let status = if builder.lock().map(|b| b.is_some()).unwrap_or(false) {
                    "active"
                } else {
                    "consumed"
                };
                write!(f, "<CubeBuilder ({})>", status)
            }
            Value::CubeQuery(query) => {
                if let Ok(guard) = query.lock() {
                    if let Some(q) = guard.as_ref() {
                        let name = q.cube_name().unwrap_or("unnamed");
                        let ops = q.slices().len() + q.dices().len();
                        write!(f, "<CubeQuery on '{}' [{} ops]>", name, ops)
                    } else {
                        write!(f, "<CubeQuery (consumed)>")
                    }
                } else {
                    write!(f, "<CubeQuery (locked)>")
                }
            }
            Value::GuiElement(e) => write!(f, "<GuiElement {}>", e.kind_name()),
            Value::StateBinding(path) => write!(f, "<StateBinding &{path}>"),
            Value::Expectation(exp) => {
                let exp = exp.borrow();
                if exp.negated {
                    write!(f, "<Expectation not {:?}>", exp.actual)
                } else {
                    write!(f, "<Expectation {:?}>", exp.actual)
                }
            }
            Value::XmlDocument(doc) => {
                write!(f, "<XmlDocument root='{}' size={}>", doc.root_name, doc.content.len())
            }
            Value::Image(img) => {
                write!(f, "<Image {}x{}>", img.width(), img.height())
            }
            Value::WeakRef(weak) => {
                let alive = if weak.is_alive() { "alive" } else { "dead" };
                write!(f, "<weak {} ({})>", weak.target_type_name(), alive)
            }
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
            Value::Set(s) => {
                write!(f, "Set{{")?;
                for (i, v) in s.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", Value::from(v.clone()))?;
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
            Value::TcpStream(s) => write!(f, "<tcp {} -> {}>", s.local_addr, s.peer_addr),
            Value::TcpListener(l) => write!(f, "<tcp listener {}>", l.local_addr),
            Value::UdpSocket(s) => write!(f, "<udp {}>", s.local_addr),
            Value::WebSocket(ws) => write!(f, "<websocket {}>", ws.url),
            Value::WebSocketServer(wss) => write!(f, "<websocket server {}>", wss.local_addr),
            Value::WebSocketServerConn(wsc) => write!(f, "<websocket {} -> {}>", wsc.local_addr, wsc.peer_addr),
            Value::Future(fut) => {
                let fut = fut.borrow();
                match &fut.status {
                    FutureStatus::Pending => write!(f, "<future pending>"),
                    FutureStatus::Ready => {
                        if let Some(result) = &fut.result {
                            write!(f, "{result}")
                        } else {
                            write!(f, "<future ready>")
                        }
                    }
                    FutureStatus::Failed(e) => write!(f, "<future failed: {e}>"),
                }
            }
            Value::Coroutine(coro) => {
                let coro = coro.borrow();
                match &coro.status {
                    CoroutineStatus::Suspended => write!(f, "<coroutine suspended>"),
                    CoroutineStatus::Running => write!(f, "<coroutine running>"),
                    CoroutineStatus::Completed(v) => write!(f, "{v}"),
                    CoroutineStatus::Failed(e) => write!(f, "<coroutine failed: {e}>"),
                }
            }
            Value::DataFrame(df) => write!(f, "{df}"),
            Value::Series(s) => write!(f, "{s}"),
            Value::Rolling(r) => write!(f, "{r}"),
            Value::GroupedDataFrame(gdf) => write!(
                f,
                "<grouped by {:?} ({} groups)>",
                gdf.group_columns(),
                gdf.num_groups()
            ),
            Value::AggSpec(spec) => write!(
                f,
                "<agg {}({:?}) -> {}>",
                spec.op.name(),
                spec.column,
                spec.output_name
            ),
            Value::JoinSpec(spec) => write!(
                f,
                "<join {} on {} = {}>",
                spec.join_type.name(),
                spec.left_column,
                spec.right_column
            ),
            Value::SqlContext(ctx) => {
                let tables = ctx.lock().map(|c| c.tables()).unwrap_or_default();
                write!(f, "<sql context ({} tables)>", tables.len())
            }
            Value::Cube(cube) => write!(f, "{cube}"),
            Value::CubeBuilder(_) => write!(f, "<cube builder>"),
            Value::CubeQuery(query) => {
                if let Ok(guard) = query.lock() {
                    if let Some(q) = guard.as_ref() {
                        write!(f, "{}", q)
                    } else {
                        write!(f, "<cube query (consumed)>")
                    }
                } else {
                    write!(f, "<cube query>")
                }
            }
            Value::GuiElement(e) => write!(f, "<gui {}>", e.kind_name()),
            Value::StateBinding(path) => write!(f, "<binding &{path}>"),
            Value::Expectation(exp) => {
                let exp = exp.borrow();
                if exp.negated {
                    write!(f, "<expect not {}>", exp.actual)
                } else {
                    write!(f, "<expect {}>", exp.actual)
                }
            }
            Value::XmlDocument(doc) => {
                write!(f, "<xml root='{}'>", doc.root_name)
            }
            Value::Image(img) => {
                write!(f, "<image {}x{}>", img.width(), img.height())
            }
            Value::WeakRef(weak) => {
                let alive = if weak.is_alive() { "alive" } else { "dead" };
                write!(f, "<weak {} ({})>", weak.target_type_name(), alive)
            }
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

    #[test]
    fn future_value() {
        let pending = Value::pending_future();
        assert_eq!(pending.type_name(), "Future");
        assert_eq!(format!("{pending}"), "<future pending>");

        let ready = Value::ready_future(Value::Int(42));
        assert_eq!(format!("{ready}"), "42");

        let failed = Value::failed_future("error".to_string());
        assert_eq!(format!("{failed}"), "<future failed: error>");
    }

    #[test]
    fn future_state() {
        let pending = FutureState::pending();
        assert!(pending.is_pending());
        assert!(!pending.is_ready());

        let ready = FutureState::ready(Value::Int(42));
        assert!(!ready.is_pending());
        assert!(ready.is_ready());
        assert_eq!(ready.result, Some(Value::Int(42)));
    }
}
