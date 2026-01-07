# Http

HTTP client for making web requests.

## Overview

The `Http` namespace provides functions for making HTTP requests to web servers. It supports all standard HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD) with configurable headers and timeouts.

All functions return a response map containing the status code, response body, headers, and a convenience `ok` flag.

## Response Type

All HTTP functions return a `Map` with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `status` | `Int` | HTTP status code (e.g., 200, 404, 500) |
| `body` | `String` | Response body as text |
| `headers` | `Map` | Response headers as key-value pairs |
| `ok` | `Bool` | `true` if status is 2xx, `false` otherwise |

## Options Parameter

Several functions accept an optional `options` map for configuration:

| Key | Type | Description |
|-----|------|-------------|
| `headers` | `Map` | Custom headers to send with the request |
| `timeout` | `Int` | Request timeout in milliseconds |

**Example:**

```stratum
let options = {
    "headers": {
        "Authorization": "Bearer my-token",
        "Content-Type": "application/json"
    },
    "timeout": 5000
}
```

---

## Functions

### `Http.get(url, options?)`

Performs an HTTP GET request.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// Simple GET request
let response = Http.get("https://api.example.com/users")
if response.ok {
    println(response.body)
}

// GET with custom headers
let response = Http.get("https://api.example.com/data", {
    "headers": {"Authorization": "Bearer token123"}
})
println(response.status)  // 200
```

---

### `Http.post(url, body?, options?)`

Performs an HTTP POST request with an optional body.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `body` | `String?` | Optional request body |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// POST with JSON body
let payload = Json.encode({"name": "Alice", "email": "alice@example.com"})
let response = Http.post("https://api.example.com/users", payload, {
    "headers": {"Content-Type": "application/json"}
})

if response.ok {
    let user = Json.decode(response.body)
    println("Created user with ID: {user.id}")
}

// POST without body
let response = Http.post("https://api.example.com/trigger")
```

---

### `Http.put(url, body?, options?)`

Performs an HTTP PUT request with an optional body.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `body` | `String?` | Optional request body |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// Update a resource
let payload = Json.encode({"name": "Alice Smith", "email": "alice.smith@example.com"})
let response = Http.put("https://api.example.com/users/123", payload, {
    "headers": {"Content-Type": "application/json"}
})

if response.ok {
    println("User updated successfully")
} else {
    println("Error: {response.status}")
}
```

---

### `Http.patch(url, body?, options?)`

Performs an HTTP PATCH request with an optional body for partial updates.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `body` | `String?` | Optional request body |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// Partial update - only update the email
let payload = Json.encode({"email": "newemail@example.com"})
let response = Http.patch("https://api.example.com/users/123", payload, {
    "headers": {"Content-Type": "application/json"}
})

println(response.status)  // 200
```

---

### `Http.delete(url, options?)`

Performs an HTTP DELETE request.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// Delete a resource
let response = Http.delete("https://api.example.com/users/123", {
    "headers": {"Authorization": "Bearer admin-token"}
})

if response.status == 204 {
    println("User deleted")
} else if response.status == 404 {
    println("User not found")
}
```

---

### `Http.head(url, options?)`

Performs an HTTP HEAD request, retrieving only headers without the body.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | The URL to request |
| `options` | `Map?` | Optional configuration (headers, timeout) |

**Returns:** `Map` - Response with status, empty body, headers, and ok fields

**Throws:** Error if the URL is invalid or the request fails

**Example:**

```stratum
// Check if a resource exists without downloading it
let response = Http.head("https://example.com/large-file.zip")

if response.ok {
    let size = response.headers["content-length"]
    println("File size: {size} bytes")
} else {
    println("File not found")
}
```

---

## Common Patterns

### Checking Response Status

```stratum
let response = Http.get("https://api.example.com/data")

if response.ok {
    // Status is 2xx
    let data = Json.decode(response.body)
    // ... process data
} else if response.status == 404 {
    println("Resource not found")
} else if response.status >= 500 {
    println("Server error: {response.status}")
}
```

### Working with JSON APIs

```stratum
// GET JSON data
let response = Http.get("https://api.example.com/users")
let users = Json.decode(response.body)

for user in users {
    println("{user.name}: {user.email}")
}

// POST JSON data
let new_user = {"name": "Bob", "role": "admin"}
let response = Http.post(
    "https://api.example.com/users",
    Json.encode(new_user),
    {"headers": {"Content-Type": "application/json"}}
)
```

### Setting Timeouts

```stratum
// Set a 10-second timeout
let response = Http.get("https://slow-api.example.com/data", {
    "timeout": 10000
})
```

---

## See Also

- [Json](json.md) - JSON encoding/decoding for API payloads
- [Url](url.md) - URL encoding for query parameters
- [WebSocket](websocket.md) - Real-time bidirectional communication
