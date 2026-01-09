# Set

A collection type for storing unique values with efficient membership testing.

## Overview

Sets in Stratum are unordered collections that store unique hashable values. They provide O(1) average-time operations for adding, removing, and checking membership. Sets are useful when you need to eliminate duplicates or perform mathematical set operations.

**Key characteristics:**
- **Unique values**: Duplicate values are automatically ignored
- **Hashable elements**: Only null, bool, int, and string values can be stored
- **Unordered**: No guaranteed iteration order
- **Mutable**: Methods like `add()` and `remove()` modify the set in-place

---

## Creating Sets

### `Set.new()`

Creates an empty set.

**Returns:** `Set` - A new empty set

**Example:**

```stratum
let empty = Set.new()
empty.add(1)
empty.add(2)
empty.add(1)  // Duplicate, ignored
println(empty.len())  // 2
```

---

### `Set.from(list)`

Creates a set from a list, removing duplicates.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `list` | `List` | List of hashable values |

**Returns:** `Set` - A new set containing unique values from the list

**Aliases:** `Set.from_list(list)`

**Example:**

```stratum
let values = [1, 2, 2, 3, 3, 3]
let unique = Set.from(values)
println(unique.len())  // 3

let words = Set.from(["apple", "banana", "apple"])
// {"apple", "banana"}
```

---

## Properties

### `set.len()` / `set.length()`

Returns the number of elements in the set.

**Returns:** `Int` - The number of unique elements

**Example:**

```stratum
let s = Set.from([1, 2, 3])
println(s.len())  // 3
```

---

### `set.is_empty()`

Checks if the set has no elements.

**Returns:** `Bool` - `true` if the set is empty

**Example:**

```stratum
let s = Set.new()
println(s.is_empty())  // true
s.add(1)
println(s.is_empty())  // false
```

---

## Modification Methods

### `set.add(value)`

Adds a value to the set. If the value already exists, has no effect.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `hashable` | Value to add (null, bool, int, or string) |

**Returns:** `Set` - The set (for method chaining)

**Example:**

```stratum
let s = Set.new()
s.add(1).add(2).add(3)  // Method chaining
println(s.len())  // 3

s.add(1)  // Already exists, no effect
println(s.len())  // 3
```

---

### `set.remove(value)`

Removes a value from the set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `hashable` | Value to remove |

**Returns:** `Bool` - `true` if the value was present and removed

**Example:**

```stratum
let s = Set.from([1, 2, 3])
println(s.remove(2))  // true
println(s.remove(5))  // false (not present)
println(s.len())      // 2
```

---

## Query Methods

### `set.contains(value)`

Checks if a value exists in the set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `hashable` | Value to check |

**Returns:** `Bool` - `true` if the value is in the set

**Example:**

```stratum
let s = Set.from(["a", "b", "c"])
println(s.contains("b"))  // true
println(s.contains("z"))  // false
```

---

### `set.is_subset(other)`

Checks if this set is a subset of another set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to compare against |

**Returns:** `Bool` - `true` if all elements of this set are in the other set

**Example:**

```stratum
let a = Set.from([1, 2])
let b = Set.from([1, 2, 3, 4])

println(a.is_subset(b))  // true
println(b.is_subset(a))  // false
```

---

### `set.is_superset(other)`

Checks if this set is a superset of another set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to compare against |

**Returns:** `Bool` - `true` if this set contains all elements of the other set

**Example:**

```stratum
let a = Set.from([1, 2, 3, 4])
let b = Set.from([1, 2])

println(a.is_superset(b))  // true
println(b.is_superset(a))  // false
```

---

## Set Operations

### `set.union(other)`

Returns a new set containing all elements from both sets.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to union with |

**Returns:** `Set` - New set with elements from both sets

**Example:**

```stratum
let a = Set.from([1, 2, 3])
let b = Set.from([3, 4, 5])

let combined = a.union(b)
// {1, 2, 3, 4, 5}
```

---

### `set.intersection(other)`

Returns a new set containing only elements present in both sets.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to intersect with |

**Returns:** `Set` - New set with common elements

**Example:**

```stratum
let a = Set.from([1, 2, 3, 4])
let b = Set.from([3, 4, 5, 6])

let common = a.intersection(b)
// {3, 4}
```

---

### `set.difference(other)`

Returns a new set containing elements in this set but not in the other set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to subtract |

**Returns:** `Set` - New set with elements only in this set

**Example:**

```stratum
let a = Set.from([1, 2, 3, 4])
let b = Set.from([3, 4, 5, 6])

let only_in_a = a.difference(b)
// {1, 2}
```

---

### `set.symmetric_difference(other)`

Returns a new set containing elements in either set but not both.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Set` | Set to compare with |

**Returns:** `Set` - New set with elements in exactly one set

**Example:**

```stratum
let a = Set.from([1, 2, 3])
let b = Set.from([2, 3, 4])

let exclusive = a.symmetric_difference(b)
// {1, 4}
```

---

## Common Patterns

### Removing duplicates from a list

```stratum
let values = [1, 2, 2, 3, 1, 4, 3, 5]
let unique = Set.from(values)
println(unique)  // {1, 2, 3, 4, 5}
```

### Checking for common elements

```stratum
let group_a = Set.from(["Alice", "Bob", "Carol"])
let group_b = Set.from(["Bob", "David", "Eve"])

let common = group_a.intersection(group_b)
if !common.is_empty() {
    println("Common members found")
}
```

### Finding unique values across collections

```stratum
let list1 = [1, 2, 3]
let list2 = [2, 3, 4]
let list3 = [3, 4, 5]

let all = Set.from(list1)
    .union(Set.from(list2))
    .union(Set.from(list3))
// {1, 2, 3, 4, 5}
```

### Membership testing

```stratum
let allowed_statuses = Set.from(["active", "pending", "approved"])

fx validate_status(status: String) -> Bool {
    allowed_statuses.contains(status)
}
```

---

## See Also

- [List](list.md) - Ordered collection with duplicates
- [Map](map.md) - Key-value collection
