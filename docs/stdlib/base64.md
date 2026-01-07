# Base64

Base64 encoding and decoding for binary data.

## Overview

The Base64 namespace provides functions for encoding and decoding data using Base64, a binary-to-text encoding scheme. Base64 is commonly used for:

- Embedding binary data in text formats (JSON, XML, HTML)
- Encoding email attachments (MIME)
- Data URLs in web applications
- Encoding credentials for HTTP Basic Authentication

Stratum uses the **standard Base64 alphabet** (RFC 4648) with `+` and `/` characters, using `=` for padding.

---

## Functions

### `Base64.encode(input)`

Encodes a string or byte list to a Base64 string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `input` | `String \| List[Int]` | Text string or list of byte values (0-255) |

**Returns:** `String` - The Base64-encoded string

**Example:**

```stratum
// Encode a string
Base64.encode("Hello, World!")    // "SGVsbG8sIFdvcmxkIQ=="
Base64.encode("Stratum")          // "U3RyYXR1bQ=="

// Encode simple text
Base64.encode("abc")              // "YWJj"
Base64.encode("")                 // ""

// Encode byte values (useful for binary data)
Base64.encode([72, 101, 108, 108, 111])  // "SGVsbG8=" (bytes for "Hello")
Base64.encode([0, 255, 128])              // "AP+A"

// Encode credentials for HTTP Basic Auth
let credentials = "username:password"
let encoded = Base64.encode(credentials)  // "dXNlcm5hbWU6cGFzc3dvcmQ="
```

---

### `Base64.decode(encoded)`

Decodes a Base64 string back to the original data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `encoded` | `String` | A valid Base64-encoded string |

**Returns:** `String | List[Int]` - The decoded data

**Return Type Logic:**
- If the decoded bytes are valid UTF-8 text, returns a `String`
- If the decoded bytes are not valid UTF-8, returns a `List[Int]` of byte values (0-255)

**Throws:** Error if the input is not valid Base64

**Example:**

```stratum
// Decode to string (valid UTF-8)
Base64.decode("SGVsbG8sIFdvcmxkIQ==")  // "Hello, World!"
Base64.decode("U3RyYXR1bQ==")          // "Stratum"
Base64.decode("YWJj")                   // "abc"
Base64.decode("")                       // ""

// Decode credentials
let encoded = "dXNlcm5hbWU6cGFzc3dvcmQ="
let credentials = Base64.decode(encoded)  // "username:password"

// Decode binary data (returns byte list if not valid UTF-8)
let bytes = Base64.decode("AP+A")  // [0, 255, 128]

// Round-trip encoding
let original = "Test message"
let encoded = Base64.encode(original)
let decoded = Base64.decode(encoded)
assert_eq(original, decoded)
```

---

## Common Patterns

### HTTP Basic Authentication

```stratum
// Create Basic Auth header
let username = "user"
let password = "secret"
let credentials = username + ":" + password
let encoded = Base64.encode(credentials)
let auth_header = "Basic " + encoded

// auth_header: "Basic dXNlcjpzZWNyZXQ="
```

### Data URLs

```stratum
// Create a data URL for embedding
let svg_content = '<svg xmlns="http://www.w3.org/2000/svg"><circle r="50"/></svg>'
let encoded = Base64.encode(svg_content)
let data_url = "data:image/svg+xml;base64," + encoded
```

### Encoding Binary Files

```stratum
// Read binary file and encode
let bytes = File.read_bytes("image.png")
let encoded = Base64.encode(bytes)

// Save as Base64 text
File.write_text("image.b64", encoded)
```

### Decoding Email Attachments

```stratum
// Decode Base64-encoded attachment
let attachment_b64 = "SGVsbG8gZnJvbSBhdHRhY2htZW50IQ=="
let content = Base64.decode(attachment_b64)
println(content)  // "Hello from attachment!"
```

### Working with Binary Data

```stratum
// When decoded data isn't valid UTF-8, you get bytes
let binary_b64 = "////AP8A/w=="
let bytes = Base64.decode(binary_b64)

// bytes is List[Int] - each value 0-255
if type_of(bytes) == "List" {
    println("Got " + str(len(bytes)) + " bytes")
    for byte in bytes {
        print(str(byte) + " ")
    }
}
```

### Safe Encoding for URLs

```stratum
// Note: Standard Base64 uses + and / which need URL encoding
// For URL-safe scenarios, you may need to replace characters

let data = "data with special chars"
let b64 = Base64.encode(data)

// If you need URL-safe Base64, replace + and /
let url_safe = b64.replace("+", "-").replace("/", "_")
```

---

## See Also

- [Url](url.md) - URL percent-encoding
- [Crypto](crypto.md) - Cryptographic operations that may return Base64
- [File](file.md) - Reading/writing binary files
