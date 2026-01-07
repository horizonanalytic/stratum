# Input

Console input and user interaction functions.

## Overview

The Input namespace provides functions for reading user input from the terminal, displaying prompts, and validating responses. All functions operate synchronously and block until the user provides input.

These functions are useful for building interactive command-line applications, collecting user preferences, and implementing simple text-based interfaces.

---

## Functions

### `Input.read_line()`

Reads a single line from standard input (stdin).

**Parameters:** None

**Returns:** `String` - The line entered by the user (without trailing newline)

**Throws:** Error if reading from stdin fails

**Example:**

```stratum
// Basic line reading
println("Enter your name:")
let name = Input.read_line()
println("Hello, " + name + "!")

// Read multiple lines
let lines = []
for i in range(0, 3) {
    lines.push(Input.read_line())
}
```

---

### `Input.read_all()`

Reads all input from stdin until end-of-file (EOF).

**Parameters:** None

**Returns:** `String` - All input from stdin (including newlines)

**Throws:** Error if reading from stdin fails

**Example:**

```stratum
// Read piped input
// Usage: cat data.txt | stratum script.strat
let input = Input.read_all()
let lines = input.split("\n")
println("Received " + str(len(lines)) + " lines")

// Process all input at once
let data = Input.read_all()
let parsed = Json.decode(data)
```

---

### `Input.prompt(message)`

Displays a message and returns the user's input.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Text to display as the prompt |

**Returns:** `String` - User input (without trailing newline)

**Throws:** Error if reading from stdin fails

**Example:**

```stratum
// Simple prompt
let name = Input.prompt("Enter your name: ")
println("Hello, " + name)

// Prompt without space (add your own formatting)
let city = Input.prompt("City: ")
let country = Input.prompt("Country: ")

// Multi-step form
let email = Input.prompt("Email: ")
let username = Input.prompt("Username: ")
```

---

### `Input.prompt_int(message)`

Displays a message and returns the user's input parsed as an integer.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Text to display as the prompt |

**Returns:** `Int` - Parsed integer value

**Throws:**
- Error if the input cannot be parsed as an integer
- Error if reading from stdin fails

**Example:**

```stratum
// Get a number from the user
let age = Input.prompt_int("Enter your age: ")
println("You will be " + str(age + 10) + " in 10 years")

// Input validation happens automatically
// If user enters "abc", throws: "invalid integer: 'abc'"

// Get multiple numbers
let width = Input.prompt_int("Width: ")
let height = Input.prompt_int("Height: ")
let area = width * height
println("Area: " + str(area))
```

---

### `Input.prompt_bool(message)`

Displays a message and returns the user's input parsed as a boolean.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Text to display as the prompt |

**Returns:** `Bool` - Parsed boolean value

**Throws:**
- Error if the input is not a recognized boolean value
- Error if reading from stdin fails

**Accepted Values (case-insensitive):**

| True | False |
|------|-------|
| `y` | `n` |
| `yes` | `no` |
| `true` | `false` |
| `1` | `0` |

**Example:**

```stratum
// Simple yes/no question
let proceed = Input.prompt_bool("Continue? (y/n): ")
if proceed {
    println("Continuing...")
} else {
    println("Cancelled")
}

// Confirmation dialogs
let confirm = Input.prompt_bool("Delete all files? (yes/no): ")
if confirm {
    println("Deleting...")
}

// Feature toggles
let debug = Input.prompt_bool("Enable debug mode? (true/false): ")
```

---

### `Input.prompt_secret(message)`

Displays a message and reads hidden input (characters are not echoed to the terminal).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Text to display as the prompt |

**Returns:** `String` - User input (without trailing newline)

**Throws:** Error if reading secret input fails

**Note:** This function uses platform-specific APIs to hide input. It works on Windows, macOS, and Linux terminals.

**Example:**

```stratum
// Password authentication
let username = Input.prompt("Username: ")
let password = Input.prompt_secret("Password: ")

// The password is not shown while typing
if authenticate(username, password) {
    println("Login successful")
} else {
    println("Invalid credentials")
}

// API key input
let api_key = Input.prompt_secret("Enter API key: ")

// Database credentials
let db_password = Input.prompt_secret("Database password: ")
```

---

### `Input.choose(message, options)`

Displays a numbered list of options and returns the user's selection.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String` | Header text to display above the options |
| `options` | `List[String]` | List of choices to present |

**Returns:** `String` - The selected option string

**Throws:**
- Error if options list is empty
- Error if any option is not a string
- Error if user's input is not a valid number
- Error if choice is out of range

**Example:**

```stratum
// Simple menu
let color = Input.choose("Select a color:", ["Red", "Green", "Blue"])
println("You chose: " + color)

// Output:
// Select a color:
//   1. Red
//   2. Green
//   3. Blue
// Enter choice (1-3): 2
// You chose: Green

// Configuration wizard
let db_type = Input.choose("Database type:", ["SQLite", "PostgreSQL", "MySQL"])
let env = Input.choose("Environment:", ["Development", "Staging", "Production"])

// Dynamic options
let files = Dir.list(".")
let selected = Input.choose("Select a file:", files)
println("Selected: " + selected)
```

---

## Common Patterns

### Interactive CLI Application

```stratum
println("=== User Registration ===")
println()

let name = Input.prompt("Full name: ")
let email = Input.prompt("Email: ")
let age = Input.prompt_int("Age: ")
let password = Input.prompt_secret("Password: ")

let role = Input.choose("Select role:", ["User", "Admin", "Guest"])
let newsletter = Input.prompt_bool("Subscribe to newsletter? (y/n): ")

println()
println("Registration complete!")
println("Name: " + name)
println("Email: " + email)
println("Age: " + str(age))
println("Role: " + role)
println("Newsletter: " + str(newsletter))
```

### Menu-Driven Application

```stratum
fx main_menu() {
    loop {
        let choice = Input.choose("Main Menu:", [
            "View Profile",
            "Edit Settings",
            "Export Data",
            "Exit"
        ])

        if choice == "View Profile" {
            view_profile()
        } else if choice == "Edit Settings" {
            edit_settings()
        } else if choice == "Export Data" {
            export_data()
        } else if choice == "Exit" {
            println("Goodbye!")
            return
        }
    }
}
```

### Reading Piped Input

```stratum
// Usage: echo '{"name": "test"}' | stratum script.strat
let input = Input.read_all()

if input.is_empty() {
    println("No input provided")
} else {
    let data = Json.decode(input)
    println("Received: " + data.name)
}
```

### Confirmation Before Destructive Actions

```stratum
fx delete_with_confirmation(path) {
    if !File.exists(path) {
        println("File not found: " + path)
        return
    }

    let size = File.size(path)
    println("File: " + path)
    println("Size: " + str(size) + " bytes")

    let confirm = Input.prompt_bool("Delete this file? (yes/no): ")
    if confirm {
        File.delete(path)
        println("Deleted.")
    } else {
        println("Cancelled.")
    }
}
```

---

## See Also

- [Globals](globals.md) - Output functions like `println`
- [File](file.md) - File read/write operations
- [System](system.md) - System exit and platform info
