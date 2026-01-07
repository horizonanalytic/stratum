# Dir

Directory operations for listing, creating, and removing directories.

## Overview

The Dir namespace provides functions for working with directories (folders) in the file system. It supports listing directory contents, creating new directories, and removing existing ones.

All paths can be absolute or relative to the current working directory. Directory operations that fail (e.g., directory not found, permission denied) will throw an error.

---

## Functions

### `Dir.list(path)`

Lists the contents of a directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the directory |

**Returns:** `List[String]` - List of file and directory names (not full paths)

**Throws:** Error if the directory doesn't exist or can't be read

**Example:**

```stratum
// List current directory
let entries = Dir.list(".")
for entry in entries {
    println(entry)
}

// List a specific directory
let files = Dir.list("/home/user/documents")
println("Found " + str(len(files)) + " items")

// Filter for specific files
let entries = Dir.list("./src")
let strat_files = entries.filter(|name| name.ends_with(".strat"))
```

---

### `Dir.create(path)`

Creates a new directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path for the new directory |

**Returns:** `Null`

**Throws:** Error if the parent directory doesn't exist, the directory already exists, or creation fails

**Example:**

```stratum
// Create a single directory
Dir.create("output")

// Create in an existing path
Dir.create("./data/cache")  // "data" must already exist
```

**Note:** Use `Dir.create_all()` if you need to create parent directories.

---

### `Dir.create_all(path)`

Creates a directory and all necessary parent directories.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path for the new directory structure |

**Returns:** `Null`

**Throws:** Error if creation fails (e.g., permission denied)

**Example:**

```stratum
// Create nested directory structure
Dir.create_all("./output/reports/2024/q1")

// Safe directory creation (no error if exists)
Dir.create_all("./cache")  // Works even if "cache" already exists
```

---

### `Dir.remove(path)` / `Dir.delete(path)`

Removes an empty directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the directory to remove |

**Returns:** `Null`

**Throws:** Error if the directory doesn't exist, is not empty, or can't be removed

**Example:**

```stratum
// Remove an empty directory
Dir.remove("temp")

// Using the alias
Dir.delete("old_cache")

// Safe removal with check
if Dir.exists("build") {
    Dir.remove("build")
}
```

**Note:** The directory must be empty. Use `Dir.remove_all()` to remove a directory and its contents.

---

### `Dir.remove_all(path)` / `Dir.delete_all(path)`

Recursively removes a directory and all its contents.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the directory to remove |

**Returns:** `Null`

**Throws:** Error if the directory doesn't exist or removal fails

**Warning:** This operation is destructive and cannot be undone. All files and subdirectories will be permanently deleted.

**Example:**

```stratum
// Remove a directory tree
Dir.remove_all("./build")

// Clean up temporary files
if Dir.exists("./temp") {
    Dir.remove_all("./temp")
}

// Using the alias
Dir.delete_all("./cache")
```

---

### `Dir.exists(path)`

Checks if a directory exists at the given path.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to check |

**Returns:** `Bool` - `true` if a directory exists at the path, `false` otherwise

**Note:** Returns `false` for files. Use `File.exists()` to check for files, or `Path.exists()` to check for either.

**Example:**

```stratum
// Check before creating
if !Dir.exists("output") {
    Dir.create("output")
}

// Conditional logic
if Dir.exists("./config") {
    let files = Dir.list("./config")
    println("Config directory has " + str(len(files)) + " files")
}
```

---

## Common Patterns

### Ensure Directory Exists

```stratum
fx ensure_dir(path) {
    if !Dir.exists(path) {
        Dir.create_all(path)
    }
}

ensure_dir("./output/reports")
File.write_text("./output/reports/report.txt", content)
```

### Clean Build Directory

```stratum
fx clean_build() {
    if Dir.exists("./build") {
        Dir.remove_all("./build")
    }
    Dir.create("./build")
}

clean_build()
```

### List Files Recursively

```stratum
fx list_files(path) {
    let result = []
    for entry in Dir.list(path) {
        let full_path = Path.join(path, entry)
        if Path.is_dir(full_path) {
            result = result + list_files(full_path)
        } else {
            result.push(full_path)
        }
    }
    return result
}

let all_files = list_files("./src")
```

### Process All Files in Directory

```stratum
let dir = "./data"
for filename in Dir.list(dir) {
    let path = Path.join(dir, filename)
    if Path.is_file(path) && filename.ends_with(".json") {
        let data = Json.decode(File.read_text(path))
        println("Processing: " + filename)
        // ... process data
    }
}
```

---

## See Also

- [File](file.md) - File operations
- [Path](path.md) - Path manipulation utilities
