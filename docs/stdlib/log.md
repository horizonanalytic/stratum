# Log

Structured logging functions.

## Overview

The Log namespace provides functions for logging messages at different severity levels. It supports structured logging with context data, configurable output destinations (stdout, stderr, file), and customizable message formats.

Log levels from lowest to highest severity: `debug` < `info` < `warn` < `error`. Messages below the configured level are not output. The default level is `info`.

---

## Functions

### `Log.debug(message, ?context)`

Logs a debug-level message. Used for detailed diagnostic information during development.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | The log message |
| `context` | `Map?` | Optional key-value context data |

**Returns:** `Null`

**Example:**

```stratum
Log.debug("Processing started")
Log.debug("User data loaded", {user_id: 123, items: 5})

// Only visible when log level is "debug"
Log.set_level("debug")
Log.debug("Variable value", {x: 42, y: 100})
```

---

### `Log.info(message, ?context)`

Logs an info-level message. Used for general operational information.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | The log message |
| `context` | `Map?` | Optional key-value context data |

**Returns:** `Null`

**Example:**

```stratum
Log.info("Application started")
Log.info("Server listening", {port: 8080, host: "localhost"})
Log.info("Request processed", {method: "GET", path: "/api/users", duration_ms: 45})
```

---

### `Log.warn(message, ?context)` / `Log.warning(message, ?context)`

Logs a warning-level message. Used for potentially harmful situations.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | The log message |
| `context` | `Map?` | Optional key-value context data |

**Returns:** `Null`

**Example:**

```stratum
Log.warn("Configuration file not found, using defaults")
Log.warn("Rate limit approaching", {current: 95, limit: 100})
Log.warning("Deprecated API used", {endpoint: "/v1/users", replacement: "/v2/users"})
```

---

### `Log.error(message, ?context)`

Logs an error-level message. Used for error conditions that need attention.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | The log message |
| `context` | `Map?` | Optional key-value context data |

**Returns:** `Null`

**Example:**

```stratum
Log.error("Database connection failed")
Log.error("Failed to process request", {error: "timeout", path: "/api/data"})
Log.error("File not found", {path: "/config/app.json", operation: "read"})
```

---

### `Log.set_level(level)`

Sets the minimum log level. Messages below this level are not output.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `level` | `String` | Log level: `"debug"`, `"info"`, `"warn"`, or `"error"` |

**Returns:** `Null`

**Example:**

```stratum
// Show all messages including debug
Log.set_level("debug")

// Default: info and above
Log.set_level("info")

// Warnings and errors only
Log.set_level("warn")

// Errors only
Log.set_level("error")

// Set based on environment
let env = Env.get("ENVIRONMENT", "development")
if env == "production" {
    Log.set_level("warn")
} else {
    Log.set_level("debug")
}
```

---

### `Log.level()`

Returns the current log level.

**Parameters:** None

**Returns:** `String` - The current level: `"debug"`, `"info"`, `"warn"`, or `"error"`

**Example:**

```stratum
println("Current log level: " + Log.level())

// Temporarily change level
let original = Log.level()
Log.set_level("debug")
// ... detailed logging ...
Log.set_level(original)
```

---

### `Log.to_file(path)`

Directs log output to a file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the log file (appends to existing file) |

**Returns:** `Null`

**Example:**

```stratum
// Log to a file
Log.to_file("app.log")
Log.info("Application started")

// Log to dated file
let date = DateTime.format(DateTime.now(), "%Y-%m-%d")
Log.to_file("logs/app-" + date + ".log")
```

---

### `Log.to_stderr()`

Directs log output to standard error.

**Parameters:** None

**Returns:** `Null`

**Example:**

```stratum
// Send logs to stderr (useful for separating from program output)
Log.to_stderr()
Log.info("This goes to stderr")
```

---

### `Log.to_stdout()`

Directs log output to standard output (default).

**Parameters:** None

**Returns:** `Null`

**Example:**

```stratum
// Reset to stdout (after using file or stderr)
Log.to_stdout()
Log.info("This goes to stdout")
```

---

### `Log.set_format(format)`

Sets the format string for log messages.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `format` | `String` | Format string with placeholders |

**Format Placeholders:**

| Placeholder | Description |
|------------|-------------|
| `{level}` | Log level (DEBUG, INFO, WARN, ERROR) |
| `{timestamp}` | ISO 8601 timestamp with timezone |
| `{message}` | The log message text |

Context key-value pairs are appended after the formatted message.

**Returns:** `Null`

**Example:**

```stratum
// Default format
Log.set_format("[{level}] {timestamp} - {message}")

// Simple format
Log.set_format("{level}: {message}")

// Timestamp-first format
Log.set_format("{timestamp} [{level}] {message}")

// Custom for JSON logging
Log.set_format("{timestamp}|{level}|{message}")
```

---

## Common Patterns

### Application Logging Setup

```stratum
fx setup_logging() {
    let env = Env.get("ENVIRONMENT", "development")
    let log_level = Env.get("LOG_LEVEL", null)

    // Set level based on environment or override
    if log_level != null {
        Log.set_level(log_level)
    } else if env == "production" {
        Log.set_level("warn")
    } else {
        Log.set_level("debug")
    }

    // Set output destination
    let log_file = Env.get("LOG_FILE", null)
    if log_file != null {
        Log.to_file(log_file)
    }

    Log.info("Logging initialized", {level: Log.level(), environment: env})
}

setup_logging()
```

### Request Logging

```stratum
fx log_request(method, path, status, duration_ms) {
    let context = {
        method: method,
        path: path,
        status: status,
        duration_ms: duration_ms
    }

    if status >= 500 {
        Log.error("Server error", context)
    } else if status >= 400 {
        Log.warn("Client error", context)
    } else {
        Log.info("Request completed", context)
    }
}

log_request("GET", "/api/users", 200, 45)
log_request("POST", "/api/users", 400, 12)
log_request("GET", "/api/data", 500, 150)
```

### Operation Timing

```stratum
fx timed_operation(name, operation) {
    Log.debug("Starting " + name)
    let start = Time.start()

    let result = operation()

    let elapsed = Time.elapsed(start)
    Log.info("Completed " + name, {duration_ms: elapsed})

    return result
}

let data = timed_operation("data fetch", || {
    return Http.get("https://api.example.com/data").body
})
```

### Error Handling with Logging

```stratum
fx safe_file_read(path) {
    if !File.exists(path) {
        Log.error("File not found", {path: path})
        return null
    }

    Log.debug("Reading file", {path: path})

    let content = File.read_text(path)
    Log.debug("File read complete", {path: path, size: len(content)})

    return content
}
```

### Structured Logging for Events

```stratum
fx log_event(event_type, event_data) {
    let context = {
        event_type: event_type,
        timestamp: DateTime.format(DateTime.now(), "%Y-%m-%dT%H:%M:%S")
    }

    for key, value in event_data {
        context[key] = value
    }

    Log.info("Event: " + event_type, context)
}

log_event("user_login", {user_id: 123, ip: "192.168.1.1"})
log_event("purchase", {user_id: 123, amount: 99.99, currency: "USD"})
```

### Rotating Log Files

```stratum
fx get_log_path() {
    let date = DateTime.format(DateTime.now(), "%Y-%m-%d")
    let log_dir = "logs"

    if !Dir.exists(log_dir) {
        Dir.create(log_dir)
    }

    return Path.join(log_dir, "app-" + date + ".log")
}

// Set up daily log rotation
Log.to_file(get_log_path())
```

---

## See Also

- [DateTime](datetime.md) - Date and time functions
- [File](file.md) - File system operations
- [Env](env.md) - Environment variable access
- [System](system.md) - System information and control
