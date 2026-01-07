# WebSocket

WebSocket protocol for real-time bidirectional communication.

## Overview

The `WebSocket` namespace provides functions for creating WebSocket client and server connections. WebSocket is a protocol that enables full-duplex communication channels over a single TCP connection, ideal for real-time applications like chat, live updates, and multiplayer games.

All WebSocket operations are asynchronous and return `Future` values that must be awaited.

## Types

### WebSocket (Client)

A client-side WebSocket connection.

| Method | Description |
|--------|-------------|
| `send(message)` | Send a text or binary message |
| `send_text(text)` | Send a text message explicitly |
| `send_binary(data)` | Send a binary message explicitly |
| `receive()` | Receive the next message |
| `close()` | Close the connection |
| `url()` | Get the WebSocket URL |
| `is_closed()` | Check if the connection is closed |

### WebSocketServer

A WebSocket server that accepts incoming connections.

| Method | Description |
|--------|-------------|
| `accept()` | Accept the next incoming connection |
| `local_addr()` | Get the server's bound address |
| `close()` | Stop accepting connections |

### WebSocketServerConn

A server-side connection to a client.

| Method | Description |
|--------|-------------|
| `send(message)` | Send a text or binary message |
| `send_text(text)` | Send a text message explicitly |
| `send_binary(data)` | Send a binary message explicitly |
| `receive()` | Receive the next message |
| `close()` | Close the connection |
| `peer_addr()` | Get the client's address |
| `local_addr()` | Get the server's address |
| `is_closed()` | Check if the connection is closed |

## Message Type

Both `receive()` and `recv()` return a map with:

| Field | Type | Description |
|-------|------|-------------|
| `type` | `String` | Either `"text"` or `"binary"` |
| `data` | `String \| List` | Message content (String for text, List of bytes for binary) |

---

## Functions

### `WebSocket.connect(url)`

Connects to a WebSocket server.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | WebSocket URL (must start with `ws://` or `wss://`) |

**Returns:** `Future<WebSocket>` - A future that resolves to a connected WebSocket

**Throws:** Error if the URL is invalid or connection fails

**Example:**

```stratum
// Connect to a WebSocket server
let ws = await WebSocket.connect("wss://echo.websocket.org")
println("Connected to: {ws.url()}")

// Send and receive
await ws.send("Hello, WebSocket!")
let response = await ws.receive()
println("Received: {response.data}")

await ws.close()
```

---

### `WebSocket.listen(addr, port)`

Creates a WebSocket server that listens for incoming connections.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `addr` | `String` | Local address to bind (e.g., "0.0.0.0" for all interfaces) |
| `port` | `Int` | Local port to listen on (0-65535, 0 = auto-assign) |

**Returns:** `Future<WebSocketServer>` - A future that resolves to a server

**Aliases:** `WebSocket.server(addr, port)`

**Throws:** Error if the address/port cannot be bound

**Example:**

```stratum
// Start a WebSocket server
let server = await WebSocket.listen("0.0.0.0", 8080)
println("WebSocket server running on {server.local_addr()}")

while true {
    let client = await server.accept()
    println("Client connected from {client.peer_addr()}")

    // Echo messages back
    while !client.is_closed() {
        let msg = await client.receive()
        await client.send("Echo: {msg.data}")
    }
}
```

---

## WebSocket (Client) Methods

### `ws.send(message)`

Sends a message. Automatically determines whether to send as text or binary.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String \| List` | Message to send (String for text, List for binary) |

**Returns:** `Future<Null>` - A future that completes when sent

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")

// Send text
await ws.send("Hello!")

// Send binary data (list of bytes)
await ws.send([0x48, 0x65, 0x6C, 0x6C, 0x6F])
```

---

### `ws.send_text(text)`

Sends a text message explicitly.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `text` | `String` | Text message to send |

**Returns:** `Future<Null>` - A future that completes when sent

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")
await ws.send_text("This is definitely text")
```

---

### `ws.send_binary(data)`

Sends a binary message explicitly.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `List` | List of bytes (integers 0-255) to send |

**Returns:** `Future<Null>` - A future that completes when sent

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")

// Send binary data
let image_bytes = File.read_bytes("image.png")
await ws.send_binary(image_bytes)
```

---

### `ws.receive()`

Receives the next message from the server.

**Returns:** `Future<Map>` - A future that resolves to `{type: String, data: String|List}`

**Aliases:** `ws.recv()`

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")

let msg = await ws.receive()
if msg.type == "text" {
    println("Text message: {msg.data}")
} else {
    println("Binary message: {len(msg.data)} bytes")
}
```

---

### `ws.close()`

Closes the WebSocket connection.

**Returns:** `Future<Null>` - A future that completes when closed

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")
// ... use the connection
await ws.close()
```

---

### `ws.url()`

Gets the WebSocket URL.

**Returns:** `String` - The URL the client connected to

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")
println("Connected to: {ws.url()}")  // "wss://example.com/socket"
```

---

### `ws.is_closed()`

Checks if the connection is closed.

**Returns:** `Bool` - `true` if closed, `false` if still open

**Example:**

```stratum
let ws = await WebSocket.connect("wss://example.com/socket")

while !ws.is_closed() {
    let msg = await ws.receive()
    println(msg.data)
}
```

---

## WebSocketServer Methods

### `server.accept()`

Waits for and accepts the next incoming WebSocket connection.

**Returns:** `Future<WebSocketServerConn>` - A future that resolves to a client connection

**Example:**

```stratum
let server = await WebSocket.listen("0.0.0.0", 8080)

let client = await server.accept()
println("New client from: {client.peer_addr()}")
```

---

### `server.local_addr()`

Gets the address the server is bound to.

**Returns:** `String` - The bound address in "host:port" format

**Aliases:** `server.addr()`

**Example:**

```stratum
let server = await WebSocket.listen("0.0.0.0", 0)
println("Server running on: {server.local_addr()}")
```

---

### `server.close()`

Stops the server from accepting new connections.

**Returns:** `Null`

**Example:**

```stratum
let server = await WebSocket.listen("0.0.0.0", 8080)
// ... accept connections
server.close()
```

---

## WebSocketServerConn Methods

Server-side connections have the same messaging methods as client connections, plus address information.

### `conn.send(message)`

Sends a message to the connected client.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String \| List` | Message to send |

**Returns:** `Future<Null>`

---

### `conn.send_text(text)`

Sends a text message to the client.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `text` | `String` | Text message to send |

**Returns:** `Future<Null>`

---

### `conn.send_binary(data)`

Sends a binary message to the client.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `List` | List of bytes to send |

**Returns:** `Future<Null>`

---

### `conn.receive()`

Receives the next message from the client.

**Returns:** `Future<Map>` - A future that resolves to `{type: String, data: String|List}`

**Aliases:** `conn.recv()`

---

### `conn.close()`

Closes the connection to the client.

**Returns:** `Future<Null>`

---

### `conn.peer_addr()`

Gets the client's address.

**Returns:** `String` - The client's address in "host:port" format

---

### `conn.local_addr()`

Gets the server's local address for this connection.

**Returns:** `String` - The server's address

---

### `conn.is_closed()`

Checks if the connection to the client is closed.

**Returns:** `Bool`

---

## Common Patterns

### Chat Client

```stratum
let ws = await WebSocket.connect("wss://chat.example.com")

// Send username
await ws.send(Json.encode({"type": "login", "user": "Alice"}))

// Receive and display messages
while !ws.is_closed() {
    let msg = await ws.receive()
    let data = Json.decode(msg.data)

    if data.type == "message" {
        println("{data.from}: {data.text}")
    }
}
```

### Echo Server

```stratum
let server = await WebSocket.listen("0.0.0.0", 8080)
println("WebSocket echo server on port 8080")

while true {
    let client = await server.accept()
    println("Client connected: {client.peer_addr()}")

    // Handle client in a loop
    while !client.is_closed() {
        let msg = await client.receive()
        println("Received: {msg.data}")
        await client.send(msg.data)  // Echo back
    }

    println("Client disconnected")
}
```

### Broadcast Server

```stratum
let server = await WebSocket.listen("0.0.0.0", 8080)
let clients = []

// Accept loop (simplified - real implementation needs concurrency)
while true {
    let client = await server.accept()
    clients.push(client)

    // Broadcast to all clients
    let msg = await client.receive()
    for c in clients {
        if !c.is_closed() && c != client {
            await c.send(msg.data)
        }
    }
}
```

### JSON Protocol

```stratum
let ws = await WebSocket.connect("wss://api.example.com/ws")

// Send structured message
let request = {
    "action": "subscribe",
    "channel": "updates"
}
await ws.send(Json.encode(request))

// Handle responses
while !ws.is_closed() {
    let msg = await ws.receive()
    let data = Json.decode(msg.data)

    if data.action == "update" {
        println("Update: {data.value}")
    } else if data.action == "error" {
        println("Error: {data.message}")
        break
    }
}

await ws.close()
```

---

## See Also

- [Http](http.md) - HTTP client for request/response communication
- [Tcp](tcp.md) - Lower-level TCP sockets
- [Json](json.md) - JSON encoding for structured WebSocket messages
