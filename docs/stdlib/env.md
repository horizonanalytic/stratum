# Env

Environment variable access and manipulation.

## Overview

The Env namespace provides functions for reading, writing, and managing environment variables. Environment variables are key-value pairs that configure application behavior and store system settings.

Changes made to environment variables affect only the current process and its child processes. They do not persist after the program exits.

---

## Functions

### `Env.get(name, ?default)`

Retrieves the value of an environment variable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Name of the environment variable |
| `default` | `String?` | Optional default value if variable doesn't exist |

**Returns:** `String?` - The variable's value, the default value, or `null` if not found and no default provided

**Example:**

```stratum
// Get a required variable
let home = Env.get("HOME")
println(home)  // /Users/alice

// Get with a default value
let port = Env.get("PORT", "8080")
println(port)  // 8080 (if PORT not set)

// Check for null
let api_key = Env.get("API_KEY")
if api_key == null {
    println("Warning: API_KEY not configured")
}
```

---

### `Env.set(name, value)`

Sets an environment variable to the specified value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Name of the environment variable |
| `value` | `String` | Value to set |

**Returns:** `Null`

**Example:**

```stratum
// Set configuration
Env.set("DEBUG", "true")
Env.set("LOG_LEVEL", "info")

// Configure for child processes
Env.set("PATH", Env.get("PATH") + ":/custom/bin")
```

---

### `Env.remove(name)` / `Env.unset(name)`

Removes an environment variable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Name of the environment variable to remove |

**Returns:** `Null`

**Example:**

```stratum
// Clean up sensitive data
Env.remove("API_SECRET")

// Using the alias
Env.unset("TEMP_CONFIG")

// Safe removal
if Env.has("OLD_SETTING") {
    Env.remove("OLD_SETTING")
}
```

---

### `Env.all()` / `Env.vars()`

Returns all environment variables as a map.

**Parameters:** None

**Returns:** `Map[String, String]` - Map of all environment variable names to their values

**Example:**

```stratum
// List all environment variables
let vars = Env.all()
for key, value in vars {
    println(key + "=" + value)
}

// Filter for specific prefix
let vars = Env.vars()
for key, value in vars {
    if key.starts_with("MY_APP_") {
        println(key + ": " + value)
    }
}

// Count variables
println("Total env vars: " + str(len(Env.all())))
```

---

### `Env.has(name)` / `Env.contains(name)`

Checks if an environment variable exists.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Name of the environment variable to check |

**Returns:** `Bool` - `true` if the variable exists, `false` otherwise

**Example:**

```stratum
// Check before accessing
if Env.has("DATABASE_URL") {
    let db_url = Env.get("DATABASE_URL")
    connect_to_database(db_url)
} else {
    println("Error: DATABASE_URL not configured")
    System.exit(1)
}

// Using the alias
if Env.contains("CI") {
    println("Running in CI environment")
}
```

---

## Common Patterns

### Configuration with Defaults

```stratum
fx load_config() {
    return {
        host: Env.get("HOST", "localhost"),
        port: int(Env.get("PORT", "3000")),
        debug: Env.get("DEBUG", "false") == "true",
        log_level: Env.get("LOG_LEVEL", "info")
    }
}

let config = load_config()
println("Starting server on " + config.host + ":" + str(config.port))
```

### Required Environment Variables

```stratum
fx require_env(name) {
    let value = Env.get(name)
    if value == null {
        println("Error: Required environment variable '" + name + "' is not set")
        System.exit(1)
    }
    return value
}

let api_key = require_env("API_KEY")
let db_url = require_env("DATABASE_URL")
```

### Environment-Based Branching

```stratum
let env = Env.get("ENVIRONMENT", "development")

if env == "production" {
    Env.set("LOG_LEVEL", "warn")
    Env.set("DEBUG", "false")
} else if env == "development" {
    Env.set("LOG_LEVEL", "debug")
    Env.set("DEBUG", "true")
}
```

### Temporary Environment Modification

```stratum
// Save, modify, restore pattern
let original_path = Env.get("PATH")
Env.set("PATH", "/custom/bin:" + original_path)

// Do work with modified PATH
Shell.exec("custom-tool --version")

// Restore original
Env.set("PATH", original_path)
```

---

## See Also

- [Args](args.md) - Command-line argument access
- [System](system.md) - System information and control
- [Shell](shell.md) - Shell command execution
