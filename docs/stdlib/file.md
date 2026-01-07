# File

File system operations for reading, writing, and managing files.

## Overview

The File namespace provides functions for interacting with the file system. It supports reading and writing both text and binary files, as well as common file operations like copying, renaming, and deleting.

All file paths can be absolute or relative to the current working directory. File operations that fail (e.g., file not found, permission denied) will throw an error.

---

## Functions

### `File.read_text(path)`

Reads the entire contents of a file as a string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to read |

**Returns:** `String` - The file contents as text

**Throws:** Error if the file doesn't exist or can't be read

**Example:**

```stratum
// Read a text file
let content = File.read_text("config.txt")
println(content)

// Read a JSON file
let json = File.read_text("data.json")
let data = Json.decode(json)
```

---

### `File.read_bytes(path)`

Reads the entire contents of a file as a list of bytes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to read |

**Returns:** `List[Int]` - List of byte values (0-255)

**Throws:** Error if the file doesn't exist or can't be read

**Example:**

```stratum
// Read binary data
let bytes = File.read_bytes("image.png")
println(len(bytes))  // File size in bytes

// Check file signature (PNG magic bytes)
if bytes[0] == 137 && bytes[1] == 80 {
    println("Valid PNG file")
}
```

---

### `File.read_lines(path)`

Reads a file and returns its contents as a list of lines.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to read |

**Returns:** `List[String]` - List of lines (without line endings)

**Throws:** Error if the file doesn't exist or can't be read

**Example:**

```stratum
// Process a file line by line
let lines = File.read_lines("data.csv")

for line in lines {
    let fields = line.split(",")
    println(fields[0])
}

// Count lines
let line_count = len(File.read_lines("log.txt"))
println("Lines: " + str(line_count))
```

---

### `File.write_text(path, content)`

Writes text content to a file, creating it if it doesn't exist or overwriting if it does.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to write |
| `content` | `String` | Text content to write |

**Returns:** `Null`

**Throws:** Error if the file can't be written (e.g., permission denied, directory doesn't exist)

**Example:**

```stratum
// Write simple text
File.write_text("output.txt", "Hello, World!")

// Write multiple lines
let lines = ["Line 1", "Line 2", "Line 3"]
File.write_text("lines.txt", lines.join("\n"))

// Write JSON data
let data = {name: "Alice", age: 30}
File.write_text("data.json", Json.encode(data))
```

---

### `File.write_bytes(path, bytes)`

Writes binary data to a file, creating it if it doesn't exist or overwriting if it does.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to write |
| `bytes` | `List[Int]` | List of byte values (0-255) to write |

**Returns:** `Null`

**Throws:** Error if the file can't be written

**Example:**

```stratum
// Write binary data
let bytes = [72, 101, 108, 108, 111]  // "Hello" in ASCII
File.write_bytes("binary.dat", bytes)

// Copy binary content
let original = File.read_bytes("image.png")
File.write_bytes("copy.png", original)
```

---

### `File.append(path, content)`

Appends text content to a file. Creates the file if it doesn't exist.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file |
| `content` | `String` | Text content to append |

**Returns:** `Null`

**Throws:** Error if the file can't be written

**Example:**

```stratum
// Append to a log file
File.append("app.log", "Application started\n")

// Build a file incrementally
File.write_text("report.txt", "Report\n")
File.append("report.txt", "=======\n")
File.append("report.txt", "Data: 42\n")
```

---

### `File.exists(path)`

Checks if a file exists at the given path.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to check |

**Returns:** `Bool` - `true` if a file exists at the path, `false` otherwise

**Note:** Returns `false` for directories. Use `Dir.exists()` to check for directories, or `Path.exists()` to check for either.

**Example:**

```stratum
// Check before reading
if File.exists("config.json") {
    let config = Json.decode(File.read_text("config.json"))
} else {
    println("No config file found, using defaults")
}

// Avoid overwriting
if !File.exists("output.txt") {
    File.write_text("output.txt", "New content")
}
```

---

### `File.size(path)`

Returns the size of a file in bytes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file |

**Returns:** `Int` - File size in bytes

**Throws:** Error if the file doesn't exist or metadata can't be read

**Example:**

```stratum
let size = File.size("data.bin")
println("File size: " + str(size) + " bytes")

// Check file size before processing
if File.size("upload.zip") > 10_000_000 {
    println("File too large (>10MB)")
}
```

---

### `File.delete(path)` / `File.remove(path)`

Deletes a file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the file to delete |

**Returns:** `Null`

**Throws:** Error if the file doesn't exist or can't be deleted

**Example:**

```stratum
// Delete a temporary file
File.delete("temp.txt")

// Safe deletion with existence check
if File.exists("old_data.json") {
    File.delete("old_data.json")
}

// Using the alias
File.remove("cache.dat")
```

---

### `File.copy(source, destination)`

Copies a file from one location to another.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `source` | `String` | Path to the source file |
| `destination` | `String` | Path for the new copy |

**Returns:** `Int` - Number of bytes copied

**Throws:** Error if the source doesn't exist or the destination can't be written

**Example:**

```stratum
// Create a backup
let bytes = File.copy("data.db", "data.db.backup")
println("Copied " + str(bytes) + " bytes")

// Copy to a different directory
File.copy("report.pdf", "/archive/report_2024.pdf")
```

---

### `File.rename(source, destination)` / `File.move(source, destination)`

Renames or moves a file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `source` | `String` | Current path of the file |
| `destination` | `String` | New path for the file |

**Returns:** `Null`

**Throws:** Error if the source doesn't exist or the rename fails

**Example:**

```stratum
// Rename a file
File.rename("draft.txt", "final.txt")

// Move to a different directory
File.rename("download.zip", "/archive/download.zip")

// Using the alias
File.move("temp.dat", "data.dat")
```

---

## Common Patterns

### Safe File Reading with Defaults

```stratum
fx read_config(path) {
    if File.exists(path) {
        return Json.decode(File.read_text(path))
    }
    return {debug: false, port: 8080}
}

let config = read_config("config.json")
```

### Processing Log Files

```stratum
let lines = File.read_lines("server.log")

let errors = lines.filter(|line| line.contains("ERROR"))
println("Found " + str(len(errors)) + " errors")

for error in errors {
    println(error)
}
```

### Atomic File Updates

```stratum
// Write to a temp file, then rename for atomic update
let data = Json.encode({updated: true, count: 42})
File.write_text("data.json.tmp", data)
File.rename("data.json.tmp", "data.json")
```

### Building Output Files

```stratum
let output_path = "report.txt"

// Start fresh
File.write_text(output_path, "Daily Report\n")
File.append(output_path, "============\n\n")

// Add sections
for section in sections {
    File.append(output_path, "## " + section.title + "\n")
    File.append(output_path, section.content + "\n\n")
}
```

---

## See Also

- [Dir](dir.md) - Directory operations
- [Path](path.md) - Path manipulation utilities
- [Json](json.md) - JSON encoding/decoding
- [Toml](toml.md) - TOML encoding/decoding
