# Hash

Cryptographic hash functions for data integrity and verification.

## Overview

The Hash namespace provides cryptographic hash functions for computing fixed-size digests from arbitrary data. Hash functions are commonly used for:

- Verifying data integrity (file checksums, download verification)
- Storing password hashes (though prefer `Crypto.pbkdf2` for passwords)
- Creating unique identifiers from content
- Message authentication codes (HMAC)
- Digital signatures and certificate verification

All hash functions return lowercase hexadecimal-encoded strings. Stratum supports industry-standard algorithms: SHA-256, SHA-512, and MD5 (for legacy compatibility only).

> **Security Note:** MD5 is cryptographically broken and should only be used for legacy compatibility. For security-critical applications, use SHA-256 or SHA-512.

---

## Functions

### `Hash.sha256(data)`

Computes the SHA-256 hash of a string, returning a 64-character hex digest.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | The text to hash |

**Returns:** `String` - 64-character lowercase hexadecimal hash

**Example:**

```stratum
// Basic hashing
Hash.sha256("hello")
// "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

Hash.sha256("Hello, World!")
// "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"

// Empty string has a defined hash
Hash.sha256("")
// "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

// Same input always produces same output
assert_eq(Hash.sha256("test"), Hash.sha256("test"))

// Different inputs produce different hashes
assert(Hash.sha256("test1") != Hash.sha256("test2"))
```

---

### `Hash.sha256_bytes(bytes)`

Computes the SHA-256 hash of raw byte data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | List of byte values (0-255) |

**Returns:** `String` - 64-character lowercase hexadecimal hash

**Throws:** Error if any byte value is outside the range 0-255

**Example:**

```stratum
// Hash raw bytes
Hash.sha256_bytes([72, 101, 108, 108, 111])  // bytes for "Hello"
// "185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969"

// Hash binary data from a file
let file_bytes = File.read_bytes("document.pdf")
let checksum = Hash.sha256_bytes(file_bytes)
println("SHA-256: " + checksum)

// Verify file integrity
let expected = "a1b2c3d4..."
if Hash.sha256_bytes(file_bytes) == expected {
    println("File integrity verified")
}
```

---

### `Hash.sha512(data)`

Computes the SHA-512 hash of a string, returning a 128-character hex digest.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | The text to hash |

**Returns:** `String` - 128-character lowercase hexadecimal hash

**Example:**

```stratum
// SHA-512 produces longer, more secure hashes
Hash.sha512("hello")
// "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7"
// "2323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"

Hash.sha512("password123")
// Returns 128-character hex string

// Use for high-security applications
let api_key = "secret-key-value"
let key_hash = Hash.sha512(api_key)
```

---

### `Hash.sha512_bytes(bytes)`

Computes the SHA-512 hash of raw byte data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | List of byte values (0-255) |

**Returns:** `String` - 128-character lowercase hexadecimal hash

**Throws:** Error if any byte value is outside the range 0-255

**Example:**

```stratum
// Hash binary data with SHA-512
let bytes = [0, 1, 2, 3, 4, 5]
let hash = Hash.sha512_bytes(bytes)
println(len(hash))  // 128

// Hash a large file
let file_data = File.read_bytes("large_file.bin")
let checksum = Hash.sha512_bytes(file_data)
```

---

### `Hash.md5(data)`

Computes the MD5 hash of a string, returning a 32-character hex digest.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | The text to hash |

**Returns:** `String` - 32-character lowercase hexadecimal hash

> **Warning:** MD5 is cryptographically broken. Use only for legacy compatibility, checksums of non-security-critical data, or when required by external systems. Never use for passwords or security purposes.

**Example:**

```stratum
// MD5 for legacy compatibility
Hash.md5("hello")
// "5d41402abc4b2a76b9719d911017c592"

// Verifying legacy checksums
let expected_md5 = "098f6bcd4621d373cade4e832627b4f6"
if Hash.md5("test") == expected_md5 {
    println("Checksum matches")
}

// Content-based deduplication (non-security use)
let content_id = Hash.md5(file_content)
```

---

### `Hash.md5_bytes(bytes)`

Computes the MD5 hash of raw byte data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `bytes` | `List[Int]` | List of byte values (0-255) |

**Returns:** `String` - 32-character lowercase hexadecimal hash

**Throws:** Error if any byte value is outside the range 0-255

> **Warning:** MD5 is cryptographically broken. Use only for legacy compatibility.

**Example:**

```stratum
// Verify legacy file checksums
let file_bytes = File.read_bytes("download.zip")
let md5_hash = Hash.md5_bytes(file_bytes)

let expected = "d41d8cd98f00b204e9800998ecf8427e"
if md5_hash == expected {
    println("Download verified")
}
```

---

### `Hash.hmac_sha256(key, message)`

Computes an HMAC-SHA256 message authentication code.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `String` | The secret key |
| `message` | `String` | The message to authenticate |

**Returns:** `String` - 64-character lowercase hexadecimal HMAC

**Example:**

```stratum
// Create HMAC for message authentication
let secret = "my-secret-key"
let message = "important data"
let hmac = Hash.hmac_sha256(secret, message)
// "4a5e4c2f..."

// API request signing
let api_secret = Env.get("API_SECRET")
let request_body = Json.encode({"action": "transfer", "amount": 100})
let signature = Hash.hmac_sha256(api_secret, request_body)

// Include signature in request headers
let headers = {
    "X-Signature": signature,
    "Content-Type": "application/json"
}

// Verify received HMAC
fx verify_signature(key, message, received_hmac) {
    let computed = Hash.hmac_sha256(key, message)
    return computed == received_hmac
}
```

---

### `Hash.hmac_sha512(key, message)`

Computes an HMAC-SHA512 message authentication code.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `String` | The secret key |
| `message` | `String` | The message to authenticate |

**Returns:** `String` - 128-character lowercase hexadecimal HMAC

**Example:**

```stratum
// HMAC-SHA512 for extra security
let key = "super-secret-key"
let data = "sensitive information"
let hmac = Hash.hmac_sha512(key, data)
println(len(hmac))  // 128

// Webhook signature verification
let webhook_secret = Env.get("WEBHOOK_SECRET")
let payload = request.body
let received_sig = request.headers["X-Hub-Signature-512"]

let expected_sig = "sha512=" + Hash.hmac_sha512(webhook_secret, payload)
if expected_sig == received_sig {
    println("Webhook verified")
}
```

---

## Common Patterns

### File Integrity Verification

```stratum
// Generate checksum for a file
fx compute_file_hash(path) {
    let bytes = File.read_bytes(path)
    return Hash.sha256_bytes(bytes)
}

// Verify download integrity
let downloaded = "release-v1.0.zip"
let expected_hash = "3a7bd3e2360a3d..."  // From release notes

let actual_hash = compute_file_hash(downloaded)
if actual_hash == expected_hash {
    println("Download verified successfully")
} else {
    println("WARNING: Hash mismatch - file may be corrupted")
}
```

### Content-Addressed Storage

```stratum
// Store files by their content hash
fx store_content(content) {
    let hash = Hash.sha256(content)
    let path = "storage/" + hash.substring(0, 2) + "/" + hash

    if !File.exists(path) {
        Dir.create_all(Path.parent(path))
        File.write_text(path, content)
    }

    return hash
}

// Retrieve by hash
fx get_content(hash) {
    let path = "storage/" + hash.substring(0, 2) + "/" + hash
    return File.read_text(path)
}
```

### API Request Signing

```stratum
// Sign API requests with timestamp
fx sign_request(method, path, body, secret) {
    let timestamp = str(DateTime.now().timestamp())
    let payload = method + "\n" + path + "\n" + timestamp + "\n" + body
    let signature = Hash.hmac_sha256(secret, payload)

    return {
        "X-Timestamp": timestamp,
        "X-Signature": signature
    }
}

// Make signed request
let body = Json.encode({"amount": 100})
let headers = sign_request("POST", "/api/transfer", body, api_secret)
let response = Http.post("https://api.example.com/api/transfer", body, headers)
```

### Password Hashing (Simple)

```stratum
// Note: For production, prefer Crypto.pbkdf2 with proper salt
// This is a simple example for non-critical applications

fx hash_password(password, salt) {
    return Hash.sha256(salt + password + salt)
}

fx verify_password(password, salt, stored_hash) {
    return hash_password(password, salt) == stored_hash
}

// Usage
let salt = Uuid.v4()
let hash = hash_password("user-password", salt)
// Store both salt and hash in database
```

### Comparing Hashes Securely

```stratum
// Use constant-time comparison for security-sensitive operations
// This prevents timing attacks

fx constant_time_compare(a, b) {
    if len(a) != len(b) {
        return false
    }

    let result = 0
    let a_chars = a.chars()
    let b_chars = b.chars()

    for i in range(0, len(a)) {
        // XOR accumulates differences without early exit
        if a_chars[i] != b_chars[i] {
            result = result + 1
        }
    }

    return result == 0
}
```

---

## See Also

- [Crypto](crypto.md) - Encryption, decryption, and key derivation
- [Base64](base64.md) - Encoding binary data as text
- [Uuid](uuid.md) - Generating unique identifiers
