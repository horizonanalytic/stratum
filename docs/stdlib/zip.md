# Zip

ZIP archive creation, reading, and extraction.

## Overview

The Zip namespace provides functions for working with ZIP archives, the most widely-used archive format. ZIP archives can contain multiple files and directories with compression. Common uses include:

- Bundling multiple files for distribution
- Extracting downloaded archives
- Reading files directly from archives without extraction
- Creating backups and data exports

---

## Functions

### `Zip.list(path)`

Lists all entries in a ZIP archive with metadata.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the ZIP archive |

**Returns:** `List[Map]` - List of entry information maps

Each map contains:
| Key | Type | Description |
|-----|------|-------------|
| `name` | `String` | Entry path within the archive |
| `size` | `Int` | Uncompressed size in bytes |
| `compressed_size` | `Int` | Compressed size in bytes |
| `is_dir` | `Bool` | Whether the entry is a directory |

**Throws:** Error if the file doesn't exist or isn't a valid ZIP

**Example:**

```stratum
// List archive contents
let entries = Zip.list("archive.zip")
for entry in entries {
    if entry.is_dir {
        println("[DIR]  " + entry.name)
    } else {
        let ratio = float(entry.compressed_size) / float(entry.size) * 100.0
        println(entry.name + " (" + str(entry.size) + " bytes, " + str(Math.round(ratio)) + "% compressed)")
    }
}

// Count files and total size
let files = entries.filter(fx(e) { !e.is_dir })
let total_size = files.reduce(fx(acc, e) { acc + e.size }, 0)
println("Total: " + str(len(files)) + " files, " + str(total_size) + " bytes")
```

---

### `Zip.extract(archive, dest)`

Extracts all entries from a ZIP archive to a destination directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `archive` | `String` | Path to the ZIP archive |
| `dest` | `String` | Destination directory path |

**Returns:** `null`

**Throws:**
- Error if the archive doesn't exist or isn't valid
- Error if the destination can't be created or written to

**Example:**

```stratum
// Extract entire archive
Zip.extract("download.zip", "extracted/")
println("Extraction complete!")

// Extract to current directory
Zip.extract("files.zip", ".")

// Extract and list results
Zip.extract("project.zip", "project/")
let files = Dir.list("project/")
for file in files {
    println("Extracted: " + file)
}
```

---

### `Zip.extract_file(archive, entry, dest)`

Extracts a single file from a ZIP archive.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `archive` | `String` | Path to the ZIP archive |
| `entry` | `String` | Path of the entry within the archive |
| `dest` | `String` | Destination file path |

**Returns:** `null`

**Throws:**
- Error if the archive doesn't exist or isn't valid
- Error if the entry doesn't exist in the archive
- Error if the destination can't be written

**Example:**

```stratum
// Extract just the README
Zip.extract_file("project.zip", "README.md", "README.md")

// Extract a specific config file
Zip.extract_file("backup.zip", "config/settings.json", "settings.json")

// Extract nested file
Zip.extract_file("archive.zip", "src/main/app.strat", "app.strat")

// List then extract specific files
let entries = Zip.list("data.zip")
for entry in entries {
    if entry.name.ends_with(".csv") {
        Zip.extract_file("data.zip", entry.name, "csv/" + Path.filename(entry.name))
    }
}
```

---

### `Zip.create(output, files)`

Creates a new ZIP archive from a list of files.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `output` | `String` | Path for the new ZIP archive |
| `files` | `List[String]` | List of file paths to include |

**Returns:** `null`

**Throws:**
- Error if any source file doesn't exist
- Error if the output path can't be written

**Example:**

```stratum
// Create archive from specific files
Zip.create("documents.zip", [
    "report.pdf",
    "data.csv",
    "notes.txt"
])

// Create archive from directory listing
let files = Dir.list("project/")
    .filter(fx(f) { !f.ends_with(".tmp") })
Zip.create("project.zip", files)

// Archive all .strat files
let sources = Dir.list("src/")
    .filter(fx(f) { f.ends_with(".strat") })
Zip.create("source-backup.zip", sources)
```

---

### `Zip.read_text(archive, entry)`

Reads a file from a ZIP archive as text without extracting to disk.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `archive` | `String` | Path to the ZIP archive |
| `entry` | `String` | Path of the entry within the archive |

**Returns:** `String` - The file contents as text

**Throws:**
- Error if the archive doesn't exist or isn't valid
- Error if the entry doesn't exist
- Error if the content isn't valid UTF-8

**Example:**

```stratum
// Read a text file from archive
let readme = Zip.read_text("project.zip", "README.md")
println(readme)

// Read and parse JSON config from archive
let config_text = Zip.read_text("app.zip", "config.json")
let config = Json.decode(config_text)
println(config.version)

// Process CSV data without extraction
let csv_data = Zip.read_text("data.zip", "sales.csv")
let lines = csv_data.split("\n")
println("Rows: " + str(len(lines)))

// Read source code from archive
let source = Zip.read_text("backup.zip", "src/main.strat")
if source.contains("TODO") {
    println("Found TODOs in main.strat")
}
```

---

### `Zip.read_bytes(archive, entry)`

Reads a file from a ZIP archive as raw bytes without extracting to disk.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `archive` | `String` | Path to the ZIP archive |
| `entry` | `String` | Path of the entry within the archive |

**Returns:** `List[Int]` - The file contents as bytes (0-255)

**Throws:**
- Error if the archive doesn't exist or isn't valid
- Error if the entry doesn't exist

**Example:**

```stratum
// Read binary file from archive
let image_bytes = Zip.read_bytes("assets.zip", "logo.png")
println("Image size: " + str(len(image_bytes)) + " bytes")

// Read and process binary data
let data = Zip.read_bytes("archive.zip", "data.bin")
let checksum = Hash.sha256_bytes(data)
println("SHA256: " + checksum)

// Copy binary file from archive
let bytes = Zip.read_bytes("bundle.zip", "library.so")
File.write_bytes("lib/library.so", bytes)

// Check file signature
let bytes = Zip.read_bytes("files.zip", "document.pdf")
if bytes[0] == 37 && bytes[1] == 80 && bytes[2] == 68 && bytes[3] == 70 {
    println("Valid PDF signature")
}
```

---

## Common Patterns

### Inspecting Archive Contents

```stratum
// Get archive overview
let entries = Zip.list("archive.zip")
let files = entries.filter(fx(e) { !e.is_dir })
let dirs = entries.filter(fx(e) { e.is_dir })

println("Archive contains:")
println("  " + str(len(dirs)) + " directories")
println("  " + str(len(files)) + " files")

let total_size = files.reduce(fx(acc, e) { acc + e.size }, 0)
let compressed = files.reduce(fx(acc, e) { acc + e.compressed_size }, 0)
println("  " + str(total_size) + " bytes uncompressed")
println("  " + str(compressed) + " bytes compressed")
```

### Selective Extraction

```stratum
// Extract only certain file types
let entries = Zip.list("download.zip")
for entry in entries {
    if entry.name.ends_with(".json") || entry.name.ends_with(".yaml") {
        Zip.extract_file("download.zip", entry.name, "configs/" + Path.filename(entry.name))
        println("Extracted: " + entry.name)
    }
}
```

### Creating Backups

```stratum
// Backup with timestamp
let timestamp = DateTime.format(DateTime.now(), "%Y%m%d_%H%M%S")
let backup_name = "backup_" + timestamp + ".zip"

let files = Dir.list("data/")
Zip.create(backup_name, files)
println("Created backup: " + backup_name)
```

### Processing Files Without Extraction

```stratum
// Analyze JSON files in archive
let entries = Zip.list("logs.zip")
let json_files = entries.filter(fx(e) { e.name.ends_with(".json") })

for entry in json_files {
    let content = Zip.read_text("logs.zip", entry.name)
    let data = Json.decode(content)
    if data.level == "ERROR" {
        println("Error in " + entry.name + ": " + data.message)
    }
}
```

### Comparing Archives

```stratum
// Compare two archives
let old_entries = Zip.list("v1.zip")
let new_entries = Zip.list("v2.zip")

let old_names = old_entries.map(fx(e) { e.name })
let new_names = new_entries.map(fx(e) { e.name })

// Find added files
for name in new_names {
    if !old_names.contains(name) {
        println("Added: " + name)
    }
}

// Find removed files
for name in old_names {
    if !new_names.contains(name) {
        println("Removed: " + name)
    }
}
```

### Extract and Verify

```stratum
// Extract with checksum verification
let entries = Zip.list("secure.zip")
Zip.extract("secure.zip", "output/")

for entry in entries {
    if !entry.is_dir {
        let expected_size = entry.size
        let actual_path = "output/" + entry.name
        let actual_size = File.size(actual_path)

        if expected_size != actual_size {
            println("Warning: Size mismatch for " + entry.name)
        }
    }
}
println("Extraction verified!")
```

---

## See Also

- [Gzip](gzip.md) - Gzip compression for single files/streams
- [File](file.md) - File read/write operations
- [Dir](dir.md) - Directory operations
- [Path](path.md) - Path manipulation
