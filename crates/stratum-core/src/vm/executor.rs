//! Async executor for the Stratum virtual machine
//!
//! This module provides a single-threaded async executor that:
//! - Manages coroutine scheduling
//! - Waits for native futures (sleep, HTTP, etc.)
//! - Resumes suspended coroutines with results

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::runtime::Builder;
use tokio::task::LocalSet;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::connect_async;

use super::{RuntimeError, RuntimeErrorKind, RuntimeResult, VM};
use crate::bytecode::{
    CoroutineState, CoroutineStatus, FutureState, FutureStatus, HashableValue,
    TcpListenerWrapper, TcpStreamWrapper, UdpSocketWrapper,
    WebSocketWrapper, WebSocketServerWrapper, WebSocketServerConnWrapper, Value,
};
use std::sync::Arc;

/// Result of running a coroutine step
pub enum CoroutineResult {
    /// Coroutine completed with a final value
    Completed(Value),
    /// Coroutine suspended, waiting for a future
    Suspended {
        /// The coroutine state to resume later
        state: CoroutineState,
        /// The future being awaited
        pending_future: Value,
    },
}

/// A task in the ready queue
struct ReadyTask {
    /// The coroutine state to resume
    coroutine: CoroutineState,
    /// The value to resume with (result of the awaited future)
    resume_value: Value,
}

/// Async executor for running coroutines
pub struct AsyncExecutor {
    /// The tokio runtime (single-threaded)
    runtime: tokio::runtime::Runtime,
    /// Queue of coroutines ready to resume
    ready_queue: RefCell<VecDeque<ReadyTask>>,
}

impl Default for AsyncExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncExecutor {
    /// Create a new async executor with a single-threaded tokio runtime
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime: Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            ready_queue: RefCell::new(VecDeque::new()),
        }
    }

    /// Run a coroutine until completion, handling all async operations
    ///
    /// This is the main entry point for executing async code. It will:
    /// 1. Resume the coroutine with the given value
    /// 2. If it suspends, wait for the pending future
    /// 3. Resume with the future's result
    /// 4. Repeat until completion or error
    pub fn run_to_completion(
        &self,
        vm: &mut VM,
        initial_coroutine: CoroutineState,
    ) -> RuntimeResult<Value> {
        // Add the initial coroutine to the ready queue
        self.ready_queue.borrow_mut().push_back(ReadyTask {
            coroutine: initial_coroutine,
            resume_value: Value::Null,
        });

        // Create a LocalSet for running !Send futures
        let local_set = LocalSet::new();

        // Run the executor loop
        self.runtime.block_on(local_set.run_until(async {
            self.executor_loop(vm).await
        }))
    }

    /// The main executor loop
    async fn executor_loop(&self, vm: &mut VM) -> RuntimeResult<Value> {
        loop {
            // Get next ready task
            let task = self.ready_queue.borrow_mut().pop_front();

            match task {
                Some(task) => {
                    // Resume the coroutine
                    match self.step_coroutine(vm, task.coroutine, task.resume_value)? {
                        CoroutineResult::Completed(value) => {
                            // Check if there are more tasks
                            if self.ready_queue.borrow().is_empty() {
                                return Ok(value);
                            }
                            // Otherwise continue with next task
                            // (the completed value is discarded for now)
                        }
                        CoroutineResult::Suspended {
                            state,
                            pending_future,
                        } => {
                            // Wait for the future and re-queue
                            let result = self.wait_for_future(&pending_future).await;
                            self.ready_queue.borrow_mut().push_back(ReadyTask {
                                coroutine: state,
                                resume_value: result,
                            });
                        }
                    }
                }
                None => {
                    // No tasks in queue - should not happen if we always re-queue
                    return Err(RuntimeError::new(RuntimeErrorKind::Internal(
                        "Async executor: no tasks to run".to_string(),
                    )));
                }
            }
        }
    }

    /// Execute one step of a coroutine
    fn step_coroutine(
        &self,
        vm: &mut VM,
        coroutine: CoroutineState,
        resume_value: Value,
    ) -> RuntimeResult<CoroutineResult> {
        // Resume the VM with the coroutine state
        vm.resume_coroutine(&coroutine, resume_value)?;

        // Continue execution
        let result = vm.continue_execution()?;

        // Check what we got back
        match &result {
            Value::Coroutine(coro_ref) => {
                // We got a suspended coroutine back
                let coro = coro_ref.borrow();
                match &coro.status {
                    CoroutineStatus::Suspended => {
                        // Get the awaited future
                        let pending = coro.awaited_future.clone().unwrap_or(Value::Null);
                        Ok(CoroutineResult::Suspended {
                            state: coro.clone(),
                            pending_future: pending,
                        })
                    }
                    CoroutineStatus::Completed(value) => {
                        Ok(CoroutineResult::Completed((**value).clone()))
                    }
                    CoroutineStatus::Running => Err(RuntimeError::new(RuntimeErrorKind::Internal(
                        "Coroutine returned with Running status".to_string(),
                    ))),
                    CoroutineStatus::Failed(err) => {
                        Err(RuntimeError::new(RuntimeErrorKind::AsyncError(err.clone())))
                    }
                }
            }
            // Not a coroutine - execution completed
            _ => Ok(CoroutineResult::Completed(result)),
        }
    }

    /// Wait for a native future to complete and return its result
    async fn wait_for_future(&self, future: &Value) -> Value {
        match future {
            Value::Future(fut_ref) => {
                // Check if already resolved
                let (kind, metadata) = {
                    let fut = fut_ref.borrow();
                    match &fut.status {
                        FutureStatus::Ready => {
                            return fut.result.clone().unwrap_or(Value::Null);
                        }
                        FutureStatus::Failed(err) => {
                            return Value::string(format!("Error: {err}"));
                        }
                        FutureStatus::Pending => {
                            (fut.kind.clone(), fut.metadata.clone())
                        }
                    }
                };

                // Handle known future kinds
                if let Some(kind_str) = kind.as_deref() {
                    let result = match kind_str {
                        "sleep" => {
                            // Get sleep duration from metadata
                            if let Some(Value::Int(ms)) = &metadata {
                                let duration = std::time::Duration::from_millis(*ms as u64);
                                tokio::time::sleep(duration).await;
                                Ok(Value::Null)
                            } else {
                                Err("sleep: invalid duration metadata".to_string())
                            }
                        }
                        "tcp_connect" => {
                            // Connect to TCP server
                            if let Some(Value::String(addr)) = &metadata {
                                match TcpStream::connect(addr.as_str()).await {
                                    Ok(stream) => {
                                        match TcpStreamWrapper::new(stream) {
                                            Ok(wrapper) => Ok(Value::TcpStream(Arc::new(wrapper))),
                                            Err(e) => Err(format!("tcp_connect: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("tcp_connect to {}: {e}", addr)),
                                }
                            } else {
                                Err("tcp_connect: invalid address metadata".to_string())
                            }
                        }
                        "tcp_listen" => {
                            // Bind TCP listener
                            if let Some(Value::String(addr)) = &metadata {
                                match TcpListener::bind(addr.as_str()).await {
                                    Ok(listener) => {
                                        match TcpListenerWrapper::new(listener) {
                                            Ok(wrapper) => Ok(Value::TcpListener(Arc::new(wrapper))),
                                            Err(e) => Err(format!("tcp_listen: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("tcp_listen on {}: {e}", addr)),
                                }
                            } else {
                                Err("tcp_listen: invalid address metadata".to_string())
                            }
                        }
                        "tcp_accept" => {
                            // Accept a new connection
                            if let Some(Value::TcpListener(listener_wrapper)) = &metadata {
                                let listener = listener_wrapper.listener.lock().await;
                                match listener.accept().await {
                                    Ok((stream, _addr)) => {
                                        drop(listener); // Release lock
                                        match TcpStreamWrapper::new(stream) {
                                            Ok(wrapper) => Ok(Value::TcpStream(Arc::new(wrapper))),
                                            Err(e) => Err(format!("tcp_accept: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("tcp_accept: {e}")),
                                }
                            } else {
                                Err("tcp_accept: invalid listener metadata".to_string())
                            }
                        }
                        "tcp_read" => {
                            // Read from TCP stream
                            if let Some(Value::TcpStream(stream_wrapper)) = &metadata {
                                let max_bytes = {
                                    let fut = fut_ref.borrow();
                                    if let Some(Value::Map(m)) = &fut.metadata {
                                        let m = m.borrow();
                                        if let Some(Value::Int(n)) = m.get(&HashableValue::String(Rc::new("max_bytes".into()))) {
                                            *n as usize
                                        } else {
                                            8192
                                        }
                                    } else {
                                        8192
                                    }
                                };
                                let mut stream = stream_wrapper.stream.lock().await;
                                let mut buf = vec![0u8; max_bytes];
                                match stream.read(&mut buf).await {
                                    Ok(n) => {
                                        buf.truncate(n);
                                        match String::from_utf8(buf.clone()) {
                                            Ok(s) => Ok(Value::string(s)),
                                            Err(_) => Ok(Value::list(buf.into_iter().map(|b| Value::Int(b as i64)).collect())),
                                        }
                                    }
                                    Err(e) => Err(format!("tcp_read: {e}")),
                                }
                            } else {
                                Err("tcp_read: invalid stream metadata".to_string())
                            }
                        }
                        "tcp_write" => {
                            // Write to TCP stream - extract data and stream before awaiting
                            let (stream_wrapper, data) = {
                                let fut = fut_ref.borrow();
                                let stream = match &fut.metadata {
                                    Some(Value::TcpStream(s)) => Arc::clone(s),
                                    _ => return self.mark_future_done(fut_ref, Err("tcp_write: invalid stream metadata".to_string())),
                                };
                                // The data to write is stored in the original metadata before we replaced it
                                // For tcp_write, we stored the data as the initial metadata
                                let data = match &fut.result {
                                    Some(Value::String(s)) => s.as_bytes().to_vec(),
                                    Some(Value::List(l)) => {
                                        l.borrow().iter().filter_map(|v| match v {
                                            Value::Int(i) if *i >= 0 && *i <= 255 => Some(*i as u8),
                                            _ => None,
                                        }).collect()
                                    }
                                    _ => return self.mark_future_done(fut_ref, Err("tcp_write: invalid data".to_string())),
                                };
                                (stream, data)
                            };
                            let mut stream = stream_wrapper.stream.lock().await;
                            match stream.write_all(&data).await {
                                Ok(()) => Ok(Value::Int(data.len() as i64)),
                                Err(e) => Err(format!("tcp_write: {e}")),
                            }
                        }
                        "udp_bind" => {
                            // Bind UDP socket
                            if let Some(Value::String(addr)) = &metadata {
                                match UdpSocket::bind(addr.as_str()).await {
                                    Ok(socket) => {
                                        match UdpSocketWrapper::new(socket) {
                                            Ok(wrapper) => Ok(Value::UdpSocket(Arc::new(wrapper))),
                                            Err(e) => Err(format!("udp_bind: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("udp_bind on {}: {e}", addr)),
                                }
                            } else {
                                Err("udp_bind: invalid address metadata".to_string())
                            }
                        }
                        "udp_send_to" => {
                            // Send UDP datagram - extract socket and data before awaiting
                            let (socket_wrapper, data, addr) = {
                                let fut = fut_ref.borrow();
                                let socket = match &fut.metadata {
                                    Some(Value::UdpSocket(s)) => Arc::clone(s),
                                    _ => return self.mark_future_done(fut_ref, Err("udp_send_to: invalid socket metadata".to_string())),
                                };
                                let (data, addr) = match &fut.result {
                                    Some(Value::Map(m)) => {
                                        let m = m.borrow();
                                        let data = match m.get(&HashableValue::String(Rc::new("data".into()))) {
                                            Some(Value::String(s)) => s.as_bytes().to_vec(),
                                            Some(Value::List(l)) => {
                                                l.borrow().iter().filter_map(|v| match v {
                                                    Value::Int(i) if *i >= 0 && *i <= 255 => Some(*i as u8),
                                                    _ => None,
                                                }).collect()
                                            }
                                            _ => vec![],
                                        };
                                        let addr = match m.get(&HashableValue::String(Rc::new("addr".into()))) {
                                            Some(Value::String(s)) => s.to_string(),
                                            _ => String::new(),
                                        };
                                        (data, addr)
                                    }
                                    _ => return self.mark_future_done(fut_ref, Err("udp_send_to: invalid metadata".to_string())),
                                };
                                (socket, data, addr)
                            };
                            let socket = socket_wrapper.socket.lock().await;
                            match socket.send_to(&data, &addr).await {
                                Ok(n) => Ok(Value::Int(n as i64)),
                                Err(e) => Err(format!("udp_send_to: {e}")),
                            }
                        }
                        "udp_recv_from" => {
                            // Receive UDP datagram
                            if let Some(Value::UdpSocket(socket_wrapper)) = &metadata {
                                let max_bytes = if let Some(Value::Int(n)) = &metadata {
                                    *n as usize
                                } else {
                                    65535
                                };
                                let socket = socket_wrapper.socket.lock().await;
                                let mut buf = vec![0u8; max_bytes];
                                match socket.recv_from(&mut buf).await {
                                    Ok((n, addr)) => {
                                        buf.truncate(n);
                                        let data = match String::from_utf8(buf.clone()) {
                                            Ok(s) => Value::string(s),
                                            Err(_) => Value::list(buf.into_iter().map(|b| Value::Int(b as i64)).collect()),
                                        };
                                        // Return a map with data, host, port
                                        let result = Value::Map(Rc::new(RefCell::new({
                                            let mut m = std::collections::HashMap::new();
                                            m.insert(HashableValue::String(Rc::new("data".into())), data);
                                            m.insert(HashableValue::String(Rc::new("host".into())), Value::string(addr.ip().to_string()));
                                            m.insert(HashableValue::String(Rc::new("port".into())), Value::Int(addr.port() as i64));
                                            m
                                        })));
                                        Ok(result)
                                    }
                                    Err(e) => Err(format!("udp_recv_from: {e}")),
                                }
                            } else {
                                Err("udp_recv_from: invalid socket metadata".to_string())
                            }
                        }
                        // WebSocket operations
                        "ws_connect" => {
                            // Connect to a WebSocket server
                            if let Some(Value::String(url)) = &metadata {
                                match connect_async(url.as_str()).await {
                                    Ok((ws_stream, _response)) => {
                                        let (sink, stream) = ws_stream.split();
                                        let wrapper = WebSocketWrapper::new(sink, stream, url.to_string());
                                        Ok(Value::WebSocket(Arc::new(wrapper)))
                                    }
                                    Err(e) => Err(format!("ws_connect to {}: {e}", url)),
                                }
                            } else {
                                Err("ws_connect: invalid url metadata".to_string())
                            }
                        }
                        "ws_listen" => {
                            // Create a WebSocket server (bind TCP listener)
                            if let Some(Value::String(addr)) = &metadata {
                                match TcpListener::bind(addr.as_str()).await {
                                    Ok(listener) => {
                                        match WebSocketServerWrapper::new(listener) {
                                            Ok(wrapper) => Ok(Value::WebSocketServer(Arc::new(wrapper))),
                                            Err(e) => Err(format!("ws_listen: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("ws_listen on {}: {e}", addr)),
                                }
                            } else {
                                Err("ws_listen: invalid address metadata".to_string())
                            }
                        }
                        "ws_accept" => {
                            // Accept a new WebSocket connection
                            if let Some(Value::WebSocketServer(server_wrapper)) = &metadata {
                                let listener = server_wrapper.listener.lock().await;
                                match listener.accept().await {
                                    Ok((tcp_stream, peer_addr)) => {
                                        drop(listener); // Release lock before async operation
                                        let local_addr = server_wrapper.local_addr.clone();
                                        // Perform WebSocket handshake
                                        match tokio_tungstenite::accept_async(tcp_stream).await {
                                            Ok(ws_stream) => {
                                                let (sink, stream) = ws_stream.split();
                                                let wrapper = WebSocketServerConnWrapper::new(
                                                    sink,
                                                    stream,
                                                    peer_addr.to_string(),
                                                    local_addr,
                                                );
                                                Ok(Value::WebSocketServerConn(Arc::new(wrapper)))
                                            }
                                            Err(e) => Err(format!("ws_accept handshake: {e}")),
                                        }
                                    }
                                    Err(e) => Err(format!("ws_accept: {e}")),
                                }
                            } else {
                                Err("ws_accept: invalid server metadata".to_string())
                            }
                        }
                        "ws_send" | "ws_send_text" | "ws_send_binary" => {
                            // Send a message on a WebSocket client connection
                            let (ws_wrapper, message) = {
                                let fut = fut_ref.borrow();
                                let ws = match &fut.metadata {
                                    Some(Value::WebSocket(w)) => Arc::clone(w),
                                    _ => return self.mark_future_done(fut_ref, Err("ws_send: invalid websocket metadata".to_string())),
                                };
                                let msg = match &fut.result {
                                    Some(Value::String(s)) => WsMessage::Text(s.to_string().into()),
                                    Some(Value::List(l)) => {
                                        let bytes: Vec<u8> = l.borrow().iter().filter_map(|v| match v {
                                            Value::Int(i) if *i >= 0 && *i <= 255 => Some(*i as u8),
                                            _ => None,
                                        }).collect();
                                        WsMessage::Binary(bytes.into())
                                    }
                                    _ => return self.mark_future_done(fut_ref, Err("ws_send: invalid message data".to_string())),
                                };
                                (ws, msg)
                            };
                            let mut sink = ws_wrapper.sink.lock().await;
                            match sink.send(message).await {
                                Ok(()) => Ok(Value::Null),
                                Err(e) => {
                                    ws_wrapper.set_closed();
                                    Err(format!("ws_send: {e}"))
                                }
                            }
                        }
                        "ws_receive" => {
                            // Receive a message from a WebSocket client connection
                            if let Some(Value::WebSocket(ws_wrapper)) = &metadata {
                                let mut stream = ws_wrapper.stream.lock().await;
                                match stream.next().await {
                                    Some(Ok(msg)) => {
                                        let result = match msg {
                                            WsMessage::Text(text) => {
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("text"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::string(text.to_string()));
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Binary(data) => {
                                                let bytes: Vec<Value> = data.iter().map(|b| Value::Int(*b as i64)).collect();
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("binary"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::list(bytes));
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => {
                                                // Control frames - return empty with type
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("control"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Close(_) => {
                                                ws_wrapper.set_closed();
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("close"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                        };
                                        Ok(result)
                                    }
                                    Some(Err(e)) => {
                                        ws_wrapper.set_closed();
                                        Err(format!("ws_receive: {e}"))
                                    }
                                    None => {
                                        ws_wrapper.set_closed();
                                        let mut m = std::collections::HashMap::new();
                                        m.insert(HashableValue::String(Rc::new("type".into())), Value::string("close"));
                                        m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                        Ok(Value::Map(Rc::new(RefCell::new(m))))
                                    }
                                }
                            } else {
                                Err("ws_receive: invalid websocket metadata".to_string())
                            }
                        }
                        "ws_close" => {
                            // Close a WebSocket client connection
                            if let Some(Value::WebSocket(ws_wrapper)) = &metadata {
                                if !ws_wrapper.is_closed() {
                                    let mut sink = ws_wrapper.sink.lock().await;
                                    let _ = sink.send(WsMessage::Close(None)).await;
                                    let _ = sink.close().await;
                                    ws_wrapper.set_closed();
                                }
                                Ok(Value::Null)
                            } else {
                                Err("ws_close: invalid websocket metadata".to_string())
                            }
                        }
                        // WebSocket server connection operations
                        "ws_conn_send" | "ws_conn_send_text" | "ws_conn_send_binary" => {
                            let (conn_wrapper, message) = {
                                let fut = fut_ref.borrow();
                                let conn = match &fut.metadata {
                                    Some(Value::WebSocketServerConn(c)) => Arc::clone(c),
                                    _ => return self.mark_future_done(fut_ref, Err("ws_conn_send: invalid connection metadata".to_string())),
                                };
                                let msg = match &fut.result {
                                    Some(Value::String(s)) => WsMessage::Text(s.to_string().into()),
                                    Some(Value::List(l)) => {
                                        let bytes: Vec<u8> = l.borrow().iter().filter_map(|v| match v {
                                            Value::Int(i) if *i >= 0 && *i <= 255 => Some(*i as u8),
                                            _ => None,
                                        }).collect();
                                        WsMessage::Binary(bytes.into())
                                    }
                                    _ => return self.mark_future_done(fut_ref, Err("ws_conn_send: invalid message data".to_string())),
                                };
                                (conn, msg)
                            };
                            let mut sink = conn_wrapper.sink.lock().await;
                            match sink.send(message).await {
                                Ok(()) => Ok(Value::Null),
                                Err(e) => {
                                    conn_wrapper.set_closed();
                                    Err(format!("ws_conn_send: {e}"))
                                }
                            }
                        }
                        "ws_conn_receive" => {
                            if let Some(Value::WebSocketServerConn(conn_wrapper)) = &metadata {
                                let mut stream = conn_wrapper.stream.lock().await;
                                match stream.next().await {
                                    Some(Ok(msg)) => {
                                        let result = match msg {
                                            WsMessage::Text(text) => {
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("text"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::string(text.to_string()));
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Binary(data) => {
                                                let bytes: Vec<Value> = data.iter().map(|b| Value::Int(*b as i64)).collect();
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("binary"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::list(bytes));
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => {
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("control"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                            WsMessage::Close(_) => {
                                                conn_wrapper.set_closed();
                                                let mut m = std::collections::HashMap::new();
                                                m.insert(HashableValue::String(Rc::new("type".into())), Value::string("close"));
                                                m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                                Value::Map(Rc::new(RefCell::new(m)))
                                            }
                                        };
                                        Ok(result)
                                    }
                                    Some(Err(e)) => {
                                        conn_wrapper.set_closed();
                                        Err(format!("ws_conn_receive: {e}"))
                                    }
                                    None => {
                                        conn_wrapper.set_closed();
                                        let mut m = std::collections::HashMap::new();
                                        m.insert(HashableValue::String(Rc::new("type".into())), Value::string("close"));
                                        m.insert(HashableValue::String(Rc::new("data".into())), Value::Null);
                                        Ok(Value::Map(Rc::new(RefCell::new(m))))
                                    }
                                }
                            } else {
                                Err("ws_conn_receive: invalid connection metadata".to_string())
                            }
                        }
                        "ws_conn_close" => {
                            if let Some(Value::WebSocketServerConn(conn_wrapper)) = &metadata {
                                if !conn_wrapper.is_closed() {
                                    let mut sink = conn_wrapper.sink.lock().await;
                                    let _ = sink.send(WsMessage::Close(None)).await;
                                    let _ = sink.close().await;
                                    conn_wrapper.set_closed();
                                }
                                Ok(Value::Null)
                            } else {
                                Err("ws_conn_close: invalid connection metadata".to_string())
                            }
                        }
                        "all" => {
                            // Async.all - wait for all futures in the list
                            if let Some(Value::List(futures_list)) = &metadata {
                                let futures = futures_list.borrow().clone();
                                let mut results = Vec::with_capacity(futures.len());

                                for (i, future_val) in futures.iter().enumerate() {
                                    // Recursively wait for each future
                                    let result = Box::pin(self.wait_for_future(future_val)).await;
                                    // Check if this was an error (Value::String starting with "Error:")
                                    if let Value::String(s) = &result {
                                        if s.starts_with("Error:") {
                                            return self.mark_future_done(
                                                fut_ref,
                                                Err(format!("Async.all: future at index {i} failed: {s}"))
                                            );
                                        }
                                    }
                                    results.push(result);
                                }

                                Ok(Value::list(results))
                            } else {
                                Err("Async.all: invalid futures list metadata".to_string())
                            }
                        }
                        "race" => {
                            // Async.race - wait for first future to complete
                            if let Some(Value::List(futures_list)) = &metadata {
                                let futures = futures_list.borrow().clone();

                                if futures.is_empty() {
                                    return self.mark_future_done(
                                        fut_ref,
                                        Err("Async.race: empty futures list".to_string())
                                    );
                                }

                                // Poll all futures repeatedly until one completes
                                loop {
                                    for future_val in &futures {
                                        if let Value::Future(inner_ref) = future_val {
                                            let inner = inner_ref.borrow();
                                            match &inner.status {
                                                FutureStatus::Ready => {
                                                    return self.mark_future_done(
                                                        fut_ref,
                                                        Ok(inner.result.clone().unwrap_or(Value::Null))
                                                    );
                                                }
                                                FutureStatus::Failed(err) => {
                                                    return self.mark_future_done(
                                                        fut_ref,
                                                        Err(format!("Async.race: {err}"))
                                                    );
                                                }
                                                FutureStatus::Pending => {
                                                    // Check if this is a sleep or other kind we can advance
                                                    if let Some(kind) = inner.kind() {
                                                        if kind == "sleep" {
                                                            // Drop borrow, wait for it, continue
                                                            drop(inner);
                                                            let result = Box::pin(self.wait_for_future(future_val)).await;
                                                            return self.mark_future_done(fut_ref, Ok(result));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Yield and try again
                                    tokio::task::yield_now().await;
                                }
                            } else {
                                Err("Async.race: invalid futures list metadata".to_string())
                            }
                        }
                        "timeout" => {
                            // Async.timeout - add timeout to a future
                            if let Some(Value::Map(map_ref)) = &metadata {
                                let (future_val, ms) = {
                                    let map = map_ref.borrow();
                                    let inner_future = map.get(&HashableValue::String(Rc::new("future".into())));
                                    let timeout_ms = map.get(&HashableValue::String(Rc::new("ms".into())));

                                    match (inner_future, timeout_ms) {
                                        (Some(future_val), Some(Value::Int(ms))) => {
                                            (future_val.clone(), *ms)
                                        }
                                        _ => {
                                            return self.mark_future_done(
                                                fut_ref,
                                                Err("Async.timeout: invalid metadata (expected future and ms)".to_string())
                                            );
                                        }
                                    }
                                };

                                let duration = std::time::Duration::from_millis(ms as u64);

                                // Use tokio timeout
                                match tokio::time::timeout(
                                    duration,
                                    Box::pin(self.wait_for_future(&future_val))
                                ).await {
                                    Ok(result) => Ok(result),
                                    Err(_) => Err(format!("Async.timeout: operation timed out after {ms}ms")),
                                }
                            } else {
                                Err("Async.timeout: invalid metadata".to_string())
                            }
                        }
                        "spawn" => {
                            // Async.spawn - run closure on separate OS thread
                            // For true parallelism, we need to handle closures specially:
                            // - Closures with no upvalues (pure functions) can theoretically be
                            //   spawned on separate threads, but still need VM execution
                            // - Closures with captured state use Rc (not Send) and cannot be
                            //   moved across threads safely
                            //
                            // Current limitation: spawn returns a future that will execute
                            // the closure when awaited, but not on a separate OS thread.
                            // True parallelism requires VM-level changes to support thread-safe
                            // execution (Arc+Mutex or per-thread VM instances).
                            //
                            // For now, we mark this as pending and let the coroutine system
                            // handle it cooperatively - the spawned "task" will run interleaved
                            // with other async work, similar to JavaScript's event loop model.
                            if let Some(Value::Closure(closure)) = &metadata {
                                // Check if closure has captured state
                                if closure.upvalues.is_empty() {
                                    // Pure closure - could theoretically parallelize
                                    // For now, just mark as ready for cooperative execution
                                    // The VM will need to call this closure when the coroutine resumes
                                    Ok(Value::Closure(Rc::clone(closure)))
                                } else {
                                    // Closure with captured state - cooperative execution only
                                    Ok(Value::Closure(Rc::clone(closure)))
                                }
                            } else {
                                Err("Async.spawn: invalid closure metadata".to_string())
                            }
                        }
                        _ => {
                            // Unknown kind - poll until ready
                            Ok(Value::Null)
                        }
                    };

                    return self.mark_future_done(fut_ref, result);
                }

                // For other pending futures, poll until ready
                loop {
                    {
                        let fut = fut_ref.borrow();
                        match &fut.status {
                            FutureStatus::Ready => {
                                return fut.result.clone().unwrap_or(Value::Null);
                            }
                            FutureStatus::Failed(err) => {
                                return Value::string(format!("Error: {err}"));
                            }
                            FutureStatus::Pending => {
                                // Continue waiting
                            }
                        }
                    }
                    // Yield to allow other async work to progress
                    tokio::task::yield_now().await;
                }
            }
            _ => {
                // Not a future - just return null
                Value::Null
            }
        }
    }

    /// Mark a future as done with a result or error
    fn mark_future_done(&self, fut_ref: &Rc<RefCell<FutureState>>, result: Result<Value, String>) -> Value {
        let mut fut = fut_ref.borrow_mut();
        match result {
            Ok(value) => {
                fut.status = FutureStatus::Ready;
                fut.result = Some(value.clone());
                value
            }
            Err(err) => {
                fut.status = FutureStatus::Failed(err.clone());
                Value::string(format!("Error: {err}"))
            }
        }
    }

    /// Spawn an async task that will update a future when done
    pub fn spawn_native_future<F>(&self, future_state: Rc<RefCell<FutureState>>, task: F)
    where
        F: std::future::Future<Output = Result<Value, String>> + 'static,
    {
        let local_set = LocalSet::new();
        let future_ref = future_state.clone();

        local_set.spawn_local(async move {
            match task.await {
                Ok(value) => {
                    let mut fut = future_ref.borrow_mut();
                    fut.status = FutureStatus::Ready;
                    fut.result = Some(value);
                }
                Err(err) => {
                    let mut fut = future_ref.borrow_mut();
                    fut.status = FutureStatus::Failed(err);
                }
            }
        });

        self.runtime.block_on(local_set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = AsyncExecutor::new();
        assert!(executor.ready_queue.borrow().is_empty());
    }

    #[test]
    fn test_future_state_pending() {
        let future = FutureState::pending();
        assert!(future.is_pending());
        assert!(!future.is_ready());
        assert!(future.kind().is_none());
        assert!(future.metadata().is_none());
    }

    #[test]
    fn test_future_state_ready() {
        let future = FutureState::ready(Value::Int(42));
        assert!(future.is_ready());
        assert!(!future.is_pending());
        assert_eq!(future.result, Some(Value::Int(42)));
    }

    #[test]
    fn test_future_state_failed() {
        let future = FutureState::failed("error message".to_string());
        assert!(!future.is_pending());
        assert!(!future.is_ready());
        match future.status {
            FutureStatus::Failed(msg) => assert_eq!(msg, "error message"),
            _ => panic!("Expected Failed status"),
        }
    }

    #[test]
    fn test_future_state_with_metadata() {
        let future = FutureState::pending_with_metadata(Value::Int(100), "sleep".to_string());
        assert!(future.is_pending());
        assert_eq!(future.kind(), Some("sleep"));
        assert_eq!(future.metadata(), Some(&Value::Int(100)));
    }

    #[test]
    fn test_coroutine_result_variants() {
        // Test Completed variant
        let result = CoroutineResult::Completed(Value::Int(42));
        match result {
            CoroutineResult::Completed(v) => assert_eq!(v, Value::Int(42)),
            _ => panic!("Expected Completed"),
        }

        // Test Suspended variant
        let state = CoroutineState::suspended(
            vec![],
            vec![],
            vec![],
            Value::Future(Rc::new(RefCell::new(FutureState::pending()))),
        );
        let result = CoroutineResult::Suspended {
            state: state.clone(),
            pending_future: Value::Null,
        };
        match result {
            CoroutineResult::Suspended { state: _, pending_future } => {
                assert_eq!(pending_future, Value::Null);
            }
            _ => panic!("Expected Suspended"),
        }
    }
}
