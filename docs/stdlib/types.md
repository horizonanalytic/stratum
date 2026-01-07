# Type Reference

Complete reference for all types in Stratum.

## Overview

Stratum is a statically typed language with full type inference. You rarely need to write type annotations—the compiler infers types from context. When you do need explicit types, Stratum uses a clean, readable syntax.

**Key characteristics:**
- Static typing with inference: Types are checked at compile time, but rarely need to be written
- Nullable types: Use `?` suffix (e.g., `String?`) to allow `null`
- No null by default: Regular types cannot be `null`
- Reference semantics: Lists, maps, and structs are passed by reference

---

## Primitive Types

### `Int`

64-bit signed integer. Supports values from -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807.

**Literals:**

```stratum
let decimal = 42
let negative = -17
let with_separators = 1_000_000  // Underscores for readability
let hex = 0xFF                    // 255
let binary = 0b1010               // 10
let octal = 0o17                  // 15
```

**Operations:**

```stratum
// Arithmetic
5 + 3      // 8
10 - 4     // 6
6 * 7      // 42
15 / 4     // 3 (integer division, truncates toward zero)
17 % 5     // 2 (remainder)

// Comparison
5 == 5     // true
3 != 4     // true
3 < 5      // true
5 <= 5     // true
7 > 3      // true
7 >= 7     // true

// Bitwise
5 & 3      // 1 (AND)
5 | 3      // 7 (OR)
5 ^ 3      // 6 (XOR)
~5         // -6 (NOT)
8 << 2     // 32 (left shift)
8 >> 2     // 2 (right shift)
```

**Conversion:**

```stratum
int(3.7)       // 3 (truncates toward zero)
int(-3.7)      // -3
int("42")      // 42
int("0xFF")    // 255 (hex parsing)
int(true)      // 1
int(false)     // 0
```

---

### `Float`

64-bit IEEE 754 floating-point number (double precision).

**Literals:**

```stratum
let pi = 3.14159
let negative = -0.5
let scientific = 1.5e10      // 15,000,000,000
let neg_exp = 2.5e-3         // 0.0025
let with_separator = 1_000.5 // 1000.5
```

**Special values:**

```stratum
Math.INFINITY       // Positive infinity
Math.NEG_INFINITY   // Negative infinity
Math.NAN            // Not a Number

// Check for special values
Math.is_nan(0.0 / 0.0)        // true
Math.is_infinite(1.0 / 0.0)   // true
Math.is_finite(3.14)          // true
```

**Operations:**

```stratum
// Arithmetic (same operators as Int)
3.5 + 2.1    // 5.6
10.0 / 4.0   // 2.5 (true division)
10.0 % 3.0   // 1.0 (floating-point remainder)

// Mixed operations promote Int to Float
5 + 2.5      // 7.5 (result is Float)
10 / 4.0     // 2.5 (result is Float)
```

**Conversion:**

```stratum
float(42)          // 42.0
float("3.14")      // 3.14
float("-1.5e2")    // -150.0
float(true)        // 1.0
float(false)       // 0.0
```

---

### `Bool`

Boolean type with two values: `true` and `false`.

**Literals:**

```stratum
let yes = true
let no = false
```

**Operations:**

```stratum
// Logical operators
true && false    // false (AND)
true || false    // true (OR)
!true            // false (NOT)

// Short-circuit evaluation
false && expensive()   // expensive() not called
true || expensive()    // expensive() not called

// Comparison (all types support == and !=)
true == true     // true
true != false    // true
```

**Truthiness:**

In conditional contexts, only `false` and `null` are falsy. All other values are truthy:

```stratum
if 0 { println("zero is truthy") }        // Prints
if "" { println("empty string is truthy") } // Prints
if [] { println("empty list is truthy") }   // Prints

if null { println("null is falsy") }      // Does NOT print
if false { println("false is falsy") }    // Does NOT print
```

**Conversion:**

```stratum
// No implicit bool conversion - use explicit comparison
if list.len() > 0 { ... }   // Correct
// if list.len() { ... }    // Works but prefer explicit comparison
```

---

### `String`

UTF-8 encoded immutable text. See [String](string.md) for full method documentation.

**Literals:**

```stratum
let simple = "Hello, World!"
let with_escapes = "Line 1\nLine 2\tTabbed"
let raw = r"No \escapes\ here"
let multiline = "First line
Second line
Third line"

// String interpolation
let name = "Alice"
let greeting = "Hello, ${name}!"   // "Hello, Alice!"
let math = "2 + 2 = ${2 + 2}"      // "2 + 2 = 4"
```

**Escape sequences:**

| Escape | Meaning |
|--------|---------|
| `\n` | Newline |
| `\t` | Tab |
| `\r` | Carriage return |
| `\\` | Backslash |
| `\"` | Double quote |
| `\0` | Null character |
| `\xHH` | Hex byte (e.g., `\x1B` for escape) |
| `\u{HHHH}` | Unicode code point |

**Operations:**

```stratum
// Concatenation
"Hello, " + "World!"    // "Hello, World!"

// Comparison (lexicographic)
"abc" < "abd"           // true
"abc" == "abc"          // true

// Indexing (returns single-character string)
"hello"[0]              // "h"
"hello"[-1]             // "o" (last character)

// Length
"hello".len()           // 5
```

**Common methods:**

```stratum
"HELLO".to_lower()            // "hello"
"hello".to_upper()            // "HELLO"
"  hello  ".trim()            // "hello"
"hello world".split(" ")      // ["hello", "world"]
"hello".contains("ell")       // true
"hello".replace("l", "L")     // "heLLo"
```

---

### `Null`

The absence of a value. Only allowed in nullable types (`T?`).

**Literal:**

```stratum
let nothing = null
```

**Nullable types:**

```stratum
// Regular type - cannot be null
let name: String = "Alice"
// name = null  // ERROR: cannot assign null to String

// Nullable type - can be null
let maybe_name: String? = "Alice"
maybe_name = null  // OK

// Type inference for nullable
let x = null           // Type is Null
let y: Int? = null     // Type is Int?
let z: Int? = 42       // Type is Int?, value is 42
```

**Null handling:**

```stratum
// Null check
let name: String? = get_name()
if name != null {
    println("Hello, " + name)  // Safe: name is String here
}

// Null coalescing operator
let display = name ?? "Anonymous"  // Use "Anonymous" if null

// Optional chaining (if supported)
let length = name?.len() ?? 0
```

**Comparison:**

```stratum
null == null    // true
null != null    // false
null == 0       // false
null == ""      // false
null == false   // false
```

---

## Collection Types

### `List<T>`

Ordered, mutable collection of elements. See [List](list.md) for full method documentation.

**Type syntax:**

```stratum
// Type annotations (usually inferred)
let numbers: List<Int> = [1, 2, 3]
let names: List<String> = ["Alice", "Bob"]
let nested: List<List<Int>> = [[1, 2], [3, 4]]
let mixed: List<Int | String> = [1, "two", 3]  // Union type elements
```

**Creating lists:**

```stratum
// Literal syntax
let empty = []
let numbers = [1, 2, 3, 4, 5]
let mixed = [1, "hello", true, null]

// From range
let digits = range(0, 10)  // Creates a Range, not a List
```

**Key characteristics:**

| Property | Description |
|----------|-------------|
| Ordered | Elements maintain insertion order |
| Mutable | `push()`, `pop()`, `reverse()` modify in-place |
| Zero-indexed | First element is at index 0 |
| Negative indexing | `-1` is last element, `-2` is second-to-last |
| Reference semantics | Assigning creates a reference, not a copy |

**Common operations:**

```stratum
let list = [1, 2, 3]

// Access
list[0]        // 1
list[-1]       // 3 (last element)
list.first()   // 1
list.last()    // 3

// Mutation
list.push(4)   // list is now [1, 2, 3, 4]
list.pop()     // returns 4, list is [1, 2, 3]

// Query
list.len()         // 3
list.contains(2)   // true
list.is_empty()    // false

// Transform (return new lists)
list.map(|x| { x * 2 })      // [2, 4, 6]
list.filter(|x| { x > 1 })   // [2, 3]
list.sort()                   // [1, 2, 3] (new list)
```

---

### `Map<K, V>`

Mutable key-value collection backed by a hash table. See [Map](map.md) for full method documentation.

**Type syntax:**

```stratum
// Type annotations (usually inferred)
let scores: Map<String, Int> = {"alice": 95, "bob": 87}
let config: Map<String, String | Int | Bool> = {
    "host": "localhost",
    "port": 8080,
    "debug": true
}
```

**Creating maps:**

```stratum
// Literal syntax
let empty = {}
let scores = {"alice": 95, "bob": 87}
let nested = {
    "user": {"name": "Alice", "age": 30},
    "settings": {"theme": "dark"}
}
```

**Key type restrictions:**

Maps only accept hashable types as keys:

| Allowed | Examples |
|---------|----------|
| `Null` | `{null: "value"}` |
| `Bool` | `{true: 1, false: 0}` |
| `Int` | `{42: "answer", -1: "negative"}` |
| `String` | `{"name": "Alice"}` |

Non-hashable types (List, Map, Struct) cannot be used as keys.

**Key characteristics:**

| Property | Description |
|----------|-------------|
| Unordered | Iteration order not guaranteed |
| Mutable | `set()`, `remove()` modify in-place |
| Unique keys | Setting an existing key updates its value |
| Null for missing | Accessing missing key returns `null` |
| Reference semantics | Assigning creates a reference |

**Common operations:**

```stratum
let map = {"a": 1, "b": 2}

// Access
map["a"]           // 1
map["z"]           // null (missing key)
map.get("a")       // 1
map.get("z", 0)    // 0 (with default)

// Mutation
map.set("c", 3)    // {"a": 1, "b": 2, "c": 3}
map["d"] = 4       // {"a": 1, "b": 2, "c": 3, "d": 4}
map.remove("a")    // returns 1, key removed

// Query
map.len()              // 3
map.contains_key("b")  // true
map.has("b")           // true (alias)

// Iteration
map.keys()      // ["b", "c", "d"] (order varies)
map.values()    // [2, 3, 4] (order varies)
map.entries()   // [["b", 2], ["c", 3], ["d", 4]]
```

---

### Set (via Map)

Stratum doesn't have a built-in Set type, but you can use a Map with dummy values:

```stratum
// Create a set using Map
let set = {}

// Add elements
set.set("apple", true)
set.set("banana", true)
set.set("cherry", true)

// Check membership
set.has("apple")    // true
set.has("grape")    // false

// Remove element
set.remove("banana")

// Get all elements
let elements = set.keys()  // ["apple", "cherry"]

// Set size
set.len()  // 2
```

**Set operations pattern:**

```stratum
// Union
fx set_union(a: Map, b: Map) -> Map {
    let result = {}
    for key in a.keys() { result.set(key, true) }
    for key in b.keys() { result.set(key, true) }
    return result
}

// Intersection
fx set_intersection(a: Map, b: Map) -> Map {
    let result = {}
    for key in a.keys() {
        if b.has(key) { result.set(key, true) }
    }
    return result
}

// Difference
fx set_difference(a: Map, b: Map) -> Map {
    let result = {}
    for key in a.keys() {
        if !b.has(key) { result.set(key, true) }
    }
    return result
}
```

---

## Special Types

### `Range`

Represents a sequence of integers from a start to an end value.

**Creating ranges:**

```stratum
// Using the global range() function
let exclusive = range(0, 5)     // 0, 1, 2, 3, 4 (excludes 5)
let inclusive = range(1, 10)    // 1 through 9

// Range properties
exclusive.start       // 0
exclusive.end         // 5
exclusive.inclusive   // false (by default)
```

**Key characteristics:**

| Property | Description |
|----------|-------------|
| Lazy | Does not allocate a list; generates values on demand |
| Immutable | Cannot be modified after creation |
| Iterable | Can be used in `for` loops |
| Integer only | Only works with integer bounds |

**Common uses:**

```stratum
// Loop a fixed number of times
for i in range(0, 5) {
    println(i)  // 0, 1, 2, 3, 4
}

// Loop with step (using filter)
for i in range(0, 10) {
    if i % 2 == 0 {
        println(i)  // 0, 2, 4, 6, 8
    }
}

// Countdown (negative ranges)
for i in range(5, 0) {
    println(i)  // (empty - start must be less than end)
}

// Index iteration
let list = ["a", "b", "c"]
for i in range(0, list.len()) {
    println(str(i) + ": " + list[i])
}
```

**Checking containment:**

```stratum
let r = range(1, 10)
r.contains(5)    // true
r.contains(10)   // false (exclusive end)
r.contains(0)    // false (before start)
```

---

### `Future<T>`

Represents an asynchronous computation that will eventually produce a value of type `T` or fail with an error.

**Key characteristics:**

| Property | Description |
|----------|-------------|
| Lazy | Computation may not start until awaited |
| Single value | Produces exactly one value or error |
| Non-blocking | Does not block the calling thread |
| Composable | Can be chained with other futures |

**Creating futures:**

```stratum
// Async functions return futures
async fx fetch_data(url: String) -> String {
    let response = await Http.get(url)
    return response.body
}

// Call returns Future<String>, not String
let future = fetch_data("https://api.example.com/data")

// Immediately resolved future
let ready = Async.ready(42)        // Future<Int> already resolved to 42

// Immediately failed future
let failed = Async.failed("error") // Future that will fail
```

**Awaiting futures:**

```stratum
// Use await to get the value (must be in async context)
async fx main() {
    let data = await fetch_data("https://api.example.com")
    println(data)
}

// Await with timeout
let result = await Async.timeout(future, Duration.seconds(30))
```

**Future states:**

```stratum
let future = some_async_operation()

future.is_pending()  // true if not yet complete
future.is_ready()    // true if completed successfully
future.kind()        // "pending", "ready", or "failed"
```

**Concurrent execution:**

```stratum
// Run multiple operations concurrently
async fx fetch_all() {
    // Start all requests concurrently
    let future1 = Http.get("https://api1.example.com")
    let future2 = Http.get("https://api2.example.com")
    let future3 = Http.get("https://api3.example.com")

    // Wait for all to complete
    let r1 = await future1
    let r2 = await future2
    let r3 = await future3

    return [r1.body, r2.body, r3.body]
}
```

---

### `Result<T, E>`

Result is not a built-in primitive type in Stratum. Instead, it's implemented as a generic enum that you can define or import from a library:

```stratum
// Define Result as a generic enum
enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

**Creating results:**

```stratum
// Success case
let success: Result<Int, String> = Result.Ok(42)

// Error case
let failure: Result<Int, String> = Result.Err("something went wrong")
```

**Pattern matching:**

```stratum
fx process(result: Result<Int, String>) {
    match result {
        Result.Ok(value) => {
            println("Got value: " + str(value))
        }
        Result.Err(error) => {
            println("Error: " + error)
        }
    }
}
```

**Common Result patterns:**

```stratum
// Function that returns Result
fx divide(a: Int, b: Int) -> Result<Int, String> {
    if b == 0 {
        return Result.Err("division by zero")
    }
    return Result.Ok(a / b)
}

// Using the result
let result = divide(10, 2)
match result {
    Result.Ok(value) => println("Result: " + str(value))
    Result.Err(msg) => println("Error: " + msg)
}
```

**Note:** Stratum primarily uses exceptions for error handling. Use Result when you want explicit error handling in the type system or when working with operations that commonly fail.

---

## User-Defined Types

### `struct`

Structs are custom data types with named fields. All fields are public by default.

**Defining structs:**

```stratum
struct User {
    name: String,
    email: String,
    age: Int
}

// With optional fields
struct Config {
    host: String,
    port: Int,
    timeout: Int?,  // Optional (can be null)
    debug: Bool
}

// Generic struct
struct Pair<T, U> {
    first: T,
    second: U
}
```

**Creating instances:**

```stratum
// All fields required (unless nullable)
let user = User {
    name: "Alice",
    email: "alice@example.com",
    age: 30
}

// Field order doesn't matter
let config = Config {
    debug: true,
    port: 8080,
    host: "localhost",
    timeout: null
}

// Generic instantiation (type inferred)
let pair = Pair { first: 1, second: "one" }  // Pair<Int, String>
```

**Accessing fields:**

```stratum
user.name      // "Alice"
user.age       // 30

// Modify fields (structs are mutable by default)
user.age = 31
user.email = "alice@newdomain.com"
```

**Struct methods:**

```stratum
struct Rectangle {
    width: Float,
    height: Float
}

impl Rectangle {
    fx area(self) -> Float {
        return self.width * self.height
    }

    fx perimeter(self) -> Float {
        return 2.0 * (self.width + self.height)
    }

    fx scale(self, factor: Float) {
        self.width = self.width * factor
        self.height = self.height * factor
    }
}

let rect = Rectangle { width: 10.0, height: 5.0 }
rect.area()       // 50.0
rect.perimeter()  // 30.0
rect.scale(2.0)   // rect is now 20x10
```

**Reference semantics:**

```stratum
let original = User { name: "Alice", email: "a@b.com", age: 30 }
let reference = original  // Not a copy - same struct

reference.age = 31
println(original.age)  // 31 (original was modified!)
```

---

### `enum`

Enums define a type with a fixed set of variants. Each variant can optionally carry data.

**Simple enums:**

```stratum
enum Color {
    Red,
    Green,
    Blue
}

let color = Color.Red

// Pattern matching
match color {
    Color.Red => println("It's red!")
    Color.Green => println("It's green!")
    Color.Blue => println("It's blue!")
}
```

**Enums with data (tuple-style):**

```stratum
enum Message {
    Quit,
    Move(Int, Int),           // x, y coordinates
    Write(String),
    ChangeColor(Int, Int, Int) // RGB values
}

let msg = Message.Move(10, 20)

match msg {
    Message.Quit => println("Quit")
    Message.Move(x, y) => println("Move to " + str(x) + ", " + str(y))
    Message.Write(text) => println("Write: " + text)
    Message.ChangeColor(r, g, b) => println("Color: RGB(" + str(r) + "," + str(g) + "," + str(b) + ")")
}
```

**Enums with named fields (struct-style):**

```stratum
enum Shape {
    Circle { radius: Float },
    Rectangle { width: Float, height: Float },
    Triangle { base: Float, height: Float }
}

let shape = Shape.Rectangle { width: 10.0, height: 5.0 }

match shape {
    Shape.Circle { radius } => Math.PI * radius * radius
    Shape.Rectangle { width, height } => width * height
    Shape.Triangle { base, height } => 0.5 * base * height
}
```

**Generic enums:**

```stratum
enum Option<T> {
    Some(T),
    None
}

let maybe_value: Option<Int> = Option.Some(42)
let no_value: Option<Int> = Option.None

match maybe_value {
    Option.Some(v) => println("Got: " + str(v))
    Option.None => println("No value")
}
```

**Enum methods:**

```stratum
enum Status {
    Pending,
    Active,
    Completed,
    Failed(String)
}

impl Status {
    fx is_terminal(self) -> Bool {
        match self {
            Status.Completed => true
            Status.Failed(_) => true
            _ => false
        }
    }

    fx description(self) -> String {
        match self {
            Status.Pending => "Waiting to start"
            Status.Active => "Currently running"
            Status.Completed => "Finished successfully"
            Status.Failed(reason) => "Failed: " + reason
        }
    }
}
```

---

### `interface`

Interfaces define a contract that types must implement. They enable polymorphism without inheritance.

**Defining interfaces:**

```stratum
interface Printable {
    fx to_string(self) -> String
}

interface Comparable {
    fx compare(self, other: Self) -> Int
}

// Interface with default implementation
interface Describable {
    fx name(self) -> String

    fx describe(self) -> String {
        return "This is a " + self.name()
    }
}
```

**Implementing interfaces:**

```stratum
struct Person {
    first_name: String,
    last_name: String
}

impl Printable for Person {
    fx to_string(self) -> String {
        return self.first_name + " " + self.last_name
    }
}

impl Describable for Person {
    fx name(self) -> String {
        return "Person"
    }
    // describe() uses default implementation
}

let person = Person { first_name: "Alice", last_name: "Smith" }
person.to_string()  // "Alice Smith"
person.describe()   // "This is a Person"
```

**Interface as parameter type:**

```stratum
// Accept any type implementing Printable
fx print_item(item: Printable) {
    println(item.to_string())
}

print_item(person)  // Works with any Printable
```

**Multiple interfaces:**

```stratum
struct Score {
    value: Int
}

impl Printable for Score {
    fx to_string(self) -> String {
        return str(self.value)
    }
}

impl Comparable for Score {
    fx compare(self, other: Score) -> Int {
        return self.value - other.value
    }
}
```

---

## Type Annotations

While Stratum infers most types, you can add explicit annotations when needed.

**Variable annotations:**

```stratum
let x: Int = 42
let name: String = "Alice"
let scores: List<Int> = [95, 87, 92]
let config: Map<String, Int> = {"port": 8080}
```

**Function annotations:**

```stratum
fx greet(name: String) -> String {
    return "Hello, " + name + "!"
}

fx process(data: List<Int>, threshold: Int) -> List<Int> {
    return data.filter(|x| { x > threshold })
}
```

**Nullable annotations:**

```stratum
fx find_user(id: Int) -> User? {
    // May return null if not found
}

let maybe_user: User? = find_user(123)
```

**Union types:**

```stratum
fx process(value: Int | String) {
    // value can be Int or String
}

let mixed: List<Int | String> = [1, "two", 3, "four"]
```

**Function types:**

```stratum
// Function that takes a transformer function
fx transform(list: List<Int>, fn: (Int) -> Int) -> List<Int> {
    return list.map(fn)
}

// Higher-order function with multiple params
fx reduce(list: List<Int>, fn: (Int, Int) -> Int, init: Int) -> Int {
    // ...
}
```

---

## See Also

- [String](string.md) - String methods
- [List](list.md) - List methods
- [Map](map.md) - Map methods
- [Global Functions](globals.md) - Type conversion functions (`int()`, `str()`, `float()`)
- [Async](async.md) - Async utilities for futures
