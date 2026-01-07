# Udp

UDP networking for connectionless datagram communication.

## Overview

The `Udp` namespace provides functions for working with UDP (User Datagram Protocol) sockets. Unlike TCP, UDP is connectionless and provides no guarantees about delivery order or reliability, but offers lower latency and overhead.

UDP is ideal for applications where speed matters more than reliability, such as real-time games, DNS lookups, or streaming media.

All UDP operations are asynchronous and return `Future` values that must be awaited.

## Types

### UdpSocket

A bound UDP socket that can send and receive datagrams.

| Method | Description |
|--------|-------------|
| `send_to(data, host, port)` | Send data to a specific address |
| `recv_from(max_bytes?)` | Receive data from any sender |
| `local_addr()` | Get the socket's bound address |
| `close()` | Close the socket |

---

## Functions

### `Udp.bind(addr, port)`

Creates and binds a UDP socket to a local address.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `addr` | `String` | Local address to bind (e.g., "0.0.0.0" for all interfaces) |
| `port` | `Int` | Local port (0-65535, 0 = auto-assign) |

**Returns:** `Future<UdpSocket>` - A future that resolves to a bound socket

**Throws:** Error if the address/port cannot be bound

**Example:**

```stratum
// Bind to a specific port
let socket = await Udp.bind("0.0.0.0", 9000)
println("Socket bound to {socket.local_addr()}")

// Let the OS assign a port
let socket = await Udp.bind("0.0.0.0", 0)
println("Auto-assigned: {socket.local_addr()}")
```

---

## UdpSocket Methods

### `socket.send_to(data, host, port)`

Sends a datagram to a specific destination.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | The data to send |
| `host` | `String` | Destination hostname or IP address |
| `port` | `Int` | Destination port (1-65535) |

**Returns:** `Future<Int>` - A future that resolves to the number of bytes sent

**Throws:** Error if the destination is invalid or sending fails

**Example:**

```stratum
let socket = await Udp.bind("0.0.0.0", 0)

// Send a message to a remote host
let bytes_sent = await socket.send_to("Hello!", "192.168.1.100", 9000)
println("Sent {bytes_sent} bytes")

// Send to localhost
await socket.send_to("Ping", "127.0.0.1", 8080)
```

---

### `socket.recv_from(max_bytes?)`

Receives a datagram from any sender.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `max_bytes` | `Int?` | Maximum bytes to receive (default: 65535) |

**Returns:** `Future<Map>` - A future that resolves to a map containing:
  - `data` (`String`): The received data
  - `host` (`String`): The sender's IP address
  - `port` (`Int`): The sender's port

**Example:**

```stratum
let socket = await Udp.bind("0.0.0.0", 9000)

// Wait for a message
let result = await socket.recv_from()
println("Received '{result.data}' from {result.host}:{result.port}")

// With buffer size limit
let result = await socket.recv_from(1024)
```

---

### `socket.local_addr()`

Gets the address the socket is bound to.

**Returns:** `String` - The bound address in "host:port" format

**Example:**

```stratum
let socket = await Udp.bind("0.0.0.0", 0)
println("Bound to: {socket.local_addr()}")  // e.g., "0.0.0.0:54321"
```

---

### `socket.close()`

Closes the UDP socket.

**Returns:** `Null`

**Example:**

```stratum
let socket = await Udp.bind("0.0.0.0", 9000)
// ... use the socket
socket.close()
```

---

## Common Patterns

### Echo Server

```stratum
let socket = await Udp.bind("0.0.0.0", 9000)
println("UDP echo server running on port 9000")

while true {
    let result = await socket.recv_from()
    println("Received: '{result.data}' from {result.host}:{result.port}")

    // Echo back to sender
    await socket.send_to(result.data, result.host, result.port)
}
```

### Simple Client

```stratum
let socket = await Udp.bind("0.0.0.0", 0)

// Send a message to a server
await socket.send_to("Hello, server!", "127.0.0.1", 9000)

// Wait for response
let response = await socket.recv_from()
println("Server replied: {response.data}")

socket.close()
```

### Request-Response Pattern

```stratum
let socket = await Udp.bind("0.0.0.0", 0)

// Send a DNS-like query
let query = "lookup:example.com"
await socket.send_to(query, "192.168.1.1", 5353)

// Wait for response with timeout handling
let result = await socket.recv_from()
println("Response: {result.data}")

socket.close()
```

### Multi-Peer Communication

```stratum
let socket = await Udp.bind("0.0.0.0", 9000)

// Track known peers
let peers = []

while true {
    let result = await socket.recv_from()
    let peer = "{result.host}:{result.port}"

    if result.data == "JOIN" {
        peers.push(peer)
        println("New peer: {peer}")
    } else {
        // Broadcast to all other peers
        for p in peers {
            if p != peer {
                let parts = p.split(":")
                await socket.send_to(result.data, parts[0], int(parts[1]))
            }
        }
    }
}
```

---

## UDP vs TCP

| Feature | UDP | TCP |
|---------|-----|-----|
| Connection | Connectionless | Connection-oriented |
| Reliability | No guarantees | Guaranteed delivery |
| Ordering | No order guarantee | Ordered delivery |
| Speed | Faster, lower latency | Slower, higher latency |
| Use cases | Games, DNS, streaming | Web, file transfer, email |

---

## See Also

- [Tcp](tcp.md) - Reliable connection-oriented TCP sockets
- [WebSocket](websocket.md) - WebSocket protocol for real-time communication
