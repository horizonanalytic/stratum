# Url

URL percent-encoding and decoding.

## Overview

The Url namespace provides functions for percent-encoding and decoding strings for use in URLs. Percent-encoding (also called URL encoding) replaces unsafe characters with `%` followed by two hexadecimal digits representing the character's byte value.

This is essential for:
- Encoding query string parameters
- Encoding path segments with special characters
- Safely transmitting data in URLs
- Handling user input in URLs

The encoding uses UTF-8 for multi-byte characters and encodes all non-alphanumeric characters except `-`, `_`, `.`, and `~`.

---

## Functions

### `Url.encode(input)`

Percent-encodes a string for safe use in URLs.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `input` | `String` | The string to encode |

**Returns:** `String` - The percent-encoded string

**Characters Preserved:** `A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~`

**Example:**

```stratum
// Encode spaces and special characters
Url.encode("hello world")           // "hello%20world"
Url.encode("name=value")            // "name%3Dvalue"
Url.encode("a&b")                   // "a%26b"

// Encode query parameters
Url.encode("search term")           // "search%20term"
Url.encode("price > 100")           // "price%20%3E%20100"

// Reserved characters are encoded
Url.encode("foo/bar")               // "foo%2Fbar"
Url.encode("key=val&other=123")     // "key%3Dval%26other%3D123"

// Safe characters are not encoded
Url.encode("hello-world_test.txt")  // "hello-world_test.txt"
Url.encode("file~name")             // "file~name"

// Unicode characters are UTF-8 encoded
Url.encode("cafe")                  // "cafe"
```

---

### `Url.decode(encoded)`

Decodes a percent-encoded URL string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `encoded` | `String` | A percent-encoded string |

**Returns:** `String` - The decoded string

**Throws:** Error if the input contains invalid percent-encoding sequences

**Example:**

```stratum
// Decode percent-encoded strings
Url.decode("hello%20world")           // "hello world"
Url.decode("name%3Dvalue")            // "name=value"
Url.decode("a%26b")                   // "a&b"

// Decode path segments
Url.decode("foo%2Fbar")               // "foo/bar"
Url.decode("my%20file%20name.txt")    // "my file name.txt"

// Already decoded strings pass through
Url.decode("hello-world")             // "hello-world"
Url.decode("simple")                  // "simple"

// Round-trip encoding
let original = "hello world & goodbye"
let encoded = Url.encode(original)     // "hello%20world%20%26%20goodbye"
let decoded = Url.decode(encoded)      // "hello world & goodbye"
assert_eq(original, decoded)
```

---

## Common Patterns

### Building Query Strings

```stratum
// Encode individual query parameters
let name = "John Doe"
let query = "price > 50"

let params = "name=" + Url.encode(name) + "&query=" + Url.encode(query)
// "name=John%20Doe&query=price%20%3E%2050"

let url = "https://api.example.com/search?" + params
```

### Parsing Query Parameters

```stratum
// Decode query parameters from a URL
let query_string = "name=John%20Doe&city=New%20York"

// Split and decode each parameter
let pairs = query_string.split("&")
let params = {}

for pair in pairs {
    let parts = pair.split("=")
    if len(parts) == 2 {
        let key = Url.decode(parts[0])
        let value = Url.decode(parts[1])
        params[key] = value
    }
}

println(params.name)  // "John Doe"
println(params.city)  // "New York"
```

### Safe File Downloads

```stratum
// Encode filename for Content-Disposition header
let filename = "Report (Q1 2024).pdf"
let encoded_name = Url.encode(filename)
// Use in header: Content-Disposition: attachment; filename*=UTF-8''Report%20%28Q1%202024%29.pdf
```

### Building API URLs

```stratum
// Construct URL with encoded path and query
let base_url = "https://api.example.com"
let path = "/users/" + Url.encode("john doe")
let query = "?filter=" + Url.encode("active=true")

let full_url = base_url + path + query
// "https://api.example.com/users/john%20doe?filter=active%3Dtrue"
```

### Form Data Encoding

```stratum
// Encode form data for application/x-www-form-urlencoded
let form_data = {
    username: "user@example.com",
    password: "p@ss w0rd!",
    remember: "true"
}

let encoded_pairs = []
for key, value in form_data {
    encoded_pairs.push(Url.encode(key) + "=" + Url.encode(value))
}

let body = encoded_pairs.join("&")
// "username=user%40example.com&password=p%40ss%20w0rd%21&remember=true"
```

### Handling Special Characters

```stratum
// Different characters and their encodings
Url.encode(" ")   // "%20" (space)
Url.encode("+")   // "%2B" (plus)
Url.encode("&")   // "%26" (ampersand)
Url.encode("=")   // "%3D" (equals)
Url.encode("?")   // "%3F" (question mark)
Url.encode("#")   // "%23" (hash)
Url.encode("/")   // "%2F" (slash)
Url.encode("@")   // "%40" (at sign)
Url.encode(":")   // "%3A" (colon)
```

---

## See Also

- [Base64](base64.md) - Base64 encoding for binary data
- [Http](http.md) - HTTP requests with URLs
- [String](string.md) - String manipulation methods
