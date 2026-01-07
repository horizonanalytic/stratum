# Crypto

Cryptographic operations for encryption, decryption, and key derivation.

## Overview

The Crypto namespace provides cryptographic primitives for securing data. It includes:

- **AES-256-GCM encryption** - Authenticated encryption with associated data
- **PBKDF2 key derivation** - Secure password-based key generation
- **Cryptographically secure random bytes** - For keys, salts, and nonces

All cryptographic operations use industry-standard algorithms and secure defaults. AES-256-GCM provides both confidentiality and authenticity, protecting against tampering.

> **Security Note:** Cryptography is complex. These functions provide secure defaults, but proper key management, secure storage of secrets, and following security best practices are your responsibility.

---

## Functions

### `Crypto.random_bytes(n)`

Generates cryptographically secure random bytes using the operating system's secure random number generator.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Number of bytes to generate (1 to 1,000,000) |

**Returns:** `List[Int]` - List of random byte values (0-255)

**Throws:** Error if `n` is less than 1 or greater than 1,000,000

**Example:**

```stratum
// Generate random bytes for a key
let key_bytes = Crypto.random_bytes(32)  // 256 bits for AES-256
println(len(key_bytes))  // 32

// Convert to hex string for storage
fx bytes_to_hex(bytes) {
    let hex = ""
    for b in bytes {
        let h = if b < 16 { "0" } else { "" }
        // Simple hex conversion
        hex = hex + h + str(b)  // Note: Use proper hex encoding
    }
    return hex
}

// Generate a random salt
let salt = Crypto.random_bytes(16)

// Generate an initialization vector
let iv = Crypto.random_bytes(12)  // 96 bits for AES-GCM

// Generate a random token
let token_bytes = Crypto.random_bytes(32)
let token = Base64.encode(token_bytes)
```

---

### `Crypto.aes_encrypt(data, key)`

Encrypts data using AES-256-GCM authenticated encryption.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `String` | The plaintext to encrypt |
| `key` | `String` | 32-byte key as hex (64 chars) or raw 32-byte string |

**Returns:** `String` - Base64-encoded ciphertext (includes nonce and authentication tag)

**Throws:** Error if key is invalid length or encryption fails

**Key Format:**
- **Hex-encoded key:** 64 hexadecimal characters representing 32 bytes
- **Raw key:** Exactly 32 characters/bytes

**Example:**

```stratum
// Encrypt with a hex-encoded key
let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
let plaintext = "Secret message"
let encrypted = Crypto.aes_encrypt(plaintext, key)
// Returns Base64 string like "A1B2C3D4..."

// Decrypt to verify
let decrypted = Crypto.aes_decrypt(encrypted, key)
assert_eq(plaintext, decrypted)

// Generate a random key
let key_bytes = Crypto.random_bytes(32)
let key_hex = ""
for b in key_bytes {
    // Convert each byte to 2 hex chars
    let high = b / 16
    let low = b % 16
    let chars = "0123456789abcdef"
    key_hex = key_hex + chars.chars()[high] + chars.chars()[low]
}

// Encrypt with generated key
let secret = Crypto.aes_encrypt("my data", key_hex)
```

---

### `Crypto.aes_decrypt(encrypted, key)`

Decrypts data that was encrypted with `Crypto.aes_encrypt`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `encrypted` | `String` | Base64-encoded ciphertext from `aes_encrypt` |
| `key` | `String` | The same key used for encryption |

**Returns:** `String` - The original plaintext

**Throws:**
- Error if the key is incorrect
- Error if the ciphertext has been tampered with (authentication failure)
- Error if the input is not valid Base64

**Example:**

```stratum
// Basic encryption/decryption round-trip
let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
let message = "Hello, World!"

let encrypted = Crypto.aes_encrypt(message, key)
let decrypted = Crypto.aes_decrypt(encrypted, key)

assert_eq(message, decrypted)
println("Decrypted: " + decrypted)  // "Hello, World!"

// Attempting to decrypt with wrong key fails
let wrong_key = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
// Crypto.aes_decrypt(encrypted, wrong_key)  // Throws error!

// Tampering detection
let tampered = encrypted.replace("A", "B")  // Modify ciphertext
// Crypto.aes_decrypt(tampered, key)  // Throws authentication error!
```

---

### `Crypto.pbkdf2(password, salt, iterations)`

Derives a cryptographic key from a password using PBKDF2-HMAC-SHA256.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `password` | `String` | The user's password |
| `salt` | `String` | A unique salt (should be random, stored with hash) |
| `iterations` | `Int` | Work factor (higher = slower but more secure) |

**Returns:** `String` - 64-character hex-encoded key (32 bytes / 256 bits)

**Throws:**
- Error if iterations is less than 1 or greater than 10,000,000
- Error if password or salt is empty

**Recommended Iterations:**
- **100,000+** for user authentication (2024 recommendation)
- Higher is more secure but slower
- Balance security with user experience

**Example:**

```stratum
// Derive a key from a password
let password = "user-password"
let salt = Uuid.v4()  // Generate unique salt per user
let iterations = 100000

let derived_key = Crypto.pbkdf2(password, salt, iterations)
// Returns 64-char hex string

println(len(derived_key))  // 64

// Store salt and iterations with the derived key
let stored = {
    "salt": salt,
    "iterations": iterations,
    "key": derived_key
}

// Verify password
fx verify_password(input_password, stored) {
    let derived = Crypto.pbkdf2(
        input_password,
        stored["salt"],
        stored["iterations"]
    )
    return derived == stored["key"]
}

// Use derived key for encryption
let encryption_key = Crypto.pbkdf2(password, salt, iterations)
let encrypted = Crypto.aes_encrypt("secret data", encryption_key)
```

---

## Common Patterns

### Secure Password Storage

```stratum
// Store passwords securely with individual salts

fx hash_password(password) {
    let salt = Uuid.v4()
    let iterations = 100000
    let hash = Crypto.pbkdf2(password, salt, iterations)

    return {
        "hash": hash,
        "salt": salt,
        "iterations": iterations,
        "algorithm": "pbkdf2-sha256"
    }
}

fx verify_password(password, stored) {
    let computed = Crypto.pbkdf2(
        password,
        stored["salt"],
        stored["iterations"]
    )
    return computed == stored["hash"]
}

// Usage
let user_password = "correct-horse-battery-staple"
let stored = hash_password(user_password)

// Later, verify login
let attempt = "correct-horse-battery-staple"
if verify_password(attempt, stored) {
    println("Login successful")
} else {
    println("Invalid password")
}
```

### Encrypting Files

```stratum
// Encrypt a file with a password

fx encrypt_file(input_path, output_path, password) {
    // Read the file
    let content = File.read_text(input_path)

    // Derive key from password
    let salt = Uuid.v4()
    let key = Crypto.pbkdf2(password, salt, 100000)

    // Encrypt
    let encrypted = Crypto.aes_encrypt(content, key)

    // Save with salt prefix
    let output = salt + ":" + encrypted
    File.write_text(output_path, output)
}

fx decrypt_file(input_path, password) {
    // Read encrypted file
    let data = File.read_text(input_path)

    // Extract salt and ciphertext
    let parts = data.split(":")
    let salt = parts[0]
    let encrypted = parts[1]

    // Derive key
    let key = Crypto.pbkdf2(password, salt, 100000)

    // Decrypt
    return Crypto.aes_decrypt(encrypted, key)
}

// Usage
encrypt_file("secret.txt", "secret.enc", "my-password")
let content = decrypt_file("secret.enc", "my-password")
```

### Generating Secure Tokens

```stratum
// Generate secure random tokens for sessions, API keys, etc.

fx generate_token(byte_length) {
    let bytes = Crypto.random_bytes(byte_length)
    return Base64.encode(bytes)
}

fx generate_hex_token(byte_length) {
    let bytes = Crypto.random_bytes(byte_length)
    let hex = ""
    let chars = "0123456789abcdef"
    for b in bytes {
        hex = hex + chars.chars()[b / 16] + chars.chars()[b % 16]
    }
    return hex
}

// Generate session token (32 bytes = 256 bits of entropy)
let session_token = generate_token(32)
println("Session: " + session_token)

// Generate API key
let api_key = generate_hex_token(32)
println("API Key: " + api_key)
```

### Envelope Encryption

```stratum
// Use envelope encryption for large data
// - Generate a random data key
// - Encrypt data with data key
// - Encrypt data key with master key

fx envelope_encrypt(data, master_key) {
    // Generate random data encryption key
    let dek_bytes = Crypto.random_bytes(32)
    let dek = ""
    let chars = "0123456789abcdef"
    for b in dek_bytes {
        dek = dek + chars.chars()[b / 16] + chars.chars()[b % 16]
    }

    // Encrypt data with DEK
    let encrypted_data = Crypto.aes_encrypt(data, dek)

    // Encrypt DEK with master key
    let encrypted_dek = Crypto.aes_encrypt(dek, master_key)

    return {
        "encrypted_key": encrypted_dek,
        "encrypted_data": encrypted_data
    }
}

fx envelope_decrypt(envelope, master_key) {
    // Decrypt the DEK
    let dek = Crypto.aes_decrypt(envelope["encrypted_key"], master_key)

    // Decrypt data with DEK
    return Crypto.aes_decrypt(envelope["encrypted_data"], dek)
}
```

### Secure Configuration

```stratum
// Encrypt sensitive configuration values

let master_key = Env.get("ENCRYPTION_KEY")

// Encrypt a secret
let db_password = "super-secret-db-password"
let encrypted = Crypto.aes_encrypt(db_password, master_key)

// Store encrypted value in config file
let config = {
    "database": {
        "host": "localhost",
        "port": 5432,
        "password_encrypted": encrypted
    }
}
File.write_text("config.json", Json.encode(config))

// Later, decrypt when needed
let config = Json.decode(File.read_text("config.json"))
let password = Crypto.aes_decrypt(
    config["database"]["password_encrypted"],
    master_key
)
```

---

## Security Best Practices

1. **Never hardcode keys** - Store keys in environment variables or secure key management systems
2. **Use unique salts** - Generate a new random salt for each password
3. **Sufficient iterations** - Use at least 100,000 PBKDF2 iterations (increase over time)
4. **Key rotation** - Periodically rotate encryption keys
5. **Secure key storage** - Use environment variables, secrets managers, or HSMs
6. **Validate inputs** - Never trust user input for cryptographic operations
7. **Audit logging** - Log encryption/decryption operations (not keys or plaintext)

---

## See Also

- [Hash](hash.md) - Cryptographic hash functions and HMAC
- [Base64](base64.md) - Encoding binary data
- [Uuid](uuid.md) - Generating unique identifiers for salts
- [Random](random.md) - Non-cryptographic random numbers
