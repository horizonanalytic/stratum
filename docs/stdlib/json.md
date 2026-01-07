# Json

JSON (JavaScript Object Notation) encoding and decoding.

## Overview

The Json namespace provides functions for converting between Stratum values and JSON strings. JSON is a widely-used data interchange format, making these functions essential for working with web APIs, configuration files, and data storage.

Stratum's JSON implementation handles all standard JSON types and maps them to their Stratum equivalents:

| JSON Type | Stratum Type |
|-----------|--------------|
| `null` | `Null` |
| `boolean` | `Bool` |
| `number` (integer) | `Int` |
| `number` (decimal) | `Float` |
| `string` | `String` |
| `array` | `List` |
| `object` | `Map` |

**Note:** JSON does not support `NaN` or `Infinity` float values. When encoding, these are converted to `null`.

---

## Functions

### `Json.encode(value)` / `Json.stringify(value)`

Converts a Stratum value to a JSON string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Any` | The value to encode |

**Returns:** `String` - A JSON-formatted string

**Example:**

```stratum
// Encode simple values
Json.encode(42)           // "42"
Json.encode("hello")      // "\"hello\""
Json.encode(true)         // "true"
Json.encode(null)         // "null"

// Encode lists
Json.encode([1, 2, 3])    // "[1,2,3]"

// Encode maps
let user = {name: "Alice", age: 30}
Json.encode(user)         // "{\"name\":\"Alice\",\"age\":30}"

// Nested structures
let data = {
    users: [
        {name: "Alice", active: true},
        {name: "Bob", active: false}
    ],
    count: 2
}
Json.encode(data)
// {"users":[{"name":"Alice","active":true},{"name":"Bob","active":false}],"count":2}

// Using the stringify alias
Json.stringify({key: "value"})  // "{\"key\":\"value\"}"
```

**Note:** Special float values are converted: `NaN` and `Infinity` become `null`.

```stratum
Json.encode(Math.NAN)           // "null"
Json.encode(Math.INFINITY)      // "null"
Json.encode(Math.NEG_INFINITY)  // "null"
```

---

### `Json.decode(json_string)` / `Json.parse(json_string)`

Parses a JSON string into a Stratum value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `json_string` | `String` | A valid JSON string |

**Returns:** `Any` - The parsed Stratum value

**Throws:** Error if the string is not valid JSON

**Example:**

```stratum
// Decode simple values
Json.decode("42")           // 42
Json.decode("\"hello\"")    // "hello"
Json.decode("true")         // true
Json.decode("null")         // null

// Decode arrays
Json.decode("[1, 2, 3]")    // [1, 2, 3]

// Decode objects
let user = Json.decode('{"name": "Alice", "age": 30}')
println(user.name)  // "Alice"
println(user.age)   // 30

// Decode nested structures
let json = '{"users": [{"name": "Alice"}, {"name": "Bob"}]}'
let data = Json.decode(json)
println(data.users[0].name)  // "Alice"

// Using the parse alias
let obj = Json.parse('{"key": "value"}')

// Error handling
// Json.decode("invalid json")  // Throws: Invalid JSON
```

---

## Common Patterns

### Working with API Responses

```stratum
// Parse API response
let response_body = '{"status": "ok", "data": [1, 2, 3]}'
let response = Json.decode(response_body)

if response.status == "ok" {
    for item in response.data {
        println(item)
    }
}
```

### Configuration Files

```stratum
// Load JSON config
let config_json = File.read_text("config.json")
let config = Json.decode(config_json)

// Access configuration values
let port = config.server.port
let debug = config.debug ?? false
```

### Round-trip Encoding

```stratum
// Encode and decode preserves structure
let original = {
    name: "Test",
    values: [1, 2, 3],
    nested: {a: 1, b: 2}
}

let json = Json.encode(original)
let restored = Json.decode(json)

assert_eq(original.name, restored.name)
assert_eq(len(original.values), len(restored.values))
```

---

## See Also

- [Toml](toml.md) - TOML encoding/decoding
- [Yaml](yaml.md) - YAML encoding/decoding
- [Http](http.md) - HTTP requests (often returns JSON)
- [File](file.md) - Reading/writing files
