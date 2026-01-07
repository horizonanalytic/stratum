# Shell

Shell command execution.

## Overview

The Shell namespace provides functions for executing external programs and shell commands. It allows Stratum programs to interact with the operating system, run system utilities, and integrate with other tools.

Commands are executed using the system's default shell (`sh` on Unix/Linux/macOS, `cmd` on Windows). Use caution when executing commands with user-provided input to avoid security vulnerabilities.

---

## Functions

### `Shell.run(program, ?args)`

Executes a program with optional arguments and returns detailed result information.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `program` | `String` | Path or name of the program to execute |
| `args` | `List[String]?` | Optional list of arguments to pass to the program |

**Returns:** `Map` - A map containing:
- `stdout` (`String`) - Standard output from the command
- `stderr` (`String`) - Standard error output from the command
- `exit_code` (`Int`) - The process exit code
- `success` (`Bool`) - `true` if exit code is 0, `false` otherwise

**Example:**

```stratum
// Run a program with arguments
let result = Shell.run("ls", ["-la", "/tmp"])
println(result.stdout)

// Check if command succeeded
let result = Shell.run("git", ["status"])
if result.success {
    println("Git status:\n" + result.stdout)
} else {
    println("Error: " + result.stderr)
}

// Get version information
let result = Shell.run("python3", ["--version"])
println(result.stdout.trim())  // Python 3.11.0
```

---

### `Shell.exec(command)`

Executes a shell command string and returns the output.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `command` | `String` | Shell command to execute |

**Returns:** `String` - The standard output from the command

**Throws:** Error if the command fails (non-zero exit code)

**Example:**

```stratum
// Simple commands
let files = Shell.exec("ls -la")
println(files)

// Pipeline commands
let count = Shell.exec("ls | wc -l")
println("Files: " + count.trim())

// Get current date
let date = Shell.exec("date +%Y-%m-%d")
println("Today: " + date.trim())

// Using shell features
let result = Shell.exec("echo $HOME && pwd")
println(result)
```

---

## Common Patterns

### Safe Command Execution

```stratum
fx safe_exec(command) {
    let result = Shell.run("sh", ["-c", command])
    if result.success {
        return result.stdout
    }
    return null
}

let output = safe_exec("git log --oneline -5")
if output != null {
    println(output)
}
```

### Capturing Both Output and Errors

```stratum
let result = Shell.run("npm", ["install"])

if !result.success {
    println("Installation failed!")
    println("Exit code: " + str(result.exit_code))
    println("Error output:\n" + result.stderr)
} else {
    println("Installation complete")
    if result.stderr != "" {
        println("Warnings:\n" + result.stderr)
    }
}
```

### Running Git Commands

```stratum
// Get current branch
let result = Shell.run("git", ["branch", "--show-current"])
let branch = result.stdout.trim()
println("Current branch: " + branch)

// Get list of changed files
let result = Shell.run("git", ["diff", "--name-only"])
let files = result.stdout.trim().split("\n")
for file in files {
    println("Modified: " + file)
}

// Check if working directory is clean
let result = Shell.run("git", ["status", "--porcelain"])
if result.stdout == "" {
    println("Working directory is clean")
}
```

### Build Script Automation

```stratum
fx run_build_step(name, command) {
    println("Running: " + name)
    let result = Shell.run("sh", ["-c", command])

    if !result.success {
        println("FAILED: " + name)
        println(result.stderr)
        System.exit(1)
    }

    println("OK: " + name)
    return result.stdout
}

run_build_step("Install dependencies", "npm install")
run_build_step("Run tests", "npm test")
run_build_step("Build project", "npm run build")

println("Build complete!")
```

### Cross-Platform Commands

```stratum
fx list_directory(path) {
    let os = System.os()

    if os == "windows" {
        return Shell.run("dir", [path])
    } else {
        return Shell.run("ls", ["-la", path])
    }
}

let result = list_directory(".")
println(result.stdout)
```

### Command Output Processing

```stratum
// Parse command output
let result = Shell.run("ps", ["aux"])
let lines = result.stdout.split("\n")

for line in lines {
    if line.contains("python") {
        println(line)
    }
}

// Get disk usage
let result = Shell.run("df", ["-h", "/"])
let lines = result.stdout.split("\n")
if len(lines) > 1 {
    let parts = lines[1].split(" ").filter(|p| p != "")
    println("Disk usage: " + parts[4])  // e.g., "45%"
}
```

### Environment-Aware Execution

```stratum
// Run command with modified environment
Env.set("NODE_ENV", "production")
let result = Shell.run("node", ["build.js"])

// Check if a tool is available
fx command_exists(name) {
    let result = Shell.run("which", [name])
    return result.success
}

if command_exists("docker") {
    println("Docker is available")
    Shell.exec("docker --version")
} else {
    println("Docker is not installed")
}
```

---

## Security Considerations

**Warning:** Never pass untrusted user input directly to shell commands without proper validation and sanitization.

```stratum
// DANGEROUS - command injection vulnerability
let filename = Args.get(0)
Shell.exec("cat " + filename)  // User could input "; rm -rf /"

// SAFER - use Shell.run with separate arguments
let filename = Args.get(0)
let result = Shell.run("cat", [filename])

// SAFEST - validate input first
let filename = Args.get(0)
if !filename.contains("..") && !filename.contains(";") {
    let result = Shell.run("cat", [filename])
}
```

---

## See Also

- [System](system.md) - System information and control
- [Env](env.md) - Environment variable access
- [Args](args.md) - Command-line argument access
- [File](file.md) - File system operations
