# DateTime

Date and time creation, formatting, and manipulation.

## Overview

The `DateTime` namespace provides functions for working with dates and times. DateTime values are represented as Maps containing date/time components and timezone information. All functions support timezone-aware operations with conversions between timezones.

DateTime values contain these fields:
- `year`, `month`, `day` - Date components
- `hour`, `minute`, `second`, `millisecond` - Time components
- `timestamp` - Unix timestamp in milliseconds
- `timezone` - Timezone name (e.g., "UTC", "America/New_York")

---

## Creation Functions

### `DateTime.now()`

Returns the current date and time in the local timezone.

**Parameters:** None

**Returns:** `DateTime` - Current date and time

**Example:**

```stratum
let now = DateTime.now()
println(now.year)      // e.g., 2025
println(now.timezone)  // e.g., "Local"
```

---

### `DateTime.parse(string, format?)`

Parses a string into a DateTime value. If no format is provided, attempts to parse as ISO 8601.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `string` | `String` | The date/time string to parse |
| `format` | `String?` | Optional strftime format pattern |

**Returns:** `DateTime` - The parsed date and time

**Throws:** Error if the string cannot be parsed

**Format Specifiers:**

| Specifier | Description | Example |
|-----------|-------------|---------|
| `%Y` | 4-digit year | 2025 |
| `%m` | 2-digit month (01-12) | 03 |
| `%d` | 2-digit day (01-31) | 15 |
| `%H` | 2-digit hour (00-23) | 14 |
| `%M` | 2-digit minute (00-59) | 30 |
| `%S` | 2-digit second (00-59) | 45 |
| `%z` | Timezone offset | +0000 |
| `%Z` | Timezone name | UTC |

**Example:**

```stratum
// ISO 8601 format (default)
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(dt.year)   // 2025
println(dt.month)  // 3
println(dt.day)    // 15

// Custom format
let dt2 = DateTime.parse("15/03/2025 14:30", "%d/%m/%Y %H:%M")
println(dt2.hour)  // 14

// Date only
let dt3 = DateTime.parse("2025-03-15", "%Y-%m-%d")
```

---

### `DateTime.from_timestamp(milliseconds)`

Creates a DateTime from a Unix timestamp in milliseconds.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `milliseconds` | `Int` | Unix timestamp in milliseconds since epoch |

**Returns:** `DateTime` - The corresponding date and time in UTC

**Example:**

```stratum
// January 1, 2025 00:00:00 UTC
let dt = DateTime.from_timestamp(1735689600000)
println(dt.year)      // 2025
println(dt.month)     // 1
println(dt.day)       // 1
println(dt.timezone)  // "UTC"
```

---

## Formatting Functions

### `DateTime.format(datetime, pattern)`

Formats a DateTime value as a string using strftime format specifiers.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime to format |
| `pattern` | `String` | strftime format pattern |

**Returns:** `String` - The formatted date/time string

**Common Format Patterns:**

| Pattern | Output | Example |
|---------|--------|---------|
| `%Y-%m-%d` | ISO date | 2025-03-15 |
| `%H:%M:%S` | Time (24h) | 14:30:45 |
| `%I:%M %p` | Time (12h) | 02:30 PM |
| `%Y-%m-%d %H:%M:%S` | Full datetime | 2025-03-15 14:30:45 |
| `%A, %B %d, %Y` | Long date | Saturday, March 15, 2025 |
| `%a %b %d` | Short date | Sat Mar 15 |

**Example:**

```stratum
let now = DateTime.now()

// ISO format
println(DateTime.format(now, "%Y-%m-%d"))  // "2025-03-15"

// Full datetime
println(DateTime.format(now, "%Y-%m-%d %H:%M:%S"))  // "2025-03-15 14:30:45"

// Human-readable
println(DateTime.format(now, "%A, %B %d, %Y"))  // "Saturday, March 15, 2025"

// Time only
println(DateTime.format(now, "%I:%M %p"))  // "02:30 PM"
```

---

## Component Access Functions

### `DateTime.year(datetime)`

Extracts the year component from a DateTime.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The year (e.g., 2025)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.year(dt))  // 2025
```

---

### `DateTime.month(datetime)`

Extracts the month component from a DateTime (1-12).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The month (1-12)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.month(dt))  // 3
```

---

### `DateTime.day(datetime)`

Extracts the day component from a DateTime (1-31).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The day of month (1-31)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.day(dt))  // 15
```

---

### `DateTime.hour(datetime)`

Extracts the hour component from a DateTime (0-23).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The hour (0-23)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.hour(dt))  // 14
```

---

### `DateTime.minute(datetime)`

Extracts the minute component from a DateTime (0-59).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The minute (0-59)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.minute(dt))  // 30
```

---

### `DateTime.second(datetime)`

Extracts the second component from a DateTime (0-59).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The second (0-59)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:45Z")
println(DateTime.second(dt))  // 45
```

---

### `DateTime.millisecond(datetime)`

Extracts the millisecond component from a DateTime (0-999).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - The millisecond (0-999)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:45.123Z")
println(DateTime.millisecond(dt))  // 123
```

---

### `DateTime.timestamp(datetime)`

Returns the Unix timestamp in milliseconds for a DateTime.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `Int` - Unix timestamp in milliseconds since epoch

**Example:**

```stratum
let dt = DateTime.parse("2025-01-01T00:00:00Z")
println(DateTime.timestamp(dt))  // 1735689600000
```

---

### `DateTime.weekday(datetime)`

Returns the name of the day of the week.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |

**Returns:** `String` - Day name (Monday, Tuesday, ..., Sunday)

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")
println(DateTime.weekday(dt))  // "Saturday"
```

---

## Arithmetic Functions

### `DateTime.add(datetime, duration)`

Adds a duration to a DateTime, returning a new DateTime.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |
| `duration` | `Duration` | The duration to add |

**Returns:** `DateTime` - New datetime with duration added

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")

// Add 2 hours
let later = DateTime.add(dt, Duration.hours(2))
println(DateTime.hour(later))  // 16

// Add 7 days
let next_week = DateTime.add(dt, Duration.days(7))
println(DateTime.day(next_week))  // 22

// Chain additions
let future = DateTime.add(
    DateTime.add(dt, Duration.days(30)),
    Duration.hours(12)
)
```

---

### `DateTime.subtract(datetime, duration)`

Subtracts a duration from a DateTime, returning a new DateTime.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime value |
| `duration` | `Duration` | The duration to subtract |

**Returns:** `DateTime` - New datetime with duration subtracted

**Example:**

```stratum
let dt = DateTime.parse("2025-03-15T14:30:00Z")

// Go back 3 hours
let earlier = DateTime.subtract(dt, Duration.hours(3))
println(DateTime.hour(earlier))  // 11

// Go back 1 day
let yesterday = DateTime.subtract(dt, Duration.days(1))
println(DateTime.day(yesterday))  // 14
```

---

### `DateTime.diff(datetime1, datetime2)`

Calculates the difference between two DateTimes as a Duration.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime1` | `DateTime` | The first datetime |
| `datetime2` | `DateTime` | The second datetime |

**Returns:** `Duration` - The difference (datetime1 - datetime2)

**Example:**

```stratum
let start = DateTime.parse("2025-03-15T10:00:00Z")
let end = DateTime.parse("2025-03-15T14:30:00Z")

let diff = DateTime.diff(end, start)
println(Duration.as_hours(diff))  // 4.5
println(Duration.as_mins(diff))   // 270.0

// Negative difference if reversed
let neg_diff = DateTime.diff(start, end)
println(Duration.as_hours(neg_diff))  // -4.5
```

---

## Comparison Functions

### `DateTime.compare(datetime1, datetime2)`

Compares two DateTime values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime1` | `DateTime` | The first datetime |
| `datetime2` | `DateTime` | The second datetime |

**Returns:** `Int` - Comparison result:
- `-1` if datetime1 is before datetime2
- `0` if they are equal
- `1` if datetime1 is after datetime2

**Example:**

```stratum
let dt1 = DateTime.parse("2025-03-15T10:00:00Z")
let dt2 = DateTime.parse("2025-03-15T14:00:00Z")
let dt3 = DateTime.parse("2025-03-15T10:00:00Z")

println(DateTime.compare(dt1, dt2))  // -1 (dt1 is before dt2)
println(DateTime.compare(dt2, dt1))  // 1  (dt2 is after dt1)
println(DateTime.compare(dt1, dt3))  // 0  (equal)

// Use in conditionals
if DateTime.compare(dt1, dt2) < 0 {
    println("dt1 is earlier")
}
```

---

## Timezone Functions

### `DateTime.to_utc(datetime)`

Converts a DateTime to UTC timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime to convert |

**Returns:** `DateTime` - The datetime in UTC

**Example:**

```stratum
let local = DateTime.now()
let utc = DateTime.to_utc(local)
println(utc.timezone)  // "UTC"
```

---

### `DateTime.to_local(datetime)`

Converts a DateTime to the local timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime to convert |

**Returns:** `DateTime` - The datetime in local timezone

**Example:**

```stratum
let utc = DateTime.parse("2025-03-15T14:00:00Z")
let local = DateTime.to_local(utc)
println(local.timezone)  // e.g., "Local"
```

---

### `DateTime.to_timezone(datetime, timezone)`

Converts a DateTime to a specific timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `datetime` | `DateTime` | The datetime to convert |
| `timezone` | `String` | IANA timezone name |

**Returns:** `DateTime` - The datetime in the specified timezone

**Throws:** Error if the timezone name is invalid

**Common Timezone Names:**

| Timezone | Description |
|----------|-------------|
| `UTC` | Coordinated Universal Time |
| `America/New_York` | US Eastern Time |
| `America/Los_Angeles` | US Pacific Time |
| `Europe/London` | UK Time |
| `Europe/Paris` | Central European Time |
| `Asia/Tokyo` | Japan Standard Time |
| `Australia/Sydney` | Australian Eastern Time |

**Example:**

```stratum
let utc = DateTime.parse("2025-03-15T14:00:00Z")

// Convert to different timezones
let ny = DateTime.to_timezone(utc, "America/New_York")
let tokyo = DateTime.to_timezone(utc, "Asia/Tokyo")

println(DateTime.hour(ny))     // 10 (EDT, UTC-4)
println(DateTime.hour(tokyo))  // 23 (JST, UTC+9)
println(ny.timezone)           // "America/New_York"
```

---

## See Also

- [Duration](duration.md) - Duration creation and arithmetic
- [Time](time.md) - Timers and sleep functions
