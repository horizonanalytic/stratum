# List

Methods available on list (array) values.

## Overview

Lists in Stratum are ordered, mutable collections that can hold values of any type. Lists are created using square bracket syntax and support zero-based indexing with negative index support for accessing elements from the end.

List methods are called on list values using dot notation: `[1, 2, 3].len()`.

**Key characteristics:**
- Mutable: Methods like `push()`, `pop()`, and `reverse()` modify the list in-place
- Reference semantics: Assigning a list to a new variable creates a reference, not a copy
- Mixed types: Lists can contain values of different types
- Iteration: Lists can be used directly in `for` loops

---

## Creating Lists

```stratum
// Empty list
let empty = []

// List with values
let numbers = [1, 2, 3, 4, 5]
let mixed = [1, "hello", true, null]

// Nested lists
let matrix = [[1, 2], [3, 4], [5, 6]]
```

---

## Index Access

Lists support bracket notation for reading and writing elements.

```stratum
let list = ["a", "b", "c", "d"]

// Read by index (0-based)
list[0]     // "a"
list[2]     // "c"

// Negative indexing (from end)
list[-1]    // "d" (last element)
list[-2]    // "c" (second to last)

// Write by index
list[0] = "z"  // list is now ["z", "b", "c", "d"]
```

---

## Properties

### `.len()` / `.length()`

Returns the number of elements in the list.

**Returns:** `Int` - The number of elements

**Example:**

```stratum
[1, 2, 3].len()        // 3
[].len()               // 0
["a", "b"].length()    // 2
```

---

### `.is_empty()`

Checks if the list has zero elements.

**Returns:** `Bool` - `true` if the list is empty, `false` otherwise

**Example:**

```stratum
[].is_empty()          // true
[1, 2, 3].is_empty()   // false
```

---

## Access Methods

### `.first()`

Returns the first element of the list.

**Returns:** `T` - The first element

**Throws:** Error if the list is empty

**Example:**

```stratum
[1, 2, 3].first()      // 1
["a", "b"].first()     // "a"

// Handle potentially empty lists
let list = get_items()
if !list.is_empty() {
    println(list.first())
}
```

---

### `.last()`

Returns the last element of the list.

**Returns:** `T` - The last element

**Throws:** Error if the list is empty

**Example:**

```stratum
[1, 2, 3].last()       // 3
["a", "b"].last()      // "b"

// Handle potentially empty lists
let list = get_items()
if !list.is_empty() {
    println(list.last())
}
```

---

## Mutation Methods

### `.push(value)`

Appends an element to the end of the list. Modifies the list in-place.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `T` | The value to append |

**Returns:** `Null`

**Example:**

```stratum
let list = [1, 2, 3]
list.push(4)
println(list)  // [1, 2, 3, 4]

// Chain multiple pushes
list.push(5)
list.push(6)
println(list)  // [1, 2, 3, 4, 5, 6]
```

---

### `.pop()`

Removes and returns the last element of the list. Modifies the list in-place.

**Returns:** `T` - The removed element

**Throws:** Error if the list is empty

**Example:**

```stratum
let list = [1, 2, 3]
let last = list.pop()
println(last)   // 3
println(list)   // [1, 2]

// Use in a loop
while !list.is_empty() {
    println(list.pop())
}
// Prints: 2, 1
```

---

### `.reverse()`

Reverses the order of elements in the list. Modifies the list in-place.

**Returns:** `Null`

**Example:**

```stratum
let list = [1, 2, 3, 4, 5]
list.reverse()
println(list)  // [5, 4, 3, 2, 1]

let words = ["hello", "world"]
words.reverse()
println(words)  // ["world", "hello"]
```

---

## Search Methods

### `.contains(value)`

Checks if the list contains a specific value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `T` | The value to search for |

**Returns:** `Bool` - `true` if the value is found, `false` otherwise

**Example:**

```stratum
let list = [1, 2, 3, 4, 5]
list.contains(3)      // true
list.contains(10)     // false

let words = ["apple", "banana", "cherry"]
words.contains("banana")  // true
words.contains("grape")   // false
```

---

## Conversion Methods

### `.join(separator)`

Joins all elements into a single string, with a separator between each element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `separator` | `String` | The string to place between elements |

**Returns:** `String` - The joined string

**Example:**

```stratum
["a", "b", "c"].join(", ")       // "a, b, c"
[1, 2, 3].join("-")              // "1-2-3"
["hello", "world"].join(" ")     // "hello world"
[].join(", ")                    // ""
["only"].join(", ")              // "only"

// Create a CSV line
let row = ["Alice", "30", "Engineer"]
println(row.join(","))  // "Alice,30,Engineer"
```

---

## Higher-Order Methods

### `.map(fn)`

Applies a function to each element and returns a new list with the results.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `(T) -> U` | A function that transforms each element |

**Returns:** `List[U]` - A new list containing the transformed elements

**Example:**

```stratum
// Double each number
let numbers = [1, 2, 3, 4, 5]
let doubled = numbers.map(|x: Int| -> Int { x * 2 })
println(doubled)  // [2, 4, 6, 8, 10]

// Convert to strings
let strings = numbers.map(|x: Int| -> String { str(x) })
println(strings)  // ["1", "2", "3", "4", "5"]

// Extract fields from structs
let users = [
    {name: "Alice", age: 30},
    {name: "Bob", age: 25}
]
let names = users.map(|u| { u.name })
println(names)  // ["Alice", "Bob"]
```

---

### `.filter(fn)`

Returns a new list containing only elements that satisfy the predicate.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `(T) -> Bool` | A predicate function that returns `true` for elements to keep |

**Returns:** `List[T]` - A new list containing only matching elements

**Example:**

```stratum
// Keep only even numbers
let numbers = [1, 2, 3, 4, 5, 6]
let evens = numbers.filter(|x: Int| -> Bool { x % 2 == 0 })
println(evens)  // [2, 4, 6]

// Filter by condition
let words = ["apple", "banana", "apricot", "cherry"]
let a_words = words.filter(|w: String| -> Bool { w.starts_with("a") })
println(a_words)  // ["apple", "apricot"]

// Chain with map
let result = numbers
    .filter(|x: Int| -> Bool { x > 2 })
    .map(|x: Int| -> Int { x * 10 })
println(result)  // [30, 40, 50, 60]
```

---

### `.reduce(fn, initial?)`

Reduces the list to a single value by repeatedly applying a function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `(Acc, T) -> Acc` | A function that combines the accumulator with each element |
| `initial` | `Acc?` | Optional initial value. If not provided, uses the first element. |

**Returns:** `Acc` - The final accumulated value

**Throws:** Error if the list is empty and no initial value is provided

**Example:**

```stratum
// Sum all numbers
let numbers = [1, 2, 3, 4, 5]
let sum = numbers.reduce(|acc: Int, x: Int| -> Int { acc + x }, 0)
println(sum)  // 15

// Product of all numbers
let product = numbers.reduce(|acc: Int, x: Int| -> Int { acc * x }, 1)
println(product)  // 120

// Without initial value (uses first element)
let sum2 = numbers.reduce(|acc: Int, x: Int| -> Int { acc + x })
println(sum2)  // 15

// Find maximum
let max = numbers.reduce(|acc: Int, x: Int| -> Int {
    if x > acc { x } else { acc }
})
println(max)  // 5

// Concatenate strings
let words = ["Hello", " ", "World"]
let sentence = words.reduce(|acc: String, w: String| -> String { acc + w }, "")
println(sentence)  // "Hello World"
```

---

### `.find(fn)`

Returns the first element that satisfies the predicate, or `null` if none found.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `(T) -> Bool` | A predicate function |

**Returns:** `T?` - The first matching element, or `null`

**Example:**

```stratum
let numbers = [1, 2, 3, 4, 5]

// Find first even number
let first_even = numbers.find(|x: Int| -> Bool { x % 2 == 0 })
println(first_even)  // 2

// Find first number greater than 10
let large = numbers.find(|x: Int| -> Bool { x > 10 })
println(large)  // null

// Handle null result
let result = numbers.find(|x: Int| -> Bool { x > 3 })
if result != null {
    println("Found: " + str(result))
}

// With null coalescing
let found = numbers.find(|x: Int| -> Bool { x > 10 }) ?? -1
println(found)  // -1
```

---

### `.sort(comparator?)`

Returns a new sorted list. Does not modify the original list.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `comparator` | `((T, T) -> Int)?` | Optional comparison function. Should return negative if first < second, positive if first > second, zero if equal. |

**Returns:** `List[T]` - A new sorted list

**Default behavior:** Without a comparator, sorts integers, floats, and strings in ascending order.

**Example:**

```stratum
// Default sort (ascending)
let numbers = [3, 1, 4, 1, 5, 9, 2, 6]
let sorted = numbers.sort()
println(sorted)   // [1, 1, 2, 3, 4, 5, 6, 9]
println(numbers)  // [3, 1, 4, 1, 5, 9, 2, 6] (unchanged)

// Sort strings alphabetically
let words = ["banana", "apple", "cherry"]
let sorted_words = words.sort()
println(sorted_words)  // ["apple", "banana", "cherry"]

// Custom comparator: descending order
let desc = numbers.sort(|a: Int, b: Int| -> Int { b - a })
println(desc)  // [9, 6, 5, 4, 3, 2, 1, 1]

// Sort by string length
let by_length = words.sort(|a: String, b: String| -> Int {
    a.len() - b.len()
})
println(by_length)  // ["apple", "banana", "cherry"]

// Sort structs by field
let users = [
    {name: "Charlie", age: 35},
    {name: "Alice", age: 30},
    {name: "Bob", age: 25}
]
let by_age = users.sort(|a, b| { a.age - b.age })
// [{name: "Bob", age: 25}, {name: "Alice", age: 30}, {name: "Charlie", age: 35}]
```

---

### `.enumerate()`

Returns a list of (index, value) pairs.

**Returns:** `List[(Int, T)]` - List of tuples containing index and value

**Example:**

```stratum
let items = ["a", "b", "c"]
let indexed = items.enumerate()
// [(0, "a"), (1, "b"), (2, "c")]

// Useful in loops
for (idx, value) in items.enumerate() {
    println(str(idx) + ": " + value)
}
// 0: a
// 1: b
// 2: c
```

---

### `.chunk(size)`

Splits the list into chunks of the specified size.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `size` | `Int` | Maximum size of each chunk |

**Returns:** `List[List[T]]` - List of chunks

**Example:**

```stratum
let numbers = [1, 2, 3, 4, 5, 6, 7]
let chunks = numbers.chunk(3)
// [[1, 2, 3], [4, 5, 6], [7]]

let even_chunks = [1, 2, 3, 4, 5, 6].chunk(2)
// [[1, 2], [3, 4], [5, 6]]
```

---

### `.window(size)`

Creates sliding windows over the list.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `size` | `Int` | Size of each window |

**Returns:** `List[List[T]]` - List of overlapping windows

**Aliases:** `.windows(size)`

**Example:**

```stratum
let numbers = [1, 2, 3, 4, 5]
let windows = numbers.window(3)
// [[1, 2, 3], [2, 3, 4], [3, 4, 5]]

// Useful for rolling calculations
let pairs = [1, 2, 3, 4].window(2)
// [[1, 2], [2, 3], [3, 4]]
```

---

### `.unique()`

Returns a new list with duplicates removed, preserving original order.

**Returns:** `List[T]` - List with only unique elements

**Aliases:** `.distinct()`

**Example:**

```stratum
let values = [1, 2, 2, 3, 1, 4, 3, 5]
let unique = values.unique()
// [1, 2, 3, 4, 5]

let words = ["apple", "banana", "apple", "cherry"]
let unique_words = words.unique()
// ["apple", "banana", "cherry"]
```

---

### `.group_by(fn)`

Groups elements into a Map by a key function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `(T) -> K` | Function that returns the grouping key |

**Returns:** `Map[K, List[T]]` - Map from keys to groups of elements

**Example:**

```stratum
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// Group by even/odd
let by_parity = numbers.group_by(|n| n % 2 == 0)
// {true: [2, 4, 6, 8, 10], false: [1, 3, 5, 7, 9]}

// Group strings by first letter
let words = ["apple", "apricot", "banana", "blueberry", "cherry"]
let by_letter = words.group_by(|w| w[0])
// {"a": ["apple", "apricot"], "b": ["banana", "blueberry"], "c": ["cherry"]}

// Group by length
let by_length = words.group_by(|w| w.len())
// {5: ["apple"], 7: ["apricot", "cherry"], 6: ["banana"], 9: ["blueberry"]}
```

---

## Iteration

Lists can be used directly in `for` loops:

```stratum
let fruits = ["apple", "banana", "cherry"]

// Iterate over elements
for fruit in fruits {
    println(fruit)
}

// With index using range
for i in range(0, fruits.len()) {
    println(str(i) + ": " + fruits[i])
}
```

---

## Common Patterns

### Checking for existence

```stratum
let users = ["alice", "bob", "charlie"]

if users.contains("alice") {
    println("Alice found!")
}
```

### Transforming data

```stratum
let prices = [10.0, 20.0, 30.0]
let with_tax = prices.map(|p: Float| -> Float { p * 1.08 })
```

### Filtering and counting

```stratum
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let evens = numbers.filter(|n: Int| -> Bool { n % 2 == 0 })
println("Even count: " + str(evens.len()))  // Even count: 5
```

### Aggregating values

```stratum
let scores = [85, 92, 78, 95, 88]
let total = scores.reduce(|acc: Int, s: Int| -> Int { acc + s }, 0)
let average = float(total) / float(scores.len())
println("Average: " + str(average))  // Average: 87.6
```

### Building strings

```stratum
let parts = ["usr", "local", "bin"]
let path = "/" + parts.join("/")
println(path)  // /usr/local/bin
```

---

## See Also

- [Global Functions](globals.md) - `len()` for getting list length
- [String](string.md) - `split()` to create lists from strings
- [Map](map.md) - Key-value collection type
