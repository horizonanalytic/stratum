# Path

Path manipulation utilities for working with file and directory paths.

## Overview

The Path namespace provides functions for manipulating file system paths without performing actual I/O operations. These functions work with path strings to extract components, join paths, and check path properties.

Most Path functions operate purely on strings and don't access the file system. Exceptions are `exists()`, `is_file()`, `is_dir()`, and `normalize()`, which do check the actual file system.

Path manipulation is platform-aware, using the appropriate separator (`/` on Unix, `\` on Windows).

---

## Functions

### `Path.join(parts...)`

Joins path components into a single path.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `parts...` | `String` | One or more path components to join |

**Returns:** `String` - The joined path

**Example:**

```stratum
// Join directory and filename
let path = Path.join("data", "users.json")
// "data/users.json" (Unix) or "data\users.json" (Windows)

// Join multiple components
let full = Path.join("home", "user", "documents", "report.pdf")
// "home/user/documents/report.pdf"

// Join with existing path
let base = "/var/log"
let file = Path.join(base, "app", "error.log")
// "/var/log/app/error.log"
```

---

### `Path.extension(path)` / `Path.ext(path)`

Returns the file extension without the leading dot.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to extract extension from |

**Returns:** `String?` - The extension without the dot, or `null` if none

**Example:**

```stratum
Path.extension("document.pdf")      // "pdf"
Path.extension("archive.tar.gz")    // "gz"
Path.extension("Makefile")          // null
Path.extension("/path/to/file.txt") // "txt"

// Using the alias
Path.ext("image.png")               // "png"

// Check file type
let ext = Path.extension(filename)
if ext == "json" {
    let data = Json.decode(File.read_text(filename))
}
```

---

### `Path.filename(path)` / `Path.file_name(path)`

Returns the final component of a path (filename with extension).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to extract filename from |

**Returns:** `String?` - The filename, or `null` for paths ending in `/`

**Example:**

```stratum
Path.filename("/home/user/document.pdf")  // "document.pdf"
Path.filename("data/config.json")         // "config.json"
Path.filename("/var/log/")                // null
Path.filename("file.txt")                 // "file.txt"

// Using the alias
Path.file_name("/path/to/report.txt")     // "report.txt"
```

---

### `Path.parent(path)`

Returns the parent directory of a path.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to get parent from |

**Returns:** `String?` - The parent path, or `null` if at root

**Example:**

```stratum
Path.parent("/home/user/file.txt")  // "/home/user"
Path.parent("/home/user")           // "/home"
Path.parent("/home")                // "/"
Path.parent("/")                    // null
Path.parent("file.txt")             // ""

// Navigate up directory tree
let path = "/var/log/app/error.log"
let dir = Path.parent(path)      // "/var/log/app"
let parent = Path.parent(dir)    // "/var/log"
```

---

### `Path.stem(path)` / `Path.file_stem(path)`

Returns the filename without its extension.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to extract stem from |

**Returns:** `String?` - The filename without extension, or `null`

**Example:**

```stratum
Path.stem("document.pdf")           // "document"
Path.stem("/path/to/image.png")     // "image"
Path.stem("archive.tar.gz")         // "archive.tar"
Path.stem("Makefile")               // "Makefile"
Path.stem("/path/to/")              // null

// Using the alias
Path.file_stem("report.txt")        // "report"

// Create output filename based on input
let input = "data.csv"
let output = Path.stem(input) + ".json"  // "data.json"
```

---

### `Path.is_absolute(path)`

Checks if a path is absolute (starts from root).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to check |

**Returns:** `Bool` - `true` if the path is absolute

**Example:**

```stratum
// Unix paths
Path.is_absolute("/home/user")      // true
Path.is_absolute("./relative")      // false
Path.is_absolute("file.txt")        // false

// Windows paths
Path.is_absolute("C:\\Users")       // true
Path.is_absolute("data\\file.txt")  // false

// Validate user input
if !Path.is_absolute(user_path) {
    user_path = Path.join(base_dir, user_path)
}
```

---

### `Path.is_relative(path)`

Checks if a path is relative (not starting from root).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to check |

**Returns:** `Bool` - `true` if the path is relative

**Example:**

```stratum
Path.is_relative("./config")        // true
Path.is_relative("data/file.txt")   // true
Path.is_relative("/absolute/path")  // false

// Resolve relative paths
if Path.is_relative(path) {
    path = Path.join(working_dir, path)
}
```

---

### `Path.normalize(path)` / `Path.canonicalize(path)`

Resolves a path to its canonical, absolute form.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to normalize |

**Returns:** `String` - The canonical absolute path

**Throws:** Error if the path doesn't exist or can't be resolved

**Note:** This function accesses the file system to resolve the path. The file or directory must exist.

**Example:**

```stratum
// Resolve relative path
Path.normalize("./src/../data/file.txt")
// Returns: "/home/user/project/data/file.txt" (absolute path)

// Resolve symbolic links
Path.normalize("/var/log")
// May return: "/private/var/log" (on macOS)

// Using the alias
Path.canonicalize("~/documents")

// Get absolute path of current file
let abs_path = Path.normalize(".")
```

---

### `Path.exists(path)`

Checks if a path exists (file or directory).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to check |

**Returns:** `Bool` - `true` if the path exists

**Note:** This function accesses the file system. Returns `true` for both files and directories. Use `Path.is_file()` or `Path.is_dir()` to distinguish.

**Example:**

```stratum
if Path.exists("config.json") {
    println("Config found")
}

// Check any path type
if Path.exists(user_input) {
    if Path.is_file(user_input) {
        println("It's a file")
    } else {
        println("It's a directory")
    }
}
```

---

### `Path.is_file(path)`

Checks if a path points to a file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to check |

**Returns:** `Bool` - `true` if the path is a file

**Note:** This function accesses the file system. Returns `false` for directories or non-existent paths.

**Example:**

```stratum
if Path.is_file("data.txt") {
    let content = File.read_text("data.txt")
}

// Filter directory contents
let entries = Dir.list("./project")
let files = entries.filter(|e| Path.is_file(Path.join("./project", e)))
```

---

### `Path.is_dir(path)`

Checks if a path points to a directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | The path to check |

**Returns:** `Bool` - `true` if the path is a directory

**Note:** This function accesses the file system. Returns `false` for files or non-existent paths.

**Example:**

```stratum
if Path.is_dir("./output") {
    let files = Dir.list("./output")
}

// Recursive directory listing
fx list_all(dir) {
    let result = []
    for entry in Dir.list(dir) {
        let path = Path.join(dir, entry)
        if Path.is_dir(path) {
            result = result + list_all(path)
        } else {
            result.push(path)
        }
    }
    return result
}
```

---

## Common Patterns

### Build Output Paths

```stratum
fx get_output_path(input_file, output_dir) {
    let name = Path.stem(input_file)
    let ext = Path.extension(input_file)
    return Path.join(output_dir, name + "_processed." + ext)
}

let output = get_output_path("data/input.csv", "output")
// "output/input_processed.csv"
```

### Safe Path Resolution

```stratum
fx resolve_path(base, relative) {
    if Path.is_absolute(relative) {
        return relative
    }
    return Path.normalize(Path.join(base, relative))
}

let resolved = resolve_path("/app", "./config/settings.json")
```

### File Type Routing

```stratum
fx process_file(path) {
    let ext = Path.extension(path)

    if ext == "json" {
        return Json.decode(File.read_text(path))
    } else if ext == "toml" {
        return Toml.decode(File.read_text(path))
    } else if ext == "yaml" || ext == "yml" {
        return Yaml.decode(File.read_text(path))
    } else {
        return File.read_text(path)
    }
}
```

### Path Manipulation

```stratum
let path = "/home/user/documents/report.pdf"

// Extract components
let dir = Path.parent(path)          // "/home/user/documents"
let file = Path.filename(path)       // "report.pdf"
let name = Path.stem(path)           // "report"
let ext = Path.extension(path)       // "pdf"

// Build new path
let backup = Path.join(dir, name + "_backup." + ext)
// "/home/user/documents/report_backup.pdf"
```

---

## See Also

- [File](file.md) - File operations
- [Dir](dir.md) - Directory operations
