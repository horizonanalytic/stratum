# String

String manipulation methods available on all string values.

## Overview

Strings in Stratum are immutable sequences of UTF-8 characters. All string methods return new strings rather than modifying the original. Strings are also iterable - you can use them directly in `for` loops to iterate over characters.

String methods are called on string values using dot notation: `"hello".to_upper()`.

---

## Properties

### `.len()` / `.length()`

Returns the number of bytes in the string.

**Returns:** `Int` - The byte length of the string

**Note:** For ASCII strings, this equals the character count. For strings with multi-byte UTF-8 characters, use `.chars().len()` for the character count.

**Example:**

```stratum
"hello".len()       // 5
"".len()            // 0
"hello world".len() // 11
```

---

### `.is_empty()`

Checks if the string has zero length.

**Returns:** `Bool` - `true` if the string is empty, `false` otherwise

**Example:**

```stratum
"".is_empty()      // true
"hello".is_empty() // false
" ".is_empty()     // false (contains a space)
```

---

## Character Access

### `.chars()`

Returns a list of individual characters as single-character strings.

**Returns:** `List[String]` - A list where each element is a single character

**Example:**

```stratum
"hello".chars()    // ["h", "e", "l", "l", "o"]
"abc".chars()      // ["a", "b", "c"]
"".chars()         // []

// Useful for character iteration
for char in "hello".chars() {
    println(char)
}

// Or iterate directly over the string
for char in "hello" {
    println(char)
}
```

---

### `.substring(start)` / `.substring(start, end)`

Extracts a portion of the string from `start` to `end` (exclusive).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `Int` | Starting index (0-based). Negative values count from end. |
| `end` | `Int?` | Ending index (exclusive). Defaults to end of string. Negative values count from end. |

**Returns:** `String` - The extracted substring

**Example:**

```stratum
"hello world".substring(0, 5)   // "hello"
"hello world".substring(6, 11)  // "world"
"hello world".substring(6)      // "world" (to end)
"hello world".substring(-5)     // "world" (last 5 chars)
"hello world".substring(0, -6)  // "hello" (up to 6 from end)
```

---

## Search Methods

### `.contains(substring)`

Checks if the string contains a given substring.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `substring` | `String` | The substring to search for |

**Returns:** `Bool` - `true` if the substring is found, `false` otherwise

**Example:**

```stratum
"hello world".contains("world")  // true
"hello world".contains("xyz")    // false
"hello".contains("")             // true (empty string is always found)
"hello".contains("HELLO")        // false (case-sensitive)
```

---

### `.starts_with(prefix)`

Checks if the string starts with a given prefix.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `prefix` | `String` | The prefix to check for |

**Returns:** `Bool` - `true` if the string starts with the prefix, `false` otherwise

**Example:**

```stratum
"hello world".starts_with("hello")  // true
"hello world".starts_with("world")  // false
"hello".starts_with("")             // true
"hello".starts_with("Hello")        // false (case-sensitive)
```

---

### `.ends_with(suffix)`

Checks if the string ends with a given suffix.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `suffix` | `String` | The suffix to check for |

**Returns:** `Bool` - `true` if the string ends with the suffix, `false` otherwise

**Example:**

```stratum
"hello world".ends_with("world")  // true
"hello world".ends_with("hello")  // false
"hello.txt".ends_with(".txt")     // true
"hello".ends_with("")             // true
```

---

## Transformation Methods

### `.to_upper()` / `.to_uppercase()`

Converts all characters to uppercase.

**Returns:** `String` - A new string with all characters in uppercase

**Example:**

```stratum
"hello".to_upper()       // "HELLO"
"Hello World".to_upper() // "HELLO WORLD"
"123abc".to_upper()      // "123ABC"
```

---

### `.to_lower()` / `.to_lowercase()`

Converts all characters to lowercase.

**Returns:** `String` - A new string with all characters in lowercase

**Example:**

```stratum
"HELLO".to_lower()       // "hello"
"Hello World".to_lower() // "hello world"
"123ABC".to_lower()      // "123abc"
```

---

### `.trim()`

Removes leading and trailing whitespace from the string.

**Returns:** `String` - A new string with whitespace removed from both ends

**Example:**

```stratum
"  hello  ".trim()       // "hello"
"\t hello \n".trim()     // "hello"
"hello".trim()           // "hello" (no change)
"   ".trim()             // ""
```

---

### `.trim_start()` / `.ltrim()`

Removes leading whitespace from the string.

**Returns:** `String` - A new string with whitespace removed from the beginning

**Example:**

```stratum
"  hello  ".trim_start()  // "hello  "
"\t hello".trim_start()   // "hello"
"hello".trim_start()      // "hello" (no change)
```

---

### `.trim_end()` / `.rtrim()`

Removes trailing whitespace from the string.

**Returns:** `String` - A new string with whitespace removed from the end

**Example:**

```stratum
"  hello  ".trim_end()   // "  hello"
"hello \n".trim_end()    // "hello"
"hello".trim_end()       // "hello" (no change)
```

---

### `.replace(from, to)`

Replaces all occurrences of a substring with another string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `from` | `String` | The substring to find and replace |
| `to` | `String` | The replacement string |

**Returns:** `String` - A new string with all replacements made

**Example:**

```stratum
"hello world".replace("world", "there")  // "hello there"
"aaa".replace("a", "b")                  // "bbb"
"hello".replace("x", "y")                // "hello" (no match)
"hello".replace("l", "")                 // "heo" (delete matches)
```

---

### `.split(delimiter)`

Splits the string into a list of substrings using a delimiter.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `delimiter` | `String` | The string to split on |

**Returns:** `List[String]` - A list of substrings

**Example:**

```stratum
"a,b,c".split(",")           // ["a", "b", "c"]
"hello world".split(" ")     // ["hello", "world"]
"one::two::three".split("::") // ["one", "two", "three"]
"hello".split(",")           // ["hello"] (no delimiter found)
"a,,b".split(",")            // ["a", "", "b"] (empty strings preserved)
```

---

## See Also

- [Global Functions](globals.md) - `str()` for converting values to strings
- [Regex](regex.md) - Pattern-based string matching and manipulation
- [List](list.md) - List methods for working with `.split()` results
