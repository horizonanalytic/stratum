# Yaml

YAML (YAML Ain't Markup Language) encoding and decoding.

## Overview

The Yaml namespace provides functions for converting between Stratum values and YAML strings. YAML is a human-readable data serialization format commonly used for configuration files, data exchange, and storing structured data.

Stratum's YAML implementation maps types as follows:

| YAML Type | Stratum Type |
|-----------|--------------|
| `null` / `~` | `Null` |
| `boolean` | `Bool` |
| `integer` | `Int` |
| `float` | `Float` |
| `string` | `String` |
| `sequence` | `List` |
| `mapping` | `Map` |

YAML is a superset of JSON, so any valid JSON is also valid YAML. YAML additionally supports:
- Multi-line strings
- Comments
- Anchors and aliases
- Multiple documents

---

## Functions

### `Yaml.encode(value)` / `Yaml.stringify(value)`

Converts a Stratum value to a YAML string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Any` | The value to encode |

**Returns:** `String` - A YAML-formatted string

**Example:**

```stratum
// Encode simple values
Yaml.encode(42)        // "42\n"
Yaml.encode("hello")   // "hello\n"
Yaml.encode(true)      // "true\n"
Yaml.encode(null)      // "null\n"

// Encode lists
println(Yaml.encode([1, 2, 3]))
// - 1
// - 2
// - 3

// Encode maps
let user = {name: "Alice", age: 30}
println(Yaml.encode(user))
// name: Alice
// age: 30

// Nested structures
let config = {
    server: {
        host: "localhost",
        port: 8080
    },
    features: ["auth", "logging", "cache"]
}
println(Yaml.encode(config))
// server:
//   host: localhost
//   port: 8080
// features:
//   - auth
//   - logging
//   - cache

// Using the stringify alias
Yaml.stringify({key: "value"})
```

---

### `Yaml.decode(yaml_string)` / `Yaml.parse(yaml_string)`

Parses a YAML string into a Stratum value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `yaml_string` | `String` | A valid YAML string |

**Returns:** `Any` - The parsed Stratum value

**Throws:** Error if the string is not valid YAML

**Example:**

```stratum
// Decode simple values
Yaml.decode("42")        // 42
Yaml.decode("hello")     // "hello"
Yaml.decode("true")      // true
Yaml.decode("null")      // null
Yaml.decode("~")         // null

// Decode sequences
let list = Yaml.decode("""
- apple
- banana
- cherry
""")
println(list)  // ["apple", "banana", "cherry"]

// Decode mappings
let user = Yaml.decode("""
name: Alice
age: 30
active: true
""")
println(user.name)    // "Alice"
println(user.age)     // 30
println(user.active)  // true

// Decode nested structures
let config = Yaml.decode("""
database:
  host: localhost
  port: 5432
  credentials:
    user: admin
    password: secret
""")
println(config.database.host)                   // "localhost"
println(config.database.credentials.user)       // "admin"

// Using the parse alias
let obj = Yaml.parse("key: value")
```

---

## Common Patterns

### Configuration Files

```stratum
// Load YAML configuration
let config_yaml = File.read_text("config.yaml")
let config = Yaml.decode(config_yaml)

// Access nested values
let db_host = config.database?.host ?? "localhost"
let db_port = config.database?.port ?? 5432
```

### Docker Compose Style

```stratum
// Parse docker-compose.yaml style configuration
let compose = Yaml.decode("""
version: "3.8"
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
      - "443:443"
  db:
    image: postgres:13
    environment:
      POSTGRES_PASSWORD: secret
""")

for service_name, service in compose.services {
    println("Service: " + service_name)
    println("  Image: " + service.image)
}
```

### Multi-line Strings

```stratum
// YAML supports multi-line strings with | or >
let doc = Yaml.decode("""
description: |
  This is a multi-line
  description that preserves
  line breaks.
summary: >
  This is a folded string
  that becomes a single line.
""")

println(doc.description)  // Multi-line with newlines preserved
println(doc.summary)      // Single line (newlines become spaces)
```

### Saving YAML

```stratum
// Create and save YAML configuration
let config = {
    app: {
        name: "MyApp",
        version: "1.0.0"
    },
    features: ["auth", "api", "admin"],
    settings: {
        debug: false,
        log_level: "info"
    }
}

let yaml_string = Yaml.encode(config)
File.write_text("config.yaml", yaml_string)
```

### JSON Compatibility

```stratum
// YAML can parse JSON
let json_str = '{"name": "Alice", "scores": [95, 87, 92]}'
let data = Yaml.decode(json_str)
println(data.name)       // "Alice"
println(data.scores[0])  // 95
```

---

## See Also

- [Json](json.md) - JSON encoding/decoding
- [Toml](toml.md) - TOML encoding/decoding
- [File](file.md) - Reading/writing files
