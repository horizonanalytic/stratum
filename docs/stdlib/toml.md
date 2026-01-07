# Toml

TOML (Tom's Obvious Minimal Language) encoding and decoding.

## Overview

The Toml namespace provides functions for converting between Stratum values and TOML strings. TOML is a configuration file format designed to be easy to read due to its clear semantics. It's commonly used for application configuration files.

Stratum's TOML implementation maps types as follows:

| TOML Type | Stratum Type |
|-----------|--------------|
| `boolean` | `Bool` |
| `integer` | `Int` |
| `float` | `Float` |
| `string` | `String` |
| `datetime` | `String` (ISO 8601 format) |
| `array` | `List` |
| `table` | `Map` |

**Important Limitations:**
- TOML does **not** support `null` values. Attempting to encode `null` will throw an error.
- TOML map keys **must** be strings. Non-string keys will throw an error.

---

## Functions

### `Toml.encode(value)` / `Toml.stringify(value)`

Converts a Stratum value to a TOML string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Map \| Struct` | The value to encode (must be a table/map type) |

**Returns:** `String` - A TOML-formatted string

**Throws:** Error if the value contains `null` or non-string map keys

**Example:**

```stratum
// Encode a simple configuration
let config = {
    title: "My App",
    debug: true,
    port: 8080
}
println(Toml.encode(config))
// title = "My App"
// debug = true
// port = 8080

// Nested tables
let settings = {
    database: {
        host: "localhost",
        port: 5432,
        name: "mydb"
    },
    server: {
        port: 8080,
        workers: 4
    }
}
println(Toml.encode(settings))
// [database]
// host = "localhost"
// port = 5432
// name = "mydb"
//
// [server]
// port = 8080
// workers = 4

// Arrays
let data = {
    ports: [8080, 8081, 8082],
    hosts: ["localhost", "server1", "server2"]
}
println(Toml.encode(data))
// ports = [8080, 8081, 8082]
// hosts = ["localhost", "server1", "server2"]

// Using the stringify alias
Toml.stringify({key: "value"})
```

**Note:** Unlike JSON, TOML cannot represent `null`:

```stratum
// This will throw an error
// Toml.encode({value: null})  // Error: TOML does not support null values
```

---

### `Toml.decode(toml_string)` / `Toml.parse(toml_string)`

Parses a TOML string into a Stratum value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `toml_string` | `String` | A valid TOML string |

**Returns:** `Map` - The parsed configuration as a Map

**Throws:** Error if the string is not valid TOML

**Example:**

```stratum
// Parse simple TOML
let toml = """
title = "My App"
debug = true
port = 8080
"""

let config = Toml.decode(toml)
println(config.title)  // "My App"
println(config.debug)  // true
println(config.port)   // 8080

// Parse nested tables
let settings_toml = """
[database]
host = "localhost"
port = 5432

[server]
port = 8080
workers = 4
"""

let settings = Toml.decode(settings_toml)
println(settings.database.host)  // "localhost"
println(settings.server.workers) // 4

// Parse arrays
let array_toml = """
ports = [8080, 8081, 8082]
"""

let data = Toml.decode(array_toml)
println(data.ports[0])  // 8080

// Using the parse alias
let obj = Toml.parse('key = "value"')
```

---

## Common Patterns

### Application Configuration

```stratum
// Load configuration file
let config_toml = File.read_text("config.toml")
let config = Toml.decode(config_toml)

// Access with defaults
let port = config.server?.port ?? 8080
let debug = config.debug ?? false
let log_level = config.logging?.level ?? "info"
```

### Project Metadata

```stratum
// Parse a project file (like Cargo.toml or pyproject.toml)
let project_toml = """
[package]
name = "my-project"
version = "1.0.0"
authors = ["Alice", "Bob"]

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"""

let project = Toml.decode(project_toml)
println(project.package.name)     // "my-project"
println(project.package.version)  // "1.0.0"
```

### Saving Configuration

```stratum
// Create and save configuration
let config = {
    app: {
        name: "MyApp",
        version: "1.0.0"
    },
    settings: {
        theme: "dark",
        language: "en"
    }
}

let toml_string = Toml.encode(config)
File.write_text("config.toml", toml_string)
```

---

## See Also

- [Json](json.md) - JSON encoding/decoding
- [Yaml](yaml.md) - YAML encoding/decoding
- [File](file.md) - Reading/writing files
