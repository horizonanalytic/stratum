# Random

Random number generation utilities.

## Overview

The `Random` namespace provides functions for generating random values of various types. All functions use a cryptographically secure, thread-local random number generator.

Use these functions for:
- Generating random numbers for games, simulations, or sampling
- Picking random elements from collections
- Shuffling lists into random order
- Generating random bytes for tokens or identifiers

**Note:** These functions are suitable for general-purpose randomness. For cryptographic applications requiring specific security guarantees, use the [`Crypto`](crypto.md) namespace.

---

## Functions

### `Random.int(min, max)`

Generates a random integer in the range [min, max] (inclusive on both ends).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `min` | `Int` | Minimum value (inclusive) |
| `max` | `Int` | Maximum value (inclusive) |

**Returns:** `Int` - A random integer where min <= result <= max

**Throws:** Error if `min > max`

**Example:**

```stratum
// Roll a six-sided die
let die = Random.int(1, 6)
println(die)  // 1, 2, 3, 4, 5, or 6

// Generate a random percentage
let percent = Random.int(0, 100)

// Pick a random index for a list of 10 items
let index = Random.int(0, 9)
```

---

### `Random.float()`

Generates a random floating-point number in the range [0.0, 1.0) (includes 0, excludes 1).

**Parameters:** None

**Returns:** `Float` - A random float where 0.0 <= result < 1.0

**Example:**

```stratum
// Basic random float
let f = Random.float()
println(f)  // 0.7234... (varies each call)

// Scale to a different range [min, max)
let min = 10.0
let max = 20.0
let scaled = min + Random.float() * (max - min)

// Probability check (30% chance)
if Random.float() < 0.3 {
    println("Lucky!")
}
```

---

### `Random.bool()`

Generates a random boolean value with equal probability of `true` or `false`.

**Parameters:** None

**Returns:** `Bool` - Either `true` or `false` (50% chance each)

**Example:**

```stratum
// Coin flip
let heads = Random.bool()
if heads {
    println("Heads!")
} else {
    println("Tails!")
}

// Random yes/no decision
let approve = Random.bool()
```

---

### `Random.choice(list)`

Picks a random element from a list with uniform probability.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `list` | `List<T>` | A non-empty list of elements |

**Returns:** `T` - A randomly selected element from the list

**Throws:** Error if the list is empty

**Example:**

```stratum
// Pick a random color
let colors = ["red", "green", "blue", "yellow"]
let picked = Random.choice(colors)
println(picked)  // "green" (varies each call)

// Pick a random winner
let contestants = ["Alice", "Bob", "Charlie"]
let winner = Random.choice(contestants)
println("Winner: " + winner)

// Works with any list type
let numbers = [10, 20, 30, 40, 50]
let lucky = Random.choice(numbers)
```

---

### `Random.shuffle(list)`

Returns a new list with all elements in random order. The original list is not modified.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `list` | `List<T>` | A list of elements to shuffle |

**Returns:** `List<T>` - A new list with elements in random order

**Example:**

```stratum
// Shuffle a deck of cards
let deck = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"]
let shuffled = Random.shuffle(deck)
println(shuffled)  // ["7", "K", "2", ...] (random order)

// Original list is unchanged
println(deck)  // ["A", "2", "3", ...] (original order)

// Shuffle numbers
let nums = [1, 2, 3, 4, 5]
let mixed = Random.shuffle(nums)
```

**Algorithm:** Uses the Fisher-Yates shuffle for unbiased, uniform randomness.

---

### `Random.bytes(n)`

Generates a list of `n` random bytes, each represented as an integer in the range [0, 255].

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Number of random bytes to generate |

**Returns:** `List<Int>` - A list of `n` random integers, each in range [0, 255]

**Throws:**
- Error if `n < 0`
- Error if `n > 1,000,000`

**Example:**

```stratum
// Generate 4 random bytes
let bytes = Random.bytes(4)
println(bytes)  // [142, 67, 255, 12] (varies each call)

// Generate a random 16-byte token
let token = Random.bytes(16)

// Convert to hex string for display
let hex = token.map(|b| {
    let h = "0123456789abcdef"
    h.get(b / 16) + h.get(b % 16)
}).join("")
```

---

## Common Patterns

### Weighted Random Selection

```stratum
// Select with weighted probabilities
fx weighted_choice(items, weights) {
    let total = weights.reduce(|a, b| a + b, 0.0)
    let r = Random.float() * total

    let cumulative = 0.0
    for i in range(0, len(items)) {
        cumulative = cumulative + weights[i]
        if r < cumulative {
            return items[i]
        }
    }
    return items[len(items) - 1]
}

let options = ["common", "rare", "legendary"]
let weights = [70.0, 25.0, 5.0]
let result = weighted_choice(options, weights)
```

### Random Sample Without Replacement

```stratum
// Pick n unique items from a list
fx sample(list, n) {
    let shuffled = Random.shuffle(list)
    return shuffled.slice(0, n)
}

let deck = range(1, 53).to_list()
let hand = sample(deck, 5)  // Draw 5 cards
```

---

## See Also

- [Math](math.md) - Mathematical functions including `Math.floor`, `Math.ceil` for rounding random floats
- [Crypto](crypto.md) - Cryptographic utilities including `Crypto.random_bytes` for security-sensitive random data
- [List](list.md) - List methods like `map`, `filter`, `slice` for working with random results
