# Gzip

Gzip compression and decompression for data.

## Overview

The Gzip namespace provides functions for compressing and decompressing data using the gzip compression algorithm. Gzip is widely used for:

- Compressing files for storage or transfer
- HTTP content encoding (Accept-Encoding: gzip)
- Log file compression
- Data archival

Stratum provides both byte-level and text-level functions for maximum flexibility.

---

## Functions

### `Gzip.compress(bytes)`

Compresses a list of bytes using gzip compression.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | Raw byte values (0-255) to compress |

**Returns:** `List[Int]` - The gzip-compressed bytes

**Example:**

```stratum
// Compress some bytes
let data = [72, 101, 108, 108, 111]  // "Hello" as bytes
let compressed = Gzip.compress(data)
println(len(compressed))  // Compressed size (may be larger for small inputs)

// Compress larger data for better ratios
let repeated = []
for i in range(0, 1000) {
    repeated.push(65)  // 1000 'A' characters
}
let small = Gzip.compress(repeated)
println(len(small))  // Much smaller than 1000
```

---

### `Gzip.decompress(bytes)`

Decompresses gzip-encoded bytes back to original data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | Gzip-compressed byte values (0-255) |

**Returns:** `List[Int]` - The decompressed bytes

**Throws:** Error if the input is not valid gzip data

**Example:**

```stratum
// Round-trip compression
let original = [72, 101, 108, 108, 111, 33]  // "Hello!"
let compressed = Gzip.compress(original)
let decompressed = Gzip.decompress(compressed)

assert_eq(original, decompressed)

// Decompress data from a file
let compressed_data = File.read_bytes("data.gz")
let original_data = Gzip.decompress(compressed_data)
```

---

### `Gzip.compress_text(text)`

Compresses a string using gzip compression. This is a convenience function that handles UTF-8 encoding automatically.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `text` | `String` | The text string to compress |

**Returns:** `List[Int]` - The gzip-compressed bytes

**Example:**

```stratum
// Compress a string directly
let text = "Hello, Stratum! This is some text to compress."
let compressed = Gzip.compress_text(text)
println(len(compressed))

// Compress and save to file
let content = File.read_text("large-document.txt")
let compressed = Gzip.compress_text(content)
File.write_bytes("large-document.txt.gz", compressed)

// Compress JSON data
let data = {"users": [1, 2, 3], "count": 3}
let json = Json.encode(data)
let compressed = Gzip.compress_text(json)
```

---

### `Gzip.decompress_text(bytes)`

Decompresses gzip-encoded bytes and returns the result as a UTF-8 string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | Gzip-compressed byte values |

**Returns:** `String` - The decompressed text

**Throws:**
- Error if the input is not valid gzip data
- Error if the decompressed data is not valid UTF-8

**Example:**

```stratum
// Round-trip text compression
let original = "Hello, Stratum!"
let compressed = Gzip.compress_text(original)
let decompressed = Gzip.decompress_text(compressed)

assert_eq(original, decompressed)

// Read and decompress a .gz file
let compressed = File.read_bytes("log.txt.gz")
let content = Gzip.decompress_text(compressed)
println(content)

// Decompress JSON response
let compressed_json = File.read_bytes("data.json.gz")
let json_text = Gzip.decompress_text(compressed_json)
let data = Json.decode(json_text)
```

---

## Common Patterns

### Compressing Files

```stratum
// Compress a text file
let content = File.read_text("document.txt")
let compressed = Gzip.compress_text(content)
File.write_bytes("document.txt.gz", compressed)

// Compress a binary file
let binary = File.read_bytes("image.png")
let compressed = Gzip.compress(binary)
File.write_bytes("image.png.gz", compressed)
```

### Decompressing Files

```stratum
// Decompress to text
let compressed = File.read_bytes("document.txt.gz")
let text = Gzip.decompress_text(compressed)
File.write_text("document.txt", text)

// Decompress binary data
let compressed = File.read_bytes("image.png.gz")
let binary = Gzip.decompress(compressed)
File.write_bytes("image.png", binary)
```

### Working with HTTP Responses

```stratum
// Many APIs return gzip-compressed data
let response = Http.get("https://api.example.com/data")

// Check if response is gzip-encoded
if response.headers["content-encoding"] == "gzip" {
    let bytes = response.body  // as bytes
    let json_text = Gzip.decompress_text(bytes)
    let data = Json.decode(json_text)
    println(data)
}
```

### Log File Rotation

```stratum
// Compress old log file
let log_content = File.read_text("app.log")
let compressed = Gzip.compress_text(log_content)
File.write_bytes("app.log.1.gz", compressed)
File.write_text("app.log", "")  // Clear the log

// Read compressed log
let old_log = File.read_bytes("app.log.1.gz")
let content = Gzip.decompress_text(old_log)
println(content)
```

### Compression Ratio Check

```stratum
// Check compression effectiveness
let text = File.read_text("data.json")
let original_size = len(text)
let compressed = Gzip.compress_text(text)
let compressed_size = len(compressed)

let ratio = float(compressed_size) / float(original_size) * 100.0
println("Compression ratio: " + str(Math.round(ratio)) + "%")
println("Saved: " + str(original_size - compressed_size) + " bytes")
```

---

## Performance Notes

- Gzip compression works best on larger, repetitive data
- Very small inputs (< 100 bytes) may actually grow due to gzip headers
- Text and JSON typically compress very well (60-90% reduction)
- Already-compressed data (images, videos) will not compress further
- For file archiving with multiple files, consider using [Zip](zip.md) instead

---

## See Also

- [Zip](zip.md) - ZIP archive creation and extraction
- [File](file.md) - File read/write operations
- [Base64](base64.md) - Base64 encoding for compressed data
