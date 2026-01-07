# Regex

Regular expression pattern matching and text manipulation.

## Overview

The Regex namespace provides powerful pattern matching capabilities using regular expressions. It supports matching, searching, replacing, and splitting text based on regex patterns.

All functions accept either a pre-compiled `Regex` object (created with `Regex.new()`) or a pattern string. Pre-compiling patterns with `Regex.new()` is recommended when using the same pattern multiple times for better performance.

Stratum uses Rust's `regex` crate syntax, which is similar to Perl-compatible regular expressions (PCRE) but with some differences. Key features include:
- Standard character classes: `\d`, `\w`, `\s`, etc.
- Quantifiers: `*`, `+`, `?`, `{n}`, `{n,m}`
- Groups and captures: `(...)`, `(?:...)`
- Anchors: `^`, `$`, `\b`
- Alternation: `|`

---

## Functions

### `Regex.new(pattern)` / `Regex.new(pattern, options)`

Compiles a regex pattern for reuse. Pre-compiling is recommended when using the same pattern multiple times.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String` | A valid regex pattern |
| `options` | `Map?` | Optional configuration options |

**Options Map:**

| Key | Type | Description |
|-----|------|-------------|
| `case_insensitive` | `Bool` | Case-insensitive matching |
| `multiline` | `Bool` | `^` and `$` match line boundaries |
| `dot_matches_newline` | `Bool` | `.` matches newline characters |

**Returns:** `Regex` - A compiled regex object

**Throws:** Error if the pattern is invalid

**Example:**

```stratum
// Basic compilation
let digits = Regex.new(r"\d+")

// With options
let pattern = Regex.new(r"hello", {case_insensitive: true})

// Reuse compiled regex
Regex.is_match(digits, "abc 123")  // true
Regex.find(digits, "price: $42")   // {text: "42", start: 8, end: 10}
```

---

### `Regex.is_match(pattern, text)` / `Regex.is_match(regex, text)` / `Regex.is_match(pattern, options, text)`

Tests whether the pattern matches anywhere in the text.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |

**Returns:** `Bool` - `true` if the pattern matches, `false` otherwise

**Example:**

```stratum
// Using pattern string
Regex.is_match(r"\d+", "hello 123")         // true
Regex.is_match(r"\d+", "no numbers here")   // false

// Using compiled regex
let email_pattern = Regex.new(r"\w+@\w+\.\w+")
Regex.is_match(email_pattern, "contact: user@example.com")  // true

// With options
Regex.is_match(r"hello", {case_insensitive: true}, "HELLO WORLD")  // true
```

---

### `Regex.find(pattern, text)` / `Regex.find(regex, text)` / `Regex.find(pattern, options, text)`

Finds the first match in the text.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |

**Returns:** `Map | Null` - Match details or `null` if no match

**Match Map Properties:**

| Property | Type | Description |
|----------|------|-------------|
| `text` | `String` | The matched text |
| `start` | `Int` | Starting byte position (0-indexed) |
| `end` | `Int` | Ending byte position (exclusive) |

**Example:**

```stratum
// Find first number
let match = Regex.find(r"\d+", "price: $42.99")
println(match.text)   // "42"
println(match.start)  // 8
println(match.end)    // 10

// No match returns null
let result = Regex.find(r"\d+", "no numbers")
println(result)  // null

// Find word
let word = Regex.find(r"\b\w+\b", "hello world")
println(word.text)  // "hello"
```

---

### `Regex.find_all(pattern, text)` / `Regex.find_all(regex, text)` / `Regex.find_all(pattern, options, text)`

Finds all matches in the text.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |

**Returns:** `List[Map]` - List of match maps (same format as `find`), empty list if no matches

**Example:**

```stratum
// Find all numbers
let matches = Regex.find_all(r"\d+", "a1 b22 c333")
for m in matches {
    println(m.text)  // "1", "22", "333"
}
println(len(matches))  // 3

// Find all words
let words = Regex.find_all(r"\b[a-z]+\b", {case_insensitive: true}, "Hello World")
// [{text: "Hello", ...}, {text: "World", ...}]

// No matches returns empty list
let empty = Regex.find_all(r"\d+", "no numbers")
println(len(empty))  // 0
```

---

### `Regex.replace(pattern, text, replacement)` / `Regex.replace(regex, text, replacement)` / `Regex.replace(pattern, options, text, replacement)`

Replaces the first match with the replacement string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |
| `replacement` | `String` | The replacement string |

**Replacement Syntax:**

| Syntax | Description |
|--------|-------------|
| `$0` | The entire match |
| `$1`, `$2`, ... | Capture group by number |
| `$$` | Literal `$` character |

**Returns:** `String` - The text with the first match replaced

**Example:**

```stratum
// Simple replacement
Regex.replace(r"\d+", "version 1.2.3", "X")  // "version X.2.3"

// Using capture groups
Regex.replace(r"(\w+), (\w+)", "Doe, John", "$2 $1")  // "John Doe"

// Swap date format
Regex.replace(r"(\d{4})-(\d{2})-(\d{2})", "2024-03-15", "$2/$3/$1")  // "03/15/2024"

// No match - returns original
Regex.replace(r"\d+", "no numbers", "X")  // "no numbers"
```

---

### `Regex.replace_all(pattern, text, replacement)` / `Regex.replace_all(regex, text, replacement)` / `Regex.replace_all(pattern, options, text, replacement)`

Replaces all matches with the replacement string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |
| `replacement` | `String` | The replacement string |

**Returns:** `String` - The text with all matches replaced

**Example:**

```stratum
// Replace all digits with X
Regex.replace_all(r"\d", "a1b2c3", "X")  // "aXbXcX"

// Remove all whitespace
Regex.replace_all(r"\s+", "hello   world", " ")  // "hello world"

// Wrap all words in brackets
Regex.replace_all(r"\b(\w+)\b", "hello world", "[$1]")  // "[hello] [world]"

// Case-insensitive replace all
Regex.replace_all(r"cat", {case_insensitive: true}, "Cat CAT cat", "dog")  // "dog dog dog"
```

---

### `Regex.split(pattern, text)` / `Regex.split(regex, text)` / `Regex.split(pattern, options, text)`

Splits the text by the pattern.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to split |

**Returns:** `List[String]` - List of substrings between matches

**Example:**

```stratum
// Split by whitespace
Regex.split(r"\s+", "hello   world  foo")  // ["hello", "world", "foo"]

// Split by comma with optional spaces
Regex.split(r"\s*,\s*", "a, b,  c")  // ["a", "b", "c"]

// Split on multiple delimiters
Regex.split(r"[,;:]", "a,b;c:d")  // ["a", "b", "c", "d"]

// Split camelCase words
Regex.split(r"(?=[A-Z])", "camelCaseWord")  // ["camel", "Case", "Word"]

// No match returns single-element list
Regex.split(r",", "no commas")  // ["no commas"]
```

---

### `Regex.captures(pattern, text)` / `Regex.captures(regex, text)` / `Regex.captures(pattern, options, text)`

Extracts capture groups from the first match.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String \| Regex` | Pattern string or compiled regex |
| `options` | `Map?` | Options (only when using pattern string) |
| `text` | `String` | The text to search |

**Returns:** `List[String | Null] | Null` - List of captures or `null` if no match

**List Contents:**

| Index | Description |
|-------|-------------|
| `0` | The entire match |
| `1`, `2`, ... | Capture groups (or `null` if group didn't participate) |

**Example:**

```stratum
// Extract email parts
let caps = Regex.captures(r"(\w+)@(\w+)\.(\w+)", "contact: user@example.com")
println(caps[0])  // "user@example.com" (full match)
println(caps[1])  // "user"
println(caps[2])  // "example"
println(caps[3])  // "com"

// Parse URL components
let url = "https://example.com:8080/path"
let parts = Regex.captures(r"(\w+)://([^:/]+)(?::(\d+))?(/.*)?", url)
println(parts[1])  // "https"
println(parts[2])  // "example.com"
println(parts[3])  // "8080"
println(parts[4])  // "/path"

// Optional groups may be null
let caps2 = Regex.captures(r"(\d+)(?:-(\d+))?", "42")
println(caps2[0])  // "42"
println(caps2[1])  // "42"
println(caps2[2])  // null (optional group didn't match)

// No match returns null
let none = Regex.captures(r"(\d+)", "no numbers")
println(none)  // null
```

---

## Common Patterns

Here are some useful regex patterns for common tasks:

```stratum
// Email validation (simple)
let email = r"^[\w.+-]+@[\w-]+\.[\w.-]+$"

// URL matching
let url = r"https?://[^\s]+"

// Phone number (US format)
let phone = r"\d{3}[-.]?\d{3}[-.]?\d{4}"

// Date (YYYY-MM-DD)
let date = r"\d{4}-\d{2}-\d{2}"

// IPv4 address
let ipv4 = r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}"

// Hex color code
let hex_color = r"#[0-9A-Fa-f]{6}\b"

// Whitespace trimming
let leading_ws = r"^\s+"
let trailing_ws = r"\s+$"
```

---

## See Also

- [String](string.md) - String methods including basic `replace()` and `split()`
- [Global Functions](globals.md) - `str()` and other conversion functions
