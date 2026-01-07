# Duration

Duration creation, conversion, and arithmetic.

## Overview

The `Duration` namespace provides functions for creating and manipulating time durations. Durations represent a span of time and can be used with `DateTime.add()` and `DateTime.subtract()` for date/time arithmetic.

Durations are stored internally as milliseconds and can be created from various time units. They support arithmetic operations and conversion to different units.

---

## Creation Functions

### `Duration.milliseconds(ms)`

Creates a Duration from milliseconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `ms` | `Int` | Number of milliseconds |

**Returns:** `Duration` - The duration

**Aliases:** `Duration.millis(ms)`

**Example:**

```stratum
let d = Duration.milliseconds(500)
println(Duration.as_millis(d))  // 500
println(Duration.as_secs(d))    // 0.5
```

---

### `Duration.seconds(secs)`

Creates a Duration from seconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `secs` | `Int \| Float` | Number of seconds |

**Returns:** `Duration` - The duration

**Aliases:** `Duration.secs(secs)`

**Example:**

```stratum
let d = Duration.seconds(90)
println(Duration.as_secs(d))   // 90.0
println(Duration.as_mins(d))   // 1.5

// Fractional seconds
let half = Duration.seconds(0.5)
println(Duration.as_millis(half))  // 500
```

---

### `Duration.minutes(mins)`

Creates a Duration from minutes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `mins` | `Int \| Float` | Number of minutes |

**Returns:** `Duration` - The duration

**Aliases:** `Duration.mins(mins)`

**Example:**

```stratum
let d = Duration.minutes(30)
println(Duration.as_mins(d))    // 30.0
println(Duration.as_secs(d))    // 1800.0
println(Duration.as_hours(d))   // 0.5

// Fractional minutes
let half = Duration.minutes(1.5)
println(Duration.as_secs(half))  // 90.0
```

---

### `Duration.hours(hrs)`

Creates a Duration from hours.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `hrs` | `Int \| Float` | Number of hours |

**Returns:** `Duration` - The duration

**Example:**

```stratum
let d = Duration.hours(2)
println(Duration.as_hours(d))  // 2.0
println(Duration.as_mins(d))   // 120.0
println(Duration.as_secs(d))   // 7200.0

// Fractional hours
let quarter = Duration.hours(0.25)
println(Duration.as_mins(quarter))  // 15.0
```

---

### `Duration.days(d)`

Creates a Duration from days.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `d` | `Int \| Float` | Number of days |

**Returns:** `Duration` - The duration

**Example:**

```stratum
let d = Duration.days(7)
println(Duration.as_days(d))   // 7.0
println(Duration.as_hours(d))  // 168.0

// Half a day
let half = Duration.days(0.5)
println(Duration.as_hours(half))  // 12.0
```

---

## Conversion Functions

### `Duration.as_millis(duration)`

Converts a Duration to milliseconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | The duration to convert |

**Returns:** `Int` - The duration in milliseconds

**Example:**

```stratum
let d = Duration.seconds(2)
println(Duration.as_millis(d))  // 2000

let d2 = Duration.hours(1)
println(Duration.as_millis(d2))  // 3600000
```

---

### `Duration.as_secs(duration)`

Converts a Duration to seconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | The duration to convert |

**Returns:** `Float` - The duration in seconds

**Example:**

```stratum
let d = Duration.milliseconds(2500)
println(Duration.as_secs(d))  // 2.5

let d2 = Duration.minutes(1)
println(Duration.as_secs(d2))  // 60.0
```

---

### `Duration.as_mins(duration)`

Converts a Duration to minutes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | The duration to convert |

**Returns:** `Float` - The duration in minutes

**Example:**

```stratum
let d = Duration.seconds(90)
println(Duration.as_mins(d))  // 1.5

let d2 = Duration.hours(2)
println(Duration.as_mins(d2))  // 120.0
```

---

### `Duration.as_hours(duration)`

Converts a Duration to hours.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | The duration to convert |

**Returns:** `Float` - The duration in hours

**Example:**

```stratum
let d = Duration.minutes(90)
println(Duration.as_hours(d))  // 1.5

let d2 = Duration.days(1)
println(Duration.as_hours(d2))  // 24.0
```

---

### `Duration.as_days(duration)`

Converts a Duration to days.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration` | `Duration` | The duration to convert |

**Returns:** `Float` - The duration in days

**Example:**

```stratum
let d = Duration.hours(36)
println(Duration.as_days(d))  // 1.5

let d2 = Duration.hours(168)
println(Duration.as_days(d2))  // 7.0
```

---

## Arithmetic Functions

### `Duration.add(duration1, duration2)`

Adds two durations together.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration1` | `Duration` | First duration |
| `duration2` | `Duration` | Second duration |

**Returns:** `Duration` - Sum of the two durations

**Example:**

```stratum
let d1 = Duration.hours(2)
let d2 = Duration.minutes(30)

let total = Duration.add(d1, d2)
println(Duration.as_mins(total))   // 150.0
println(Duration.as_hours(total))  // 2.5

// Chain multiple additions
let work_day = Duration.add(
    Duration.add(Duration.hours(4), Duration.minutes(30)),  // Morning
    Duration.add(Duration.hours(4), Duration.minutes(30))   // Afternoon
)
println(Duration.as_hours(work_day))  // 9.0
```

---

### `Duration.subtract(duration1, duration2)`

Subtracts one duration from another.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `duration1` | `Duration` | Duration to subtract from |
| `duration2` | `Duration` | Duration to subtract |

**Returns:** `Duration` - Difference (duration1 - duration2)

**Note:** Result can be negative if duration2 > duration1.

**Example:**

```stratum
let d1 = Duration.hours(3)
let d2 = Duration.minutes(90)

let diff = Duration.subtract(d1, d2)
println(Duration.as_mins(diff))   // 90.0
println(Duration.as_hours(diff))  // 1.5

// Negative result
let neg = Duration.subtract(d2, d1)
println(Duration.as_mins(neg))  // -90.0
```

---

## Common Patterns

### Calculating Elapsed Time

```stratum
// Calculate how long an operation took
let start = DateTime.now()

// ... perform some operation ...

let end = DateTime.now()
let elapsed = DateTime.diff(end, start)
println("Operation took " + str(Duration.as_secs(elapsed)) + " seconds")
```

### Scheduling Future Events

```stratum
let now = DateTime.now()

// Schedule meeting in 2 days and 3 hours
let meeting_time = DateTime.add(
    DateTime.add(now, Duration.days(2)),
    Duration.hours(3)
)

println("Meeting scheduled for: " + DateTime.format(meeting_time, "%Y-%m-%d %H:%M"))
```

### Time Unit Conversion

```stratum
// Convert 10000 seconds to human-readable units
let d = Duration.seconds(10000)

let days = Math.floor(Duration.as_days(d))
let remaining = Duration.subtract(d, Duration.days(days))

let hours = Math.floor(Duration.as_hours(remaining))
let remaining2 = Duration.subtract(remaining, Duration.hours(hours))

let mins = Math.floor(Duration.as_mins(remaining2))

println(str(days) + " days, " + str(hours) + " hours, " + str(mins) + " minutes")
// "0 days, 2 hours, 46 minutes"
```

---

## See Also

- [DateTime](datetime.md) - Date/time creation and manipulation
- [Time](time.md) - Timers and sleep functions
