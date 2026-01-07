# Map

Methods available on map (dictionary) values.

## Overview

Maps in Stratum are mutable key-value collections backed by hash tables. Maps are created using curly brace syntax with colon-separated key-value pairs and support direct key access via bracket notation.

Map methods are called on map values using dot notation: `{"a": 1}.len()`.

**Key characteristics:**
- Mutable: Methods like `set()` and `remove()` modify the map in-place
- Reference semantics: Assigning a map to a new variable creates a reference, not a copy
- Hashable keys only: Keys must be `Null`, `Bool`, `Int`, or `String`
- Any value type: Values can be any type, including lists, other maps, and structs
- Unordered: Key iteration order is not guaranteed

---

## Creating Maps

```stratum
// Empty map
let empty = {}

// Map with string keys
let scores = {"alice": 95, "bob": 87, "charlie": 92}

// Map with integer keys
let sparse = {0: "first", 100: "hundredth"}

// Map with boolean keys
let flags = {true: "enabled", false: "disabled"}

// Nested maps
let config = {
    "database": {"host": "localhost", "port": 5432},
    "cache": {"enabled": true, "ttl": 3600}
}
```

---

## Hashable Key Types

Maps only accept hashable types as keys:

| Type | Example |
|------|---------|
| `Null` | `{null: "value"}` |
| `Bool` | `{true: 1, false: 0}` |
| `Int` | `{42: "answer"}` |
| `String` | `{"name": "Alice"}` |

**Note:** Using a non-hashable type (List, Map, Struct, etc.) as a key will throw an error.

```stratum
// These will throw UnhashableType error
let bad1 = {[1, 2]: "value"}     // List as key - ERROR
let bad2 = {{"a": 1}: "value"}   // Map as key - ERROR
```

---

## Index Access

Maps support bracket notation for reading and writing entries.

```stratum
let scores = {"alice": 95, "bob": 87}

// Read by key
scores["alice"]     // 95
scores["bob"]       // 87

// Missing key returns null
scores["charlie"]   // null

// Write by key (insert or update)
scores["charlie"] = 92   // Insert new entry
scores["alice"] = 100    // Update existing entry
println(scores)  // {"alice": 100, "bob": 87, "charlie": 92}
```

---

## Properties

### `.len()` / `.length()`

Returns the number of key-value pairs in the map.

**Returns:** `Int` - The number of entries

**Example:**

```stratum
{"a": 1, "b": 2, "c": 3}.len()   // 3
{}.len()                          // 0
{"x": 10}.length()                // 1
```

---

### `.is_empty()`

Checks if the map has zero entries.

**Returns:** `Bool` - `true` if the map is empty, `false` otherwise

**Example:**

```stratum
{}.is_empty()                     // true
{"a": 1}.is_empty()               // false
```

---

## Access Methods

### `.get(key)` / `.get(key, default)`

Returns the value associated with a key, or a default value if the key is not found.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `Null \| Bool \| Int \| String` | The key to look up |
| `default` | `T?` | Optional value to return if key not found (defaults to `null`) |

**Returns:** `T` - The value associated with the key, or the default value

**Throws:** `UnhashableType` if the key is not a hashable type

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87}

// Basic lookup
scores.get("alice")           // 95
scores.get("charlie")         // null

// With default value
scores.get("charlie", 0)      // 0
scores.get("alice", 0)        // 95 (key exists, ignores default)

// Use with conditionals
let score = scores.get("dave", -1)
if score == -1 {
    println("Player not found")
}
```

---

### `.contains_key(key)` / `.has(key)`

Checks if a key exists in the map.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `Null \| Bool \| Int \| String` | The key to check for |

**Returns:** `Bool` - `true` if the key exists, `false` otherwise

**Throws:** `UnhashableType` if the key is not a hashable type

**Aliases:** `has()`

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87}

scores.contains_key("alice")  // true
scores.contains_key("charlie")// false

scores.has("bob")             // true
scores.has("dave")            // false

// Common pattern: check before access
if scores.has("alice") {
    println("Alice's score: " + str(scores["alice"]))
}
```

---

## Mutation Methods

### `.set(key, value)`

Inserts or updates a key-value pair in the map. Modifies the map in-place.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `Null \| Bool \| Int \| String` | The key to set |
| `value` | `T` | The value to associate with the key |

**Returns:** `Map` - The map itself (enables method chaining)

**Throws:** `UnhashableType` if the key is not a hashable type

**Example:**

```stratum
let scores = {"alice": 95}

// Add new entry
scores.set("bob", 87)
println(scores)  // {"alice": 95, "bob": 87}

// Update existing entry
scores.set("alice", 100)
println(scores)  // {"alice": 100, "bob": 87}

// Method chaining
scores.set("charlie", 92).set("dave", 88).set("eve", 91)
println(scores.len())  // 5

// Build a map incrementally
let config = {}
config
    .set("host", "localhost")
    .set("port", 8080)
    .set("debug", true)
```

---

### `.remove(key)`

Removes a key-value pair from the map and returns the removed value. Modifies the map in-place.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `Null \| Bool \| Int \| String` | The key to remove |

**Returns:** `T?` - The removed value, or `null` if the key did not exist

**Throws:** `UnhashableType` if the key is not a hashable type

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87, "charlie": 92}

// Remove and get the value
let removed = scores.remove("bob")
println(removed)   // 87
println(scores)    // {"alice": 95, "charlie": 92}

// Remove non-existent key
let nothing = scores.remove("dave")
println(nothing)   // null

// Conditional removal
if scores.has("alice") {
    let old = scores.remove("alice")
    println("Removed Alice with score: " + str(old))
}
```

---

## Iteration Methods

### `.keys()`

Returns all keys in the map as a list.

**Returns:** `List` - A list containing all keys in the map

**Note:** The order of keys is not guaranteed.

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87, "charlie": 92}

let all_keys = scores.keys()
println(all_keys)  // ["alice", "bob", "charlie"] (order may vary)

// Iterate over keys
for key in scores.keys() {
    println(key + ": " + str(scores[key]))
}

// Check if specific keys exist
let required = ["alice", "bob"]
for name in required {
    if !scores.keys().contains(name) {
        println("Missing: " + name)
    }
}
```

---

### `.values()`

Returns all values in the map as a list.

**Returns:** `List` - A list containing all values in the map

**Note:** The order of values is not guaranteed.

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87, "charlie": 92}

let all_values = scores.values()
println(all_values)  // [95, 87, 92] (order may vary)

// Calculate statistics
let total = all_values.reduce(|acc: Int, v: Int| -> Int { acc + v }, 0)
let average = float(total) / float(all_values.len())
println("Average score: " + str(average))

// Find max value
let max_score = all_values.reduce(|acc: Int, v: Int| -> Int {
    if v > acc { v } else { acc }
})
println("Highest score: " + str(max_score))
```

---

### `.entries()`

Returns all key-value pairs as a list of two-element lists.

**Returns:** `List[List]` - A list where each element is `[key, value]`

**Note:** The order of entries is not guaranteed.

**Example:**

```stratum
let scores = {"alice": 95, "bob": 87}

let all_entries = scores.entries()
println(all_entries)  // [["alice", 95], ["bob", 87]] (order may vary)

// Iterate over entries
for entry in scores.entries() {
    let key = entry[0]
    let value = entry[1]
    println(key + " scored " + str(value))
}

// Transform entries
let labels = scores.entries().map(|e| {
    e[0] + ": " + str(e[1])
})
println(labels.join(", "))  // "alice: 95, bob: 87"

// Filter entries by value
let passing = scores.entries().filter(|e| { e[1] >= 90 })
println(passing)  // [["alice", 95]]
```

---

## Iteration

Maps can be iterated using their `keys()`, `values()`, or `entries()` methods:

```stratum
let config = {"host": "localhost", "port": "8080", "debug": "true"}

// Iterate over keys
for key in config.keys() {
    println("Key: " + key)
}

// Iterate over values
for value in config.values() {
    println("Value: " + value)
}

// Iterate over entries (most common pattern)
for entry in config.entries() {
    println(entry[0] + " = " + entry[1])
}
```

---

## Common Patterns

### Counting occurrences

```stratum
let words = ["apple", "banana", "apple", "cherry", "banana", "apple"]
let counts = {}

for word in words {
    let current = counts.get(word, 0)
    counts.set(word, current + 1)
}
println(counts)  // {"apple": 3, "banana": 2, "cherry": 1}
```

### Grouping data

```stratum
let users = [
    {name: "Alice", dept: "Engineering"},
    {name: "Bob", dept: "Sales"},
    {name: "Charlie", dept: "Engineering"}
]

let by_dept = {}
for user in users {
    let dept = user.dept
    if !by_dept.has(dept) {
        by_dept.set(dept, [])
    }
    by_dept[dept].push(user.name)
}
println(by_dept)  // {"Engineering": ["Alice", "Charlie"], "Sales": ["Bob"]}
```

### Configuration with defaults

```stratum
let defaults = {"timeout": 30, "retries": 3, "debug": false}
let user_config = {"timeout": 60, "debug": true}

// Merge with defaults
let config = {}
for entry in defaults.entries() {
    config.set(entry[0], entry[1])
}
for entry in user_config.entries() {
    config.set(entry[0], entry[1])
}
println(config)  // {"timeout": 60, "retries": 3, "debug": true}
```

### Inverting a map

```stratum
let codes = {"US": "United States", "UK": "United Kingdom", "CA": "Canada"}

let inverted = {}
for entry in codes.entries() {
    inverted.set(entry[1], entry[0])
}
println(inverted["Canada"])  // "CA"
```

### Caching results

```stratum
let cache = {}

fx expensive_compute(n: Int) -> Int {
    // Check cache first
    if cache.has(n) {
        return cache[n]
    }

    // Compute and cache
    let result = n * n * n  // Expensive operation
    cache.set(n, result)
    return result
}
```

---

## See Also

- [Global Functions](globals.md) - `len()` for getting map length
- [List](list.md) - Ordered collection type
- [Json](json.md) - Encoding/decoding maps to/from JSON
