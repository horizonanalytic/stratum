# Signal

Signal handling for graceful process control.

## Overview

The `Signal` namespace provides functions for handling operating system signals. Signals are used for inter-process communication and allow programs to respond to events like interrupts (Ctrl+C), termination requests, and other system events.

---

## Functions

### `Signal.handle(signal, handler)`

Registers a handler function to be called when a signal is received.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `signal` | `String` | Signal name: "INT", "TERM", "HUP", etc. |
| `handler` | `Function` | Handler function to call when signal is received |

**Returns:** `Null`

**Supported Signals:**

| Signal | Description | Common Trigger |
|--------|-------------|----------------|
| `"INT"` | Interrupt | Ctrl+C |
| `"TERM"` | Termination | `kill` command |
| `"HUP"` | Hangup | Terminal closed |
| `"USR1"` | User-defined 1 | Custom use |
| `"USR2"` | User-defined 2 | Custom use |

**Example:**

```stratum
// Handle Ctrl+C gracefully
Signal.handle("INT", || {
    println("Interrupt received, cleaning up...")
    cleanup()
    System.exit(0)
})

// Handle termination request
Signal.handle("TERM", || {
    println("Termination requested")
    save_state()
    System.exit(0)
})

// Run the main application
main_loop()
```

---

## Common Patterns

### Graceful Shutdown

```stratum
let running = true

Signal.handle("INT", || {
    println("\nShutting down gracefully...")
    running = false
})

Signal.handle("TERM", || {
    println("\nTermination requested...")
    running = false
})

// Main loop that can be stopped gracefully
while running {
    process_next_item()
}

println("Cleanup complete, exiting")
```

### Config Reload on SIGHUP

```stratum
let config = load_config()

Signal.handle("HUP", || {
    println("Reloading configuration...")
    config = load_config()
    println("Configuration reloaded")
})

// Server continues running with updated config
run_server(config)
```

### Progress Report on SIGUSR1

```stratum
let processed = 0
let total = 1000

Signal.handle("USR1", || {
    let percent = (processed * 100) / total
    println("Progress: " + str(percent) + "% (" + str(processed) + "/" + str(total) + ")")
})

// Long-running process
for i in range(0, total) {
    process_item(i)
    processed = i + 1
}
```

### Multiple Signal Handling

```stratum
fx setup_signal_handlers() {
    // Graceful shutdown
    Signal.handle("INT", || {
        println("Interrupted - saving work...")
        save_work()
        System.exit(130)  // 128 + signal number
    })

    // Clean termination
    Signal.handle("TERM", || {
        println("Terminated - cleaning up...")
        cleanup()
        System.exit(143)
    })

    // Reload config
    Signal.handle("HUP", || {
        println("Reloading...")
        reload_config()
    })
}

setup_signal_handlers()
main()
```

---

## Notes

- Signal handlers should be quick and avoid blocking operations
- Some signals cannot be caught (e.g., SIGKILL)
- Signal handling behavior may vary between operating systems
- On Windows, only a subset of signals is supported

---

## See Also

- [Process](process.md) - Process spawning and management
- [System](system.md) - System functions including `System.exit()`
