# Uuid

Universally unique identifier generation and validation.

## Overview

The Uuid namespace provides functions for generating and working with UUIDs (Universally Unique Identifiers). UUIDs are 128-bit identifiers that are globally unique without requiring a central authority.

Stratum supports:

- **UUID v4** - Random UUIDs (most common)
- **UUID v7** - Time-ordered UUIDs (sortable, newer standard)

UUIDs are commonly used for:

- Database primary keys
- Distributed system identifiers
- Session tokens and API keys
- File and resource naming
- Correlation IDs for logging

All UUIDs are returned in the standard lowercase format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (36 characters with hyphens).

---

## Functions

### `Uuid.v4()`

Generates a random UUID version 4.

**Parameters:** None

**Returns:** `String` - A random UUID in standard format

**Example:**

```stratum
// Generate a random UUID
let id = Uuid.v4()
println(id)  // "550e8400-e29b-41d4-a716-446655440000"

// Each call generates a unique ID
let id1 = Uuid.v4()
let id2 = Uuid.v4()
assert(id1 != id2)

// Use for database records
let user = {
    "id": Uuid.v4(),
    "name": "Alice",
    "email": "alice@example.com"
}

// Use for unique filenames
let filename = "upload-" + Uuid.v4() + ".png"
```

---

### `Uuid.v7()`

Generates a time-ordered UUID version 7.

UUID v7 encodes a Unix timestamp in the first 48 bits, making UUIDs generated close in time sort together. This is beneficial for:

- Database indexing efficiency
- Time-based ordering without a separate timestamp field
- Distributed systems where rough ordering matters

**Parameters:** None

**Returns:** `String` - A time-ordered UUID in standard format

**Example:**

```stratum
// Generate time-ordered UUIDs
let id1 = Uuid.v7()
Time.sleep_ms(1)
let id2 = Uuid.v7()
Time.sleep_ms(1)
let id3 = Uuid.v7()

// v7 UUIDs sort chronologically
let ids = [id3, id1, id2]
ids.sort()
assert_eq(ids, [id1, id2, id3])

// Use for log entries (maintains order)
let log_entry = {
    "id": Uuid.v7(),
    "level": "info",
    "message": "User logged in"
}

// Use for event sourcing
let event = {
    "event_id": Uuid.v7(),
    "type": "OrderCreated",
    "data": { "order_id": "12345" }
}
```

---

### `Uuid.parse(uuid_string)`

Parses a UUID string and returns it in canonical lowercase format.

This function accepts UUIDs in various formats and normalizes them:
- With or without hyphens
- Uppercase or lowercase
- With or without braces

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `uuid_string` | `String` | A UUID string to parse |

**Returns:** `String` - The UUID in canonical lowercase format with hyphens

**Throws:** Error if the string is not a valid UUID

**Example:**

```stratum
// Parse various formats
Uuid.parse("550E8400-E29B-41D4-A716-446655440000")
// "550e8400-e29b-41d4-a716-446655440000"

Uuid.parse("550e8400e29b41d4a716446655440000")
// "550e8400-e29b-41d4-a716-446655440000"

Uuid.parse("{550E8400-E29B-41D4-A716-446655440000}")
// "550e8400-e29b-41d4-a716-446655440000"

// Normalize user input
let user_input = "550E8400E29B41D4A716446655440000"
let normalized = Uuid.parse(user_input)
println(normalized)  // Standard format

// Handle potential parse errors
let input = request.query["id"]
if Uuid.is_valid(input) {
    let uuid = Uuid.parse(input)
    // Process valid UUID
} else {
    println("Invalid UUID provided")
}
```

---

### `Uuid.is_valid(uuid_string)`

Checks if a string is a valid UUID.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `uuid_string` | `String` | A string to validate |

**Returns:** `Bool` - `true` if the string is a valid UUID, `false` otherwise

**Example:**

```stratum
// Validate UUIDs
Uuid.is_valid("550e8400-e29b-41d4-a716-446655440000")  // true
Uuid.is_valid("550E8400-E29B-41D4-A716-446655440000")  // true (case insensitive)
Uuid.is_valid("550e8400e29b41d4a716446655440000")      // true (no hyphens)

// Invalid UUIDs
Uuid.is_valid("")                                      // false
Uuid.is_valid("not-a-uuid")                            // false
Uuid.is_valid("550e8400-e29b-41d4-a716")               // false (too short)
Uuid.is_valid("550e8400-e29b-41d4-a716-44665544000g")  // false (invalid char)

// Input validation
fx process_user(user_id) {
    if !Uuid.is_valid(user_id) {
        throw "Invalid user ID format"
    }

    // Safe to use
    let normalized = Uuid.parse(user_id)
    return fetch_user(normalized)
}
```

---

## Common Patterns

### Database Primary Keys

```stratum
// Using v7 for better database performance
// v7 UUIDs are time-ordered, reducing index fragmentation

fx create_user(name, email) {
    return {
        "id": Uuid.v7(),  // Time-ordered for better indexing
        "name": name,
        "email": email,
        "created_at": DateTime.now()
    }
}

// Insert multiple records
let users = [
    create_user("Alice", "alice@example.com"),
    create_user("Bob", "bob@example.com"),
    create_user("Charlie", "charlie@example.com")
]

// IDs will sort in creation order
```

### Correlation IDs for Logging

```stratum
// Track requests across distributed services

fx handle_request(request) {
    // Generate or extract correlation ID
    let correlation_id = request.headers["X-Correlation-ID"]
    if correlation_id == null || !Uuid.is_valid(correlation_id) {
        correlation_id = Uuid.v4()
    } else {
        correlation_id = Uuid.parse(correlation_id)
    }

    // Log with correlation ID
    Log.info("Request received", {
        "correlation_id": correlation_id,
        "path": request.path
    })

    // Pass to downstream services
    let response = Http.get(
        "https://api.internal/data",
        { "X-Correlation-ID": correlation_id }
    )

    return response
}
```

### Unique File Names

```stratum
// Generate unique filenames for uploads

fx save_upload(file_bytes, original_name) {
    // Extract extension
    let ext = Path.extension(original_name)

    // Generate unique name
    let unique_name = Uuid.v4() + "." + ext

    // Save file
    let path = Path.join("uploads", unique_name)
    File.write_bytes(path, file_bytes)

    return {
        "original": original_name,
        "stored": unique_name,
        "path": path
    }
}
```

### Session Tokens

```stratum
// Generate secure session identifiers

fx create_session(user_id) {
    let session_id = Uuid.v4()

    let session = {
        "id": session_id,
        "user_id": user_id,
        "created_at": DateTime.now(),
        "expires_at": DateTime.add_hours(DateTime.now(), 24)
    }

    // Store session
    sessions[session_id] = session

    return session_id
}

fx get_session(session_id) {
    if !Uuid.is_valid(session_id) {
        return null
    }

    let normalized = Uuid.parse(session_id)
    return sessions[normalized]
}
```

### Idempotency Keys

```stratum
// Use UUIDs for idempotent operations

fx transfer_money(from_account, to_account, amount) {
    // Client generates idempotency key
    let idempotency_key = Uuid.v4()

    let response = Http.post(
        "https://api.bank.com/transfers",
        Json.encode({
            "from": from_account,
            "to": to_account,
            "amount": amount
        }),
        {
            "Idempotency-Key": idempotency_key
        }
    )

    return response
}
```

### API Resource IDs

```stratum
// REST API with UUID-based resource IDs

let orders = {}

fx create_order(items) {
    let order_id = Uuid.v7()  // Time-ordered

    let order = {
        "id": order_id,
        "items": items,
        "status": "pending",
        "created_at": DateTime.now()
    }

    orders[order_id] = order
    return order
}

fx get_order(order_id) {
    if !Uuid.is_valid(order_id) {
        throw "Invalid order ID"
    }

    let normalized = Uuid.parse(order_id)
    let order = orders[normalized]

    if order == null {
        throw "Order not found"
    }

    return order
}
```

---

## UUID Version Comparison

| Version | Generation | Use Case |
|---------|-----------|----------|
| **v4** | Random | General purpose, privacy (no timestamp leakage) |
| **v7** | Time + Random | Database keys, sortable IDs, event ordering |

### When to Use v4

- Privacy is important (no timestamp embedded)
- Random distribution is desired
- Compatibility with older systems

### When to Use v7

- Database primary keys (better index performance)
- Event sourcing and logging
- Distributed systems needing rough ordering
- When creation time should be encoded in the ID

---

## See Also

- [Random](random.md) - Non-cryptographic random values
- [Crypto](crypto.md) - Cryptographic operations (UUIDs can be used as salts)
- [Hash](hash.md) - Hashing functions
- [DateTime](datetime.md) - Timestamps (v7 UUIDs encode time)
