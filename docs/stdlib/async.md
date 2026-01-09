# Async

Primitives for creating and working with asynchronous futures.

## Overview

The `Async` namespace provides basic primitives for creating Future values. Futures represent values that may not be available yet—they're either pending (still computing), ready (completed with a value), or failed (completed with an error).

Stratum's async model uses `await` to resolve futures:

```stratum
let future = Async.sleep(1000)  // Create pending future
let result = await future       // Wait for completion
```

Most async operations come from I/O namespaces like `Http`, `Tcp`, `File`, etc. The `Async` namespace provides low-level primitives for creating futures directly.

---

## Functions

### `Async.sleep(ms)`

Creates a future that resolves after the specified milliseconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ms` | `Int` | Milliseconds to sleep |

**Returns:** `Future<Null>` - Future that resolves to null after the delay

**Example:**

```stratum
// Wait for 2 seconds
await Async.sleep(2000)
println("2 seconds have passed")

// Use with timeout patterns
let result = race([
    some_slow_operation(),
    Async.sleep(5000)  // 5 second timeout
])
```

---

### `Async.ready(value)`

Creates an immediately resolved future with the given value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Value` | The value to wrap |

**Returns:** `Future<T>` - Immediately ready future containing the value

**Example:**

```stratum
let future = Async.ready(42)
let value = await future  // Returns immediately: 42

// Useful for APIs that expect futures
fx get_cached_or_fetch(key: String) -> Future<String> {
    if cache.has(key) {
        return Async.ready(cache.get(key))
    }
    return Http.get(url)
}
```

---

### `Async.failed(message)`

Creates an immediately failed future with the given error message.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Error message |

**Returns:** `Future<Never>` - Immediately failed future

**Throws:** Error when awaited

**Example:**

```stratum
fx validate_and_fetch(id: Int) -> Future<Data> {
    if id < 0 {
        return Async.failed("Invalid ID: must be non-negative")
    }
    return fetch_data(id)
}

// This will throw when awaited
let bad = Async.failed("Something went wrong")
await bad  // Throws: "Something went wrong"
```

---

## Future Type

The `Future<T>` type represents an asynchronous computation. Futures have three possible states:

| State | Description |
|-------|-------------|
| Pending | Computation is still in progress |
| Ready | Computation completed successfully with a value |
| Failed | Computation completed with an error |

### Awaiting Futures

Use the `await` keyword to block until a future resolves:

```stratum
let response = await Http.get("https://api.example.com/data")
println(response.body)
```

### Concurrent Execution

Multiple futures can run concurrently:

```stratum
// Start both requests
let future1 = Http.get("https://api.example.com/users")
let future2 = Http.get("https://api.example.com/posts")

// Wait for both
let users = await future1
let posts = await future2
```

---

## Combinators

### `Async.all(futures)`

Waits for all futures to complete and returns a list of results.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `futures` | `List<Future>` | List of futures to await |

**Returns:** `Future<List>` - Future that resolves to a list of all results

**Throws:** Fails immediately if any future fails

**Example:**

```stratum
let urls = [
    "https://api.example.com/users",
    "https://api.example.com/posts",
    "https://api.example.com/comments"
]

let futures = urls.map(|url| Http.get(url))
let responses = await Async.all(futures)
// responses is a list of all HTTP responses

// Process all results
for response in responses {
    println(response.status)
}
```

---

### `Async.race(futures)`

Returns the result of the first future to complete.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `futures` | `List<Future>` | List of futures to race |

**Returns:** `Future` - Future that resolves with the first completed result

**Example:**

```stratum
// Use multiple mirrors, take fastest response
let mirrors = [
    Http.get("https://mirror1.example.com/file"),
    Http.get("https://mirror2.example.com/file"),
    Http.get("https://mirror3.example.com/file")
]

let fastest = await Async.race(mirrors)
println("Got response from fastest mirror")
```

---

### `Async.timeout(future, ms)`

Adds a timeout to a future.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `future` | `Future` | The future to add timeout to |
| `ms` | `Int` | Timeout in milliseconds |

**Returns:** `Future` - Future that fails if timeout is exceeded

**Throws:** Error "Timeout" if the future doesn't complete in time

**Example:**

```stratum
// Fail if request takes more than 5 seconds
let response = await Async.timeout(
    Http.get("https://slow-api.example.com/data"),
    5000
)

// Or handle the timeout
try {
    let data = await Async.timeout(fetch_data(), 3000)
} catch (e) {
    if e == "Timeout" {
        println("Request timed out, using cached data")
        data = get_cached_data()
    }
}
```

---

### `Async.spawn(closure)`

Spawns a background task (cooperative concurrency).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `closure` | `Function` | Closure to execute in background |

**Returns:** `Future` - Future that resolves when the closure completes

**Example:**

```stratum
// Run computation in background
let background_task = Async.spawn(|| {
    // Do heavy computation
    let result = expensive_calculation()
    return result
})

// Continue doing other work
do_other_work()

// Get the result when needed
let result = await background_task
```

---

## Examples

### Delayed Operations

```stratum
fx countdown(n: Int) {
    for i in range(n, 0, -1) {
        println(i)
        await Async.sleep(1000)
    }
    println("Done!")
}

countdown(5)
```

### Conditional Async

```stratum
fx maybe_fetch(should_fetch: Bool, url: String) -> Future<String> {
    if should_fetch {
        return Http.get(url).body
    }
    return Async.ready("Default value")
}
```

### Error Handling

```stratum
fx safe_fetch(url: String) -> String? {
    try {
        let response = await Http.get(url)
        return response.body
    } catch {
        return null
    }
}

// Or return a failed future for callers to handle
fx validated_fetch(url: String) -> Future<String> {
    if !url.starts_with("https://") {
        return Async.failed("Only HTTPS URLs are allowed")
    }
    return Http.get(url).body
}
```

---

## See Also

- [Time](time.md) - Synchronous sleep and timing
- [Http](http.md) - HTTP client returning futures
- [Tcp](tcp.md) - TCP networking with async I/O
- [WebSocket](websocket.md) - WebSocket connections
