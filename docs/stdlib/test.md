# Test

A testing framework for Stratum with Jest/Mocha-style assertions and mocking.

## Overview

The `Test` namespace provides a testing framework inspired by JavaScript testing libraries like Jest and Mocha. It offers:

- **Assertion matchers** via `Test.expect()` with fluent API
- **Test organization** with `Test.describe()` and `Test.it()`
- **Mocking support** for isolating code under test
- **Test control** methods for skipping, pending, and explicit failure

---

## Assertions

### `Test.expect(value)`

Creates an expectation object that can be chained with matchers.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `any` | The value to test |

**Returns:** `Expectation` - Object with matcher methods

**Example:**

```stratum
Test.expect(2 + 2).to_be(4)
Test.expect("hello").to_equal("hello")
Test.expect([1, 2, 3]).to_contain(2)
```

---

## Matchers

All matchers are called on an `Expectation` object returned by `Test.expect()`.

### `.to_be(expected)`

Strict equality check using `===` semantics.

**Aliases:** `.toBe(expected)`

**Example:**

```stratum
Test.expect(5).to_be(5)       // Pass
Test.expect("a").to_be("a")   // Pass
Test.expect(5).to_be("5")     // Fail (different types)
```

---

### `.to_equal(expected)`

Deep equality check for complex values.

**Aliases:** `.toEqual(expected)`

**Example:**

```stratum
Test.expect({a: 1, b: 2}).to_equal({a: 1, b: 2})  // Pass
Test.expect([1, 2, 3]).to_equal([1, 2, 3])         // Pass
```

---

### `.to_be_truthy()`

Checks if the value is truthy.

**Aliases:** `.toBeTruthy()`

**Example:**

```stratum
Test.expect(1).to_be_truthy()        // Pass
Test.expect("hello").to_be_truthy()  // Pass
Test.expect(true).to_be_truthy()     // Pass
Test.expect(0).to_be_truthy()        // Fail
Test.expect("").to_be_truthy()       // Fail
Test.expect(null).to_be_truthy()     // Fail
```

---

### `.to_be_falsy()`

Checks if the value is falsy.

**Aliases:** `.toBeFalsy()`

**Example:**

```stratum
Test.expect(0).to_be_falsy()      // Pass
Test.expect("").to_be_falsy()     // Pass
Test.expect(null).to_be_falsy()   // Pass
Test.expect(false).to_be_falsy()  // Pass
Test.expect(1).to_be_falsy()      // Fail
```

---

### `.to_be_null()`

Checks if the value is exactly `null`.

**Aliases:** `.toBeNull()`

**Example:**

```stratum
Test.expect(null).to_be_null()   // Pass
Test.expect(0).to_be_null()      // Fail
Test.expect("").to_be_null()     // Fail
```

---

### `.to_be_type(type_name)`

Checks if the value is of the specified type.

**Aliases:** `.toBeType(type_name)`

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `type_name` | `String` | Expected type: "int", "float", "string", "bool", "list", "map", etc. |

**Example:**

```stratum
Test.expect(42).to_be_type("int")
Test.expect(3.14).to_be_type("float")
Test.expect("hello").to_be_type("string")
Test.expect([1, 2, 3]).to_be_type("list")
Test.expect({a: 1}).to_be_type("map")
```

---

### `.to_be_greater_than(expected)`

Checks if the value is greater than the expected value.

**Aliases:** `.toBeGreaterThan(expected)`

**Example:**

```stratum
Test.expect(10).to_be_greater_than(5)   // Pass
Test.expect(10).to_be_greater_than(10)  // Fail
```

---

### `.to_be_less_than(expected)`

Checks if the value is less than the expected value.

**Aliases:** `.toBeLessThan(expected)`

**Example:**

```stratum
Test.expect(5).to_be_less_than(10)   // Pass
Test.expect(10).to_be_less_than(10)  // Fail
```

---

### `.to_be_greater_than_or_equal(expected)`

Checks if the value is greater than or equal to the expected value.

**Aliases:** `.toBeGreaterThanOrEqual(expected)`

**Example:**

```stratum
Test.expect(10).to_be_greater_than_or_equal(10)  // Pass
Test.expect(15).to_be_greater_than_or_equal(10)  // Pass
```

---

### `.to_be_less_than_or_equal(expected)`

Checks if the value is less than or equal to the expected value.

**Aliases:** `.toBeLessThanOrEqual(expected)`

**Example:**

```stratum
Test.expect(10).to_be_less_than_or_equal(10)  // Pass
Test.expect(5).to_be_less_than_or_equal(10)   // Pass
```

---

### `.to_contain(element)`

Checks if a collection contains the specified element.

**Aliases:** `.toContain(element)`

**Example:**

```stratum
Test.expect([1, 2, 3]).to_contain(2)       // Pass
Test.expect("hello world").to_contain("world")  // Pass
Test.expect([1, 2, 3]).to_contain(5)       // Fail
```

---

## Test Control

### `Test.fail(message?)`

Immediately fails the current test with an optional message.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String?` | Optional failure message |

**Example:**

```stratum
if some_condition {
    Test.fail("Unexpected condition was met")
}

// Unconditional failure
Test.fail()
```

---

### `Test.skip(message?)`

Marks the current test as skipped. The test will not run but will be reported as skipped.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String?` | Optional reason for skipping |

**Example:**

```stratum
Test.skip("Feature not yet implemented")

// Skip without reason
Test.skip()
```

---

### `Test.pending(message?)`

Marks the current test as pending (not yet implemented).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `String?` | Optional description |

**Example:**

```stratum
Test.pending("TODO: Add validation tests")
```

---

## Mocking

### `Test.mock(return_value?)`

Creates a mock function that records calls and returns a configured value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `return_value` | `any?` | Value the mock returns when called (default: `null`) |

**Returns:** `Map` - Mock object with the following properties:
- `__is_mock`: `true` - Marker identifying this as a mock
- `return_value`: The configured return value
- `calls`: List of argument lists from each call
- `call_count`: Number of times called

**Example:**

```stratum
// Create a mock that returns 42
let mock_fn = Test.mock(42)

// Check properties after calls
println(mock_fn.call_count)  // 0
println(mock_fn.calls)       // []

// After using the mock
println(mock_fn.call_count)  // Number of calls
println(mock_fn.calls)       // [[arg1, arg2], [arg3], ...]
```

---

### `Test.spy(fn?)`

Creates a spy that wraps a function and tracks calls.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `fn` | `Function?` | Optional function to wrap |

**Returns:** `Map` - Spy object with the following properties:
- `__is_spy`: `true` - Marker identifying this as a spy
- `wrapped`: The wrapped function (or `null`)
- `calls`: List of argument lists from each call
- `call_count`: Number of times called

**Example:**

```stratum
// Create a spy on an existing function
let original_fn = |x| x * 2
let spy = Test.spy(original_fn)

// Use the spy like the original function
// It will track calls while delegating to the original

println(spy.call_count)
println(spy.calls)
```

---

## Test Organization

While the Test namespace provides assertion primitives, tests in Stratum can be organized using standard language constructs:

```stratum
// test_math.strat

fx describe(name: String, tests: () -> ()) {
    println("Suite: " + name)
    tests()
}

fx it(name: String, test: () -> ()) {
    try {
        test()
        println("  PASS: " + name)
    } catch (e) {
        println("  FAIL: " + name + " - " + str(e))
    }
}

// Usage
describe("Math operations", || {
    it("should add numbers correctly", || {
        Test.expect(2 + 2).to_be(4)
        Test.expect(0 + 0).to_be(0)
    })

    it("should multiply numbers", || {
        Test.expect(3 * 4).to_be(12)
    })
})
```

---

## Complete Example

```stratum
// test_user_service.strat

fx test_user_validation() {
    // Test valid user
    let valid_user = {name: "Alice", age: 25}
    Test.expect(validate_user(valid_user)).to_be(true)

    // Test invalid age
    let invalid_age = {name: "Bob", age: -5}
    Test.expect(validate_user(invalid_age)).to_be(false)

    // Test missing name
    let missing_name = {age: 30}
    Test.expect(validate_user(missing_name)).to_be(false)
}

fx test_user_repository() {
    // Create mock database
    let mock_db = Test.mock([{id: 1, name: "Alice"}])

    // Test that repository uses the database correctly
    let repo = UserRepository.new(mock_db)
    let user = repo.find_by_id(1)

    Test.expect(user).to_equal({id: 1, name: "Alice"})
    Test.expect(mock_db.call_count).to_be_greater_than(0)
}

fx test_edge_cases() {
    // Test empty list
    Test.expect([].len()).to_be(0)

    // Test null handling
    let result = process_nullable(null)
    Test.expect(result).to_be_null()

    // Test type checking
    Test.expect(42).to_be_type("int")
    Test.expect("hello").to_be_type("string")
}

// Run tests
test_user_validation()
test_user_repository()
test_edge_cases()

println("All tests passed!")
```

---

## See Also

- [Async](async.md) - Asynchronous testing patterns
- [Log](log.md) - Logging for test debugging
