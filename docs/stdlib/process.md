# Process

Process management functions for spawning and controlling external processes.

## Overview

The `Process` namespace provides functions for creating and managing child processes. Unlike `Shell.run()` which blocks until completion, `Process.spawn()` allows non-blocking process execution with separate control over the process lifecycle.

---

## Functions

### `Process.spawn(command, args?)`

Spawns a new process without blocking.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `command` | `String` | The command or executable to run |
| `args` | `List<String>?` | Optional list of command arguments |

**Returns:** `Map` - Process handle with PID and control methods

The returned map contains:
- `pid`: `Int` - Process ID
- `status`: `String` - Current status ("running", "exited", "failed")

**Example:**

```stratum
// Spawn a simple command
let proc = Process.spawn("sleep", ["10"])
println("Started process with PID: " + str(proc.pid))

// Spawn without arguments
let proc2 = Process.spawn("myserver")

// Spawn with multiple arguments
let build = Process.spawn("cargo", ["build", "--release"])

// Check the process status
println("Status: " + proc.status)
```

---

### `Process.kill(pid)`

Terminates a process by its PID.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pid` | `Int` | Process ID to terminate |

**Returns:** `Bool` - `true` if the signal was sent successfully

**Example:**

```stratum
// Spawn then kill a process
let proc = Process.spawn("sleep", ["100"])
println("Started PID: " + str(proc.pid))

await Async.sleep(1000)  // Wait a second

let killed = Process.kill(proc.pid)
if killed {
    println("Process terminated")
}
```

---

## Common Patterns

### Background Server

```stratum
// Start a development server in background
let server = Process.spawn("python", ["-m", "http.server", "8080"])
println("Server started on port 8080 (PID: " + str(server.pid) + ")")

// Do other work...
await some_setup_tasks()

// When done, clean up
Process.kill(server.pid)
```

### Process Pool

```stratum
// Start multiple worker processes
let workers = []
for i in range(0, System.cpu_count()) {
    let worker = Process.spawn("./worker", [str(i)])
    workers.push(worker)
}

println("Started " + str(workers.len()) + " workers")

// Later, kill all workers
for worker in workers {
    Process.kill(worker.pid)
}
```

### Build and Watch Pattern

```stratum
// Start a file watcher in background
let watcher = Process.spawn("fswatch", [".", "-r"])

// Run build when files change
// ... (handle watcher output)

// Clean up
Process.kill(watcher.pid)
```

---

## See Also

- [Shell](shell.md) - Blocking shell command execution
- [Signal](signal.md) - Signal handling for processes
- [System](system.md) - System information and control
