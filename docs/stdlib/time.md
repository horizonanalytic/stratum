# Time

Timers and sleep functions for timing operations.

## Overview

The `Time` namespace provides functions for pausing execution and measuring elapsed time. Use `sleep` functions to pause your program, and timers to benchmark code performance or measure operation duration.

Timers use high-resolution monotonic clocks, making them ideal for performance measurement since they're not affected by system time changes.

---

## Sleep Functions

### `Time.sleep(duration)`

Pauses execution for the specified duration.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | How long to sleep |

**Returns:** `Null`

**Example:**

```stratum
println("Starting...")
Time.sleep(Duration.seconds(2))
println("2 seconds later!")

// Sleep for half a second
Time.sleep(Duration.milliseconds(500))

// Sleep for 1 minute
Time.sleep(Duration.minutes(1))
```

---

### `Time.sleep_ms(milliseconds)`

Pauses execution for the specified number of milliseconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `milliseconds` | `Int` | How long to sleep in milliseconds |

**Returns:** `Null`

**Throws:** Error if milliseconds is negative

**Example:**

```stratum
println("Starting...")
Time.sleep_ms(1000)  // Sleep for 1 second
println("1 second later!")

// Short delay
Time.sleep_ms(100)  // 100 milliseconds

// Wait 5 seconds
Time.sleep_ms(5000)
```

---

## Timer Functions

### `Time.start()`

Starts a new high-resolution timer.

**Parameters:** None

**Returns:** `Timer` - A timer object for use with `Time.elapsed()`

**Example:**

```stratum
let timer = Time.start()

// ... do some work ...

let elapsed = Time.elapsed(timer)
println("Work took " + str(Duration.as_secs(elapsed)) + " seconds")
```

---

### `Time.elapsed(timer)`

Returns the duration since a timer was started.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `timer` | `Timer` | Timer from `Time.start()` |

**Returns:** `Duration` - Time elapsed since the timer started

**Example:**

```stratum
let timer = Time.start()

// Simulate some work
Time.sleep_ms(250)

let elapsed = Time.elapsed(timer)
println(Duration.as_millis(elapsed))  // ~250 (plus small overhead)
println(Duration.as_secs(elapsed))    // ~0.25
```

---

## Common Patterns

### Benchmarking Code

```stratum
// Measure how long a function takes
fx benchmark(fn, iterations) {
    let timer = Time.start()

    for i in range(0, iterations) {
        fn()
    }

    let elapsed = Time.elapsed(timer)
    let avg_ms = Duration.as_millis(elapsed) / iterations

    println("Total: " + str(Duration.as_secs(elapsed)) + "s")
    println("Average: " + str(avg_ms) + "ms per iteration")
}

// Usage
benchmark(fx() {
    let sum = 0
    for i in range(0, 10000) {
        sum = sum + i
    }
}, 100)
```

### Progress Reporting

```stratum
// Report progress during long operations
fx process_items(items) {
    let timer = Time.start()
    let total = len(items)

    for i in range(0, total) {
        process(items[i])

        // Report every 100 items
        if (i + 1) % 100 == 0 {
            let elapsed = Time.elapsed(timer)
            let rate = (i + 1) / Duration.as_secs(elapsed)
            println("Processed " + str(i + 1) + "/" + str(total) +
                   " (" + str(Math.round(rate)) + " items/sec)")
        }
    }

    let total_time = Time.elapsed(timer)
    println("Completed in " + str(Duration.as_secs(total_time)) + " seconds")
}
```

### Rate Limiting

```stratum
// Ensure operations don't exceed a rate limit
fx rate_limited_calls(items, min_interval_ms) {
    for item in items {
        let timer = Time.start()

        process(item)

        let elapsed = Time.elapsed(timer)
        let elapsed_ms = Duration.as_millis(elapsed)

        // If we finished too fast, sleep the remaining time
        if elapsed_ms < min_interval_ms {
            Time.sleep_ms(min_interval_ms - elapsed_ms)
        }
    }
}

// Process items with at least 100ms between each
rate_limited_calls(items, 100)
```

### Timeout Pattern

```stratum
// Try an operation with a timeout
fx with_timeout(operation, timeout_ms) {
    let timer = Time.start()

    // In practice, you'd use async/await for true timeouts
    // This pattern works for polling scenarios
    while true {
        let result = operation()

        if result != null {
            return result
        }

        if Duration.as_millis(Time.elapsed(timer)) > timeout_ms {
            return null  // Timed out
        }

        Time.sleep_ms(10)  // Small delay before retry
    }
}
```

### Countdown Timer

```stratum
// Display a countdown
fx countdown(seconds) {
    for i in range(seconds, 0) {
        println(str(i) + "...")
        Time.sleep(Duration.seconds(1))
    }
    println("Done!")
}

countdown(5)
// Output:
// 5...
// 4...
// 3...
// 2...
// 1...
// Done!
```

---

## See Also

- [DateTime](datetime.md) - Date/time creation and manipulation
- [Duration](duration.md) - Duration creation and conversion
- [Async](async.md) - Asynchronous sleep and operations
