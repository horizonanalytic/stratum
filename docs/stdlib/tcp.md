# Tcp

TCP networking for client and server connections.

## Overview

The `Tcp` namespace provides functions for creating TCP client and server connections. TCP (Transmission Control Protocol) provides reliable, ordered, connection-based byte streams between networked applications.

All TCP operations are asynchronous and return `Future` values that must be awaited.

## Types

### TcpStream

A connected TCP socket that can read and write data.

| Method | Description |
|--------|-------------|
| `read(max_bytes?)` | Read up to max_bytes from the stream |
| `read_exact(num_bytes)` | Read exactly num_bytes from the stream |
| `write(data)` | Write data to the stream |
| `close()` | Close the connection |
| `peer_addr()` | Get the remote peer's address |
| `local_addr()` | Get the local address |

### TcpListener

A TCP server socket that accepts incoming connections.

| Method | Description |
|--------|-------------|
| `accept()` | Accept the next incoming connection |
| `local_addr()` | Get the listener's bound address |
| `close()` | Stop accepting connections |

---

## Functions

### `Tcp.connect(host, port)`

Connects to a remote TCP server.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `host` | `String` | Remote host to connect to (hostname or IP) |
| `port` | `Int` | Remote port (1-65535) |

**Returns:** `Future<TcpStream>` - A future that resolves to a connected stream

**Throws:** Error if connection fails or port is out of range

**Example:**

```stratum
// Connect to a server
let stream = await Tcp.connect("example.com", 80)
println("Connected to {stream.peer_addr()}")

// Send an HTTP request
await stream.write("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
let response = await stream.read()
println(response)
stream.close()
```

---

### `Tcp.listen(addr, port)`

Creates a TCP server that listens for incoming connections.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `addr` | `String` | Local address to bind (e.g., "0.0.0.0" for all interfaces) |
| `port` | `Int` | Local port to listen on (0-65535, 0 = auto-assign) |

**Returns:** `Future<TcpListener>` - A future that resolves to a listener

**Throws:** Error if the address/port cannot be bound

**Example:**

```stratum
// Start a server on port 8080
let server = await Tcp.listen("0.0.0.0", 8080)
println("Server listening on {server.local_addr()}")

// Accept connections in a loop
while true {
    let client = await server.accept()
    println("Client connected from {client.peer_addr()}")

    // Handle the client
    let data = await client.read()
    await client.write("Echo: {data}")
    client.close()
}
```

---

## TcpStream Methods

### `stream.read(max_bytes?)`

Reads data from the stream.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `max_bytes` | `Int?` | Maximum bytes to read (default: 8192) |

**Returns:** `Future<String>` - A future that resolves to the received data

**Throws:** Error if the connection is closed or reading fails

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 80)
await stream.write("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")

// Read up to 8192 bytes (default)
let response = await stream.read()
println(response)

// Read with a specific buffer size
let chunk = await stream.read(1024)
```

---

### `stream.read_exact(num_bytes)`

Reads exactly the specified number of bytes from the stream.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `num_bytes` | `Int` | Exact number of bytes to read (must be positive) |

**Returns:** `Future<String>` - A future that resolves to exactly num_bytes of data

**Throws:** Error if not enough data is available or connection closes

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 12345)

// Read a fixed-size header
let header = await stream.read_exact(16)
let length = int(header)

// Read the body based on header
let body = await stream.read_exact(length)
```

---

### `stream.write(data)`

Writes data to the stream.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | Data to send |

**Returns:** `Future<Int>` - A future that resolves to the number of bytes written

**Throws:** Error if the connection is closed or writing fails

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 80)

let bytes_sent = await stream.write("Hello, server!")
println("Sent {bytes_sent} bytes")
```

---

### `stream.close()`

Closes the TCP connection.

**Returns:** `Null`

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 80)
// ... use the stream
stream.close()
```

---

### `stream.peer_addr()`

Gets the remote peer's socket address.

**Returns:** `String` - The peer address in "host:port" format

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 80)
println("Connected to: {stream.peer_addr()}")  // e.g., "93.184.216.34:80"
```

---

### `stream.local_addr()`

Gets the local socket address.

**Returns:** `String` - The local address in "host:port" format

**Example:**

```stratum
let stream = await Tcp.connect("example.com", 80)
println("Local address: {stream.local_addr()}")  // e.g., "192.168.1.100:54321"
```

---

## TcpListener Methods

### `listener.accept()`

Waits for and accepts the next incoming connection.

**Returns:** `Future<TcpStream>` - A future that resolves to a connected client stream

**Example:**

```stratum
let server = await Tcp.listen("0.0.0.0", 8080)

// Accept a single connection
let client = await server.accept()
println("Client connected: {client.peer_addr()}")
```

---

### `listener.local_addr()`

Gets the address the listener is bound to.

**Returns:** `String` - The bound address in "host:port" format

**Example:**

```stratum
let server = await Tcp.listen("0.0.0.0", 0)  // Port 0 = auto-assign
println("Listening on: {server.local_addr()}")  // e.g., "0.0.0.0:54321"
```

---

### `listener.close()`

Stops the listener and closes the server socket.

**Returns:** `Null`

**Example:**

```stratum
let server = await Tcp.listen("0.0.0.0", 8080)
// ... accept connections
server.close()  // Stop accepting new connections
```

---

## Common Patterns

### Simple Echo Server

```stratum
let server = await Tcp.listen("0.0.0.0", 8080)
println("Echo server running on port 8080")

while true {
    let client = await server.accept()

    // Read until client disconnects
    while true {
        let data = await client.read()
        if len(data) == 0 {
            break  // Client disconnected
        }
        await client.write(data)  // Echo back
    }

    client.close()
}
```

### Simple HTTP Request

```stratum
let stream = await Tcp.connect("example.com", 80)

// Send HTTP request
await stream.write("GET / HTTP/1.0\r\n")
await stream.write("Host: example.com\r\n")
await stream.write("\r\n")

// Read response
let response = await stream.read()
println(response)

stream.close()
```

### Multi-Message Protocol

```stratum
let stream = await Tcp.connect("server.local", 9000)

// Send multiple messages
let messages = ["Hello", "World", "Goodbye"]
for msg in messages {
    let length = str(len(msg))
    await stream.write("{length}\n{msg}")

    let ack = await stream.read()
    println("Received: {ack}")
}

stream.close()
```

---

## See Also

- [Udp](udp.md) - Connectionless UDP sockets
- [WebSocket](websocket.md) - WebSocket protocol over TCP
- [Http](http.md) - Higher-level HTTP client
