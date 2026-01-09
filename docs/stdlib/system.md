# System

System information and control functions.

## Overview

The System namespace provides functions for querying system information (OS, architecture, memory), managing the current working directory, creating temporary files, and controlling program execution.

These functions are useful for writing cross-platform scripts, getting system metrics, and managing program lifecycle.

---

## Functions

### `System.os()`

Returns the name of the operating system.

**Parameters:** None

**Returns:** `String` - The OS name: `"macos"`, `"linux"`, `"windows"`, or another platform identifier

**Example:**

```stratum
let os = System.os()
println("Running on: " + os)

// Platform-specific behavior
if System.os() == "windows" {
    let home = Env.get("USERPROFILE")
} else {
    let home = Env.get("HOME")
}
```

---

### `System.arch()`

Returns the CPU architecture.

**Parameters:** None

**Returns:** `String` - The architecture: `"x86_64"`, `"aarch64"`, `"arm"`, or another identifier

**Example:**

```stratum
let arch = System.arch()
println("Architecture: " + arch)

// Check for Apple Silicon
if System.os() == "macos" && System.arch() == "aarch64" {
    println("Running on Apple Silicon")
}
```

---

### `System.cwd()`

Returns the current working directory.

**Parameters:** None

**Returns:** `String` - Absolute path to the current working directory

**Example:**

```stratum
let cwd = System.cwd()
println("Current directory: " + cwd)

// Build paths relative to cwd
let config_path = Path.join(System.cwd(), "config.json")
```

---

### `System.set_cwd(path)`

Changes the current working directory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the new working directory |

**Returns:** `Null`

**Throws:** Error if the directory doesn't exist or access is denied

**Example:**

```stratum
// Change to a specific directory
System.set_cwd("/home/user/project")
println("Now in: " + System.cwd())

// Save and restore working directory
let original = System.cwd()
System.set_cwd("/tmp")
// ... do work ...
System.set_cwd(original)
```

---

### `System.temp_dir()`

Returns the path to the system's temporary directory.

**Parameters:** None

**Returns:** `String` - Path to the temporary directory

**Example:**

```stratum
let tmp = System.temp_dir()
println("Temp directory: " + tmp)  // e.g., /tmp or C:\Users\...\AppData\Local\Temp

// Create a temp file path
let temp_file = Path.join(System.temp_dir(), "my_temp_file.txt")
File.write_text(temp_file, "temporary data")
```

---

### `System.temp_file()`

Creates a new temporary file and returns its path.

**Parameters:** None

**Returns:** `String` - Path to the newly created temporary file

**Example:**

```stratum
// Create a temp file
let temp_path = System.temp_file()
println("Created: " + temp_path)

// Write data to the temp file
File.write_text(temp_path, "temporary content")

// Process and clean up
let content = File.read_text(temp_path)
File.delete(temp_path)
```

---

### `System.exit(?code)`

Terminates the program with an optional exit code.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `code` | `Int?` | Exit code (default: 0). Convention: 0 = success, non-zero = error |

**Returns:** Does not return (program terminates)

**Example:**

```stratum
// Success exit
println("Done!")
System.exit()  // Exit with code 0

// Error exit
if !File.exists("required.txt") {
    println("Error: required.txt not found")
    System.exit(1)
}

// Exit with specific error code
if validation_failed {
    System.exit(2)  // Custom error code
}
```

---

### `System.cpu_count()`

Returns the number of CPU cores available.

**Parameters:** None

**Returns:** `Int` - Number of CPU cores

**Example:**

```stratum
let cpus = System.cpu_count()
println("CPU cores: " + str(cpus))

// Scale workers to CPU count
let worker_count = System.cpu_count()
for i in range(0, worker_count) {
    spawn_worker(i)
}
```

---

### `System.total_memory()`

Returns the total system memory in bytes.

**Parameters:** None

**Returns:** `Int` - Total memory in bytes

**Example:**

```stratum
let memory = System.total_memory()
println("Total memory: " + str(memory) + " bytes")

// Format as human-readable
let gb = memory / (1024 * 1024 * 1024)
println("Total memory: " + str(gb) + " GB")

// Check minimum requirements
let min_memory = 4 * 1024 * 1024 * 1024  // 4 GB
if System.total_memory() < min_memory {
    println("Warning: Less than 4 GB of RAM available")
}
```

---

### `System.hostname()`

Returns the system's hostname.

**Parameters:** None

**Returns:** `String` - The hostname of the machine

**Example:**

```stratum
let hostname = System.hostname()
println("Hostname: " + hostname)

// Use in logging
fx log_with_host(message: String) {
    let ts = DateTime.now()
    println("[" + System.hostname() + "] " + str(ts) + ": " + message)
}
```

---

### `System.uptime()`

Returns the system uptime in seconds.

**Parameters:** None

**Returns:** `Int` - System uptime in seconds

**Example:**

```stratum
let uptime = System.uptime()
println("System uptime: " + str(uptime) + " seconds")

// Format as human-readable
let hours = uptime / 3600
let minutes = (uptime % 3600) / 60
println("Uptime: " + str(hours) + "h " + str(minutes) + "m")

// Check for fresh reboot
if System.uptime() < 300 {  // Less than 5 minutes
    println("System was recently restarted")
}
```

---

## Common Patterns

### System Information Display

```stratum
fx print_system_info() {
    println("System Information")
    println("==================")
    println("OS:           " + System.os())
    println("Architecture: " + System.arch())
    println("CPU Cores:    " + str(System.cpu_count()))

    let memory_gb = System.total_memory() / (1024 * 1024 * 1024)
    println("Memory:       " + str(memory_gb) + " GB")

    println("CWD:          " + System.cwd())
    println("Temp Dir:     " + System.temp_dir())
}

print_system_info()
```

### Cross-Platform Path Handling

```stratum
fx get_config_dir() {
    let os = System.os()

    if os == "windows" {
        return Env.get("APPDATA") + "\\MyApp"
    } else if os == "macos" {
        return Env.get("HOME") + "/Library/Application Support/MyApp"
    } else {
        // Linux and others
        return Env.get("HOME") + "/.config/myapp"
    }
}

let config_dir = get_config_dir()
if !Dir.exists(config_dir) {
    Dir.create_all(config_dir)
}
```

### Graceful Shutdown Handler

```stratum
fx main() {
    println("Starting application...")

    if !initialize() {
        println("Failed to initialize")
        System.exit(1)
    }

    let result = run_application()

    if result.success {
        println("Application completed successfully")
        System.exit(0)
    } else {
        println("Application failed: " + result.error)
        System.exit(result.exit_code)
    }
}

main()
```

### Temporary File Workflow

```stratum
// Process data through a temp file
fx process_with_temp(data) {
    let temp = System.temp_file()

    // Write input
    File.write_text(temp, data)

    // Process with external tool
    let result = Shell.run("processor", [temp])

    // Read result (if tool modified in place)
    let output = File.read_text(temp)

    // Clean up
    File.delete(temp)

    return output
}
```

### Resource-Aware Processing

```stratum
fx choose_batch_size() {
    let memory_gb = System.total_memory() / (1024 * 1024 * 1024)
    let cpus = System.cpu_count()

    // Scale batch size based on available resources
    if memory_gb >= 16 && cpus >= 8 {
        return 10000  // High-resource system
    } else if memory_gb >= 8 && cpus >= 4 {
        return 5000   // Medium-resource system
    } else {
        return 1000   // Low-resource system
    }
}

let batch_size = choose_batch_size()
println("Using batch size: " + str(batch_size))
```

---

## See Also

- [Env](env.md) - Environment variable access
- [Args](args.md) - Command-line argument access
- [Shell](shell.md) - Shell command execution
- [Path](path.md) - Path manipulation utilities
- [File](file.md) - File system operations
