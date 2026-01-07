# Standard Library Reference

Stratum's standard library provides a comprehensive set of built-in functions organized into namespaces. All functions are available without imports.

## [Type Reference](types.md)

Complete reference for all types in Stratum.

| Category | Types |
|----------|-------|
| Primitives | `Int`, `Float`, `Bool`, `String`, `Null` |
| Collections | `List<T>`, `Map<K, V>` |
| Special | `Range`, `Future<T>` |
| User-Defined | `struct`, `enum`, `interface` |

---

## [Global Functions](globals.md)

Built-in functions available without a namespace prefix.

| Function | Description |
|----------|-------------|
| [`print(args...)`](globals.md#printargs) | Output values without newline |
| [`println(args...)`](globals.md#printlnargs) | Output values with newline |
| [`assert(condition)`](globals.md#assertcondition) | Assert condition is truthy |
| [`assert_eq(a, b)`](globals.md#assert_eqexpected-actual) | Assert two values are equal |
| [`type_of(value)`](globals.md#type_ofvalue) | Get type name as string |
| [`len(collection)`](globals.md#lencollection) | Get length of string, list, or map |
| [`str(value)`](globals.md#strvalue) | Convert value to string |
| [`int(value)`](globals.md#intvalue) | Convert value to integer |
| [`float(value)`](globals.md#floatvalue) | Convert value to float |
| [`range(start, end)`](globals.md#rangestart-end) | Create an exclusive range `[start, end)` |

---

## Namespaces by Category

### Math & Numbers

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Math](math.md) | Mathematical constants and functions | 45+ |
| [Random](random.md) | Random number generation | 6 |

### Strings & Text

| Type/Namespace | Description | Methods |
|----------------|-------------|---------|
| [String](string.md) | String manipulation methods | 14 |
| [Regex](regex.md) | Regular expression operations | 8 |

### Collections

| Type | Description | Methods |
|------|-------------|---------|
| [List](list.md) | Ordered, mutable collection | 14 |
| [Map](map.md) | Key-value dictionary | 10 |

### Data Encoding

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Json](json.md) | JSON encoding/decoding | 2 |
| [Toml](toml.md) | TOML encoding/decoding | 2 |
| [Yaml](yaml.md) | YAML encoding/decoding | 2 |
| [Base64](base64.md) | Base64 encoding/decoding | 2 |
| [Url](url.md) | URL encoding/decoding | 2 |

### File System

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [File](file.md) | File read/write operations | 11 |
| [Dir](dir.md) | Directory operations | 7 |
| [Path](path.md) | Path manipulation | 11 |
| [Input](input.md) | Console input/prompts | 7 |

### Date & Time

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [DateTime](datetime.md) | Date/time creation and manipulation | 20 |
| [Duration](duration.md) | Duration creation and arithmetic | 10 |
| [Time](time.md) | Timers and sleep | 4 |

### Networking

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Http](http.md) | HTTP client requests | 6 |
| [Tcp](tcp.md) | TCP client/server | 6 |
| [Udp](udp.md) | UDP sockets | 4 |
| [WebSocket](websocket.md) | WebSocket client/server | 6 |

### Security & Hashing

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Hash](hash.md) | Cryptographic hash functions | 8 |
| [Crypto](crypto.md) | Encryption and key derivation | 4 |
| [Uuid](uuid.md) | UUID generation and validation | 4 |

### Compression

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Gzip](gzip.md) | Gzip compression | 4 |
| [Zip](zip.md) | ZIP archive operations | 6 |

### System

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [System](system.md) | System info and control | 9 |
| [Env](env.md) | Environment variables | 5 |
| [Args](args.md) | Command-line arguments | 3 |
| [Shell](shell.md) | Shell command execution | 2 |
| [Log](log.md) | Logging and output control | 10 |

### Data Operations

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Data](data.md) | DataFrame creation and I/O | 12 |
| [Agg](agg.md) | Aggregation functions | 7 |
| [Join](join.md) | DataFrame join operations | 5 |
| [Cube](cube.md) | OLAP cube operations | 7 |

### Async

| Namespace | Description | Functions |
|-----------|-------------|-----------|
| [Async](async.md) | Async utilities | 3 |
| [Db](db.md) | Database connections | 8 |

---

## Quick Reference

### Most Common Functions

```stratum
// Output
println("Hello, World!")
print("No newline")

// Type conversion
let s = str(42)        // "42"
let n = int("42")      // 42
let f = float("3.14")  // 3.14

// Collections
let length = len([1, 2, 3])  // 3

// Math
let pi = Math.PI
let sqrt2 = Math.sqrt(2)
let random = Random.int(1, 100)

// Files
let content = File.read_text("data.txt")
File.write_text("output.txt", "Hello!")

// JSON
let obj = Json.decode('{"name": "Stratum"}')
let json = Json.encode(obj)

// HTTP
let response = Http.get("https://api.example.com/data")
println(response.body)

// DateTime
let now = DateTime.now()
println(DateTime.format(now, "%Y-%m-%d"))
```

### Pipeline Operations

```stratum
// DataFrame with pipeline operator
let result = Data.read_csv("sales.csv")
    |> select("product", "revenue")
    |> group_by("product")
    |> sum("revenue")
    |> sort_by("revenue", "desc")
    |> take(10)
```

---

## Documentation Format

Each namespace page follows a consistent format:

1. **Overview** - Purpose and common use cases
2. **Constants** - Named constants (if any)
3. **Functions** - Complete function reference with:
   - Signature
   - Parameters table
   - Return type
   - Examples
4. **See Also** - Related namespaces

See [_template.md](_template.md) for the documentation template.
