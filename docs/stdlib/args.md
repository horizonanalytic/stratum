# Args

Command-line argument access.

## Overview

The Args namespace provides functions for accessing command-line arguments passed to your Stratum program. Arguments are the values provided after the script name when running from the command line.

For example, running `stratum run script.strat hello world` makes "hello" and "world" available as arguments.

---

## Functions

### `Args.all()` / `Args.list()`

Returns all command-line arguments as a list.

**Parameters:** None

**Returns:** `List[String]` - List of all arguments (excluding the program name)

**Example:**

```stratum
// Run: stratum run script.strat one two three
let args = Args.all()
println(args)  // ["one", "two", "three"]

// Iterate over arguments
for arg in Args.list() {
    println("Argument: " + arg)
}

// Check if any arguments provided
if len(Args.all()) == 0 {
    println("Usage: script.strat <input> <output>")
    System.exit(1)
}
```

---

### `Args.get(index)`

Retrieves a specific argument by its index.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index` | `Int` | Zero-based index of the argument to retrieve |

**Returns:** `String?` - The argument at the given index, or `null` if index is out of bounds

**Example:**

```stratum
// Run: stratum run script.strat input.txt output.txt
let input_file = Args.get(0)   // "input.txt"
let output_file = Args.get(1)  // "output.txt"
let missing = Args.get(2)      // null

// Safe access with validation
let filename = Args.get(0)
if filename == null {
    println("Error: No filename provided")
    System.exit(1)
}

// Process the file
let content = File.read_text(filename)
```

---

### `Args.count()` / `Args.len()`

Returns the number of command-line arguments.

**Parameters:** None

**Returns:** `Int` - The total number of arguments

**Example:**

```stratum
// Check argument count
if Args.count() < 2 {
    println("Usage: script.strat <source> <destination>")
    System.exit(1)
}

// Using the alias
let num_args = Args.len()
println("Received " + str(num_args) + " arguments")
```

---

## Common Patterns

### Simple CLI Tool

```stratum
// Run: stratum run greet.strat Alice
let name = Args.get(0)

if name == null {
    println("Usage: greet.strat <name>")
    System.exit(1)
}

println("Hello, " + name + "!")
```

### File Processing Tool

```stratum
// Run: stratum run convert.strat input.json output.yaml
if Args.count() != 2 {
    println("Usage: convert.strat <input.json> <output.yaml>")
    System.exit(1)
}

let input_path = Args.get(0)
let output_path = Args.get(1)

let data = Json.decode(File.read_text(input_path))
File.write_text(output_path, Yaml.encode(data))

println("Converted " + input_path + " to " + output_path)
```

### Flag Parsing

```stratum
// Run: stratum run tool.strat --verbose --output result.txt file.txt
let args = Args.all()
let verbose = false
let output = null
let files = []

let i = 0
while i < len(args) {
    let arg = args[i]

    if arg == "--verbose" || arg == "-v" {
        verbose = true
    } else if arg == "--output" || arg == "-o" {
        i = i + 1
        output = Args.get(i)
    } else if !arg.starts_with("-") {
        files.push(arg)
    }

    i = i + 1
}

if verbose {
    println("Processing " + str(len(files)) + " files")
}
```

### Multiple File Processing

```stratum
// Run: stratum run process.strat file1.txt file2.txt file3.txt
let files = Args.all()

if len(files) == 0 {
    println("Usage: process.strat <file1> [file2] [file3] ...")
    System.exit(1)
}

for file in files {
    if File.exists(file) {
        process_file(file)
    } else {
        println("Warning: File not found: " + file)
    }
}
```

### Subcommand Pattern

```stratum
// Run: stratum run cli.strat init project-name
// Run: stratum run cli.strat build --release
let command = Args.get(0)

if command == null {
    println("Usage: cli.strat <command> [args...]")
    println("Commands: init, build, run, test")
    System.exit(1)
}

if command == "init" {
    let project_name = Args.get(1)
    if project_name == null {
        println("Usage: cli.strat init <project-name>")
        System.exit(1)
    }
    Dir.create(project_name)
    println("Created project: " + project_name)
} else if command == "build" {
    println("Building project...")
} else if command == "run" {
    println("Running project...")
} else {
    println("Unknown command: " + command)
    System.exit(1)
}
```

---

## See Also

- [Env](env.md) - Environment variable access
- [System](system.md) - System information and control
- [Input](input.md) - Interactive user input
