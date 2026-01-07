# Global Functions

Built-in functions available without a namespace prefix. These are the core functions used in nearly every Stratum program.

## Overview

Global functions provide fundamental operations like printing output, type conversions, assertions for testing, and collection utilities. They are available everywhere without any import or namespace prefix.

---

## Output Functions

### `print(args...)`

Prints values to standard output without a trailing newline.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `args` | `Any...` | Zero or more values to print |

**Returns:** `Null`

**Example:**

```stratum
print("Hello")
print(" ")
print("World")
// Output: Hello World (no newline at end)

print(1, 2, 3)
// Output: 1 2 3
```

---

### `println(args...)`

Prints values to standard output followed by a newline.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `args` | `Any...` | Zero or more values to print |

**Returns:** `Null`

**Example:**

```stratum
println("Hello, World!")
// Output: Hello, World!

println("Sum:", 1 + 2)
// Output: Sum: 3

println()  // Just prints a newline
```

---

## Type Inspection

### `type_of(value)`

Returns the type name of a value as a string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Any` | The value to inspect |

**Returns:** `String` - The type name

**Example:**

```stratum
type_of(42)           // "Int"
type_of(3.14)         // "Float"
type_of("hello")      // "String"
type_of(true)         // "Bool"
type_of(null)         // "Null"
type_of([1, 2, 3])    // "List"
type_of({"a": 1})     // "Map"
type_of(1..10)        // "Range"
```

---

## Assertions

### `assert(condition)`

Asserts that a condition is truthy. Throws an error if the condition is falsy.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `Any` | Value to test for truthiness |

**Returns:** `Null`

**Throws:** `"assertion failed"` if condition is falsy

**Example:**

```stratum
assert(true)        // OK
assert(1 + 1 == 2)  // OK
assert(false)       // Error: assertion failed

// Truthy values
assert(1)           // OK (non-zero is truthy)
assert("hello")     // OK (non-empty string is truthy)
assert([1, 2, 3])   // OK (non-empty list is truthy)
```

---

### `assert_eq(expected, actual)`

Asserts that two values are equal. Throws a descriptive error if they differ.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `expected` | `Any` | The expected value |
| `actual` | `Any` | The actual value to compare |

**Returns:** `Null`

**Throws:** Formatted error message showing both values if not equal

**Example:**

```stratum
assert_eq(4, 2 + 2)           // OK
assert_eq("hello", "hello")   // OK
assert_eq([1, 2], [1, 2])     // OK

assert_eq(5, 2 + 2)
// Error: assertion failed: 5 != 4
```

---

## Type Conversion

### `str(value)`

Converts any value to its string representation.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Any` | The value to convert |

**Returns:** `String` - String representation of the value

**Example:**

```stratum
str(42)           // "42"
str(3.14)         // "3.14"
str(true)         // "true"
str(null)         // "null"
str([1, 2, 3])    // "[1, 2, 3]"
str({"a": 1})     // "{\"a\": 1}"
```

---

### `int(value)`

Converts a value to an integer.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Int \| Float \| String \| Bool` | The value to convert |

**Returns:** `Int` - The integer value

**Throws:** Error if string cannot be parsed as an integer

**Conversions:**
- `Int` → returns unchanged
- `Float` → truncates toward zero
- `String` → parses as decimal integer
- `Bool` → `true` becomes `1`, `false` becomes `0`

**Example:**

```stratum
int(42)       // 42
int(3.7)      // 3 (truncated)
int(-2.9)     // -2 (truncated toward zero)
int("123")    // 123
int(true)     // 1
int(false)    // 0

int("hello")  // Error: invalid digit found in string
```

---

### `float(value)`

Converts a value to a floating-point number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Float \| Int \| String` | The value to convert |

**Returns:** `Float` - The floating-point value

**Throws:** Error if string cannot be parsed as a float

**Example:**

```stratum
float(42)       // 42.0
float(3.14)     // 3.14
float("3.14")   // 3.14
float("1e10")   // 10000000000.0

float("hello")  // Error: invalid float literal
```

---

## Collection Utilities

### `len(collection)`

Returns the length or size of a collection.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `collection` | `String \| List \| Map` | The collection to measure |

**Returns:** `Int` - The length/size

**Throws:** Error if value type does not have a length

**Example:**

```stratum
len("hello")       // 5 (bytes, not characters)
len([1, 2, 3])     // 3
len({"a": 1, "b": 2})  // 2

len(42)  // Error: Int has no length
```

**Note:** For strings, `len()` returns the byte length, not the character count. For Unicode strings with multi-byte characters, use `str.chars()` to get the character count.

---

### `range(start, end)`

Creates an exclusive range from start to end.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `Int` | The start value (inclusive) |
| `end` | `Int` | The end value (exclusive) |

**Returns:** `Range` - A range object `[start, end)`

**Throws:** Error if arguments are not integers

**Example:**

```stratum
range(0, 5)  // 0..5 (includes 0, 1, 2, 3, 4)

// Use in for loops
for i in range(1, 4) {
    println(i)
}
// Output:
// 1
// 2
// 3

// Alternative syntax (equivalent)
for i in 1..4 {
    println(i)
}
```

**Note:** For an inclusive range that includes the end value, use the `..=` syntax: `1..=5` includes 1, 2, 3, 4, 5.

---

## See Also

- [Math](math.md) - Mathematical functions and constants
- [String methods](types.md#string) - String manipulation methods
- [List methods](types.md#list) - List manipulation methods
- [Map methods](types.md#map) - Map manipulation methods
