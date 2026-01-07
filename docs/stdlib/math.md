# Math

Mathematical constants and functions for numeric operations.

## Overview

The `Math` namespace provides fundamental mathematical operations including trigonometry, exponentials, logarithms, rounding, and utility functions. All functions accept both `Int` and `Float` arguments where numeric input is expected.

Constants are accessed as properties (e.g., `Math.PI`), while functions are called with parentheses (e.g., `Math.sqrt(16)`).

---

## Constants

### `Math.PI`

The mathematical constant π (pi), the ratio of a circle's circumference to its diameter.

**Type:** `Float`

**Value:** `3.141592653589793`

---

### `Math.E`

Euler's number *e*, the base of natural logarithms.

**Type:** `Float`

**Value:** `2.718281828459045`

---

### `Math.TAU`

The mathematical constant τ (tau), equal to 2π. Represents a full turn in radians.

**Type:** `Float`

**Value:** `6.283185307179586`

---

### `Math.INFINITY`

Positive infinity. Result of operations like `1.0 / 0.0`.

**Type:** `Float`

**Value:** `inf`

---

### `Math.NEG_INFINITY`

Negative infinity. Result of operations like `-1.0 / 0.0`.

**Type:** `Float`

**Value:** `-inf`

---

### `Math.NAN`

Not a Number. Result of undefined operations like `0.0 / 0.0`.

**Type:** `Float`

**Value:** `nan`

**Note:** `NaN` is not equal to itself. Use `Math.is_nan()` to check for NaN values.

---

## Basic Functions

### `Math.abs(x)`

Returns the absolute value of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int | Float` - The absolute value (same type as input)

**Example:**

```stratum
Math.abs(-5)      // 5
Math.abs(3.14)    // 3.14
Math.abs(-2.5)    // 2.5
Math.abs(0)       // 0
```

---

### `Math.floor(x)`

Returns the largest integer less than or equal to a number (rounds toward negative infinity).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int` - The floor value

**Example:**

```stratum
Math.floor(3.7)   // 3
Math.floor(3.2)   // 3
Math.floor(-2.3)  // -3 (toward negative infinity)
Math.floor(5)     // 5
```

---

### `Math.ceil(x)`

Returns the smallest integer greater than or equal to a number (rounds toward positive infinity).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int` - The ceiling value

**Example:**

```stratum
Math.ceil(3.2)    // 4
Math.ceil(3.7)    // 4
Math.ceil(-2.7)   // -2 (toward positive infinity)
Math.ceil(5)      // 5
```

---

### `Math.round(x)`

Rounds a number to the nearest integer. Ties round away from zero.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int` - The rounded value

**Example:**

```stratum
Math.round(3.4)   // 3
Math.round(3.5)   // 4
Math.round(3.6)   // 4
Math.round(-2.5)  // -3 (away from zero)
```

---

### `Math.trunc(x)`

Truncates a number by removing its fractional part (rounds toward zero).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int` - The truncated value

**Example:**

```stratum
Math.trunc(3.7)   // 3
Math.trunc(-2.7)  // -2 (toward zero, unlike floor)
Math.trunc(5.9)   // 5
```

---

### `Math.sign(x)`

Returns the sign of a number: -1, 0, or 1.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Int` - The sign (-1, 0, or 1); returns `NaN` if input is `NaN`

**Aliases:** `Math.signum(x)`

**Example:**

```stratum
Math.sign(-42)    // -1
Math.sign(0)      // 0
Math.sign(100)    // 1
Math.sign(-3.14)  // -1
```

---

### `Math.fract(x)`

Returns the fractional part of a number (the part after the decimal point).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input number |

**Returns:** `Float` - The fractional part

**Example:**

```stratum
Math.fract(3.75)   // 0.75
Math.fract(-2.25)  // -0.25
Math.fract(5)      // 0.0
Math.fract(1.5)    // 0.5
```

---

## Trigonometric Functions

All trigonometric functions work with radians. Use `Math.to_radians()` to convert degrees to radians.

### `Math.sin(x)`

Returns the sine of an angle in radians.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Angle in radians |

**Returns:** `Float` - The sine value (range: -1 to 1)

**Example:**

```stratum
Math.sin(0)              // 0.0
Math.sin(Math.PI / 2)    // 1.0
Math.sin(Math.PI)        // ~0.0 (very small due to floating point)
```

---

### `Math.cos(x)`

Returns the cosine of an angle in radians.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Angle in radians |

**Returns:** `Float` - The cosine value (range: -1 to 1)

**Example:**

```stratum
Math.cos(0)              // 1.0
Math.cos(Math.PI / 2)    // ~0.0
Math.cos(Math.PI)        // -1.0
```

---

### `Math.tan(x)`

Returns the tangent of an angle in radians.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Angle in radians |

**Returns:** `Float` - The tangent value

**Example:**

```stratum
Math.tan(0)              // 0.0
Math.tan(Math.PI / 4)    // ~1.0
```

---

### `Math.asin(x)`

Returns the arcsine (inverse sine) of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Value in range -1 to 1 |

**Returns:** `Float` - Angle in radians (range: -π/2 to π/2)

**Example:**

```stratum
Math.asin(0)    // 0.0
Math.asin(1)    // ~1.5707963 (π/2)
Math.asin(0.5)  // ~0.5235987 (π/6)
```

---

### `Math.acos(x)`

Returns the arccosine (inverse cosine) of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Value in range -1 to 1 |

**Returns:** `Float` - Angle in radians (range: 0 to π)

**Example:**

```stratum
Math.acos(1)    // 0.0
Math.acos(0)    // ~1.5707963 (π/2)
Math.acos(-1)   // ~3.1415926 (π)
```

---

### `Math.atan(x)`

Returns the arctangent (inverse tangent) of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Any numeric value |

**Returns:** `Float` - Angle in radians (range: -π/2 to π/2)

**Example:**

```stratum
Math.atan(0)    // 0.0
Math.atan(1)    // ~0.7853981 (π/4)
Math.atan(-1)   // ~-0.7853981
```

---

### `Math.atan2(y, x)`

Returns the arctangent of the quotient y/x, using the signs of both arguments to determine the quadrant. This is the angle in radians between the positive x-axis and the point (x, y).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `y` | `Int \| Float` | The y coordinate |
| `x` | `Int \| Float` | The x coordinate |

**Returns:** `Float` - Angle in radians (range: -π to π)

**Example:**

```stratum
Math.atan2(1, 1)    // ~0.7853981 (π/4, first quadrant)
Math.atan2(1, -1)   // ~2.3561944 (3π/4, second quadrant)
Math.atan2(-1, -1)  // ~-2.3561944 (-3π/4, third quadrant)
Math.atan2(-1, 1)   // ~-0.7853981 (-π/4, fourth quadrant)
```

---

### `Math.sinh(x)`

Returns the hyperbolic sine of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input value |

**Returns:** `Float` - The hyperbolic sine

**Example:**

```stratum
Math.sinh(0)    // 0.0
Math.sinh(1)    // ~1.1752011
```

---

### `Math.cosh(x)`

Returns the hyperbolic cosine of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input value |

**Returns:** `Float` - The hyperbolic cosine

**Example:**

```stratum
Math.cosh(0)    // 1.0
Math.cosh(1)    // ~1.5430806
```

---

### `Math.tanh(x)`

Returns the hyperbolic tangent of a value.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The input value |

**Returns:** `Float` - The hyperbolic tangent (range: -1 to 1)

**Example:**

```stratum
Math.tanh(0)    // 0.0
Math.tanh(1)    // ~0.7615941
```

---

## Exponential and Logarithmic Functions

### `Math.exp(x)`

Returns *e* raised to the power of x (e^x).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The exponent |

**Returns:** `Float` - e^x

**Example:**

```stratum
Math.exp(0)    // 1.0
Math.exp(1)    // ~2.7182818 (e)
Math.exp(2)    // ~7.3890560
```

---

### `Math.exp2(x)`

Returns 2 raised to the power of x (2^x).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The exponent |

**Returns:** `Float` - 2^x

**Example:**

```stratum
Math.exp2(0)    // 1.0
Math.exp2(3)    // 8.0
Math.exp2(10)   // 1024.0
```

---

### `Math.ln(x)`

Returns the natural logarithm (base *e*) of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | A positive number |

**Returns:** `Float` - The natural logarithm

**Aliases:** `Math.log(x)`

**Note:** Returns `-inf` for 0, `NaN` for negative numbers.

**Example:**

```stratum
Math.ln(1)          // 0.0
Math.ln(Math.E)     // 1.0
Math.ln(10)         // ~2.3025850
```

---

### `Math.log2(x)`

Returns the base-2 logarithm of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | A positive number |

**Returns:** `Float` - The base-2 logarithm

**Example:**

```stratum
Math.log2(1)      // 0.0
Math.log2(2)      // 1.0
Math.log2(8)      // 3.0
Math.log2(1024)   // 10.0
```

---

### `Math.log10(x)`

Returns the base-10 logarithm of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | A positive number |

**Returns:** `Float` - The base-10 logarithm

**Example:**

```stratum
Math.log10(1)      // 0.0
Math.log10(10)     // 1.0
Math.log10(100)    // 2.0
Math.log10(1000)   // 3.0
```

---

### `Math.pow(base, exp)`

Returns base raised to the power of exp.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `base` | `Int \| Float` | The base number |
| `exp` | `Int \| Float` | The exponent |

**Returns:** `Float` - base^exp

**Example:**

```stratum
Math.pow(2, 3)      // 8.0
Math.pow(2, 0.5)    // ~1.4142135 (square root of 2)
Math.pow(10, -2)    // 0.01
Math.pow(4, 0.5)    // 2.0
```

---

### `Math.sqrt(x)`

Returns the square root of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | A non-negative number |

**Returns:** `Float` - The square root

**Note:** Returns `NaN` for negative numbers.

**Example:**

```stratum
Math.sqrt(4)     // 2.0
Math.sqrt(16)    // 4.0
Math.sqrt(2)     // ~1.4142135
Math.sqrt(0)     // 0.0
```

---

### `Math.cbrt(x)`

Returns the cube root of a number.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | Any number (including negative) |

**Returns:** `Float` - The cube root

**Example:**

```stratum
Math.cbrt(8)     // 2.0
Math.cbrt(27)    // 3.0
Math.cbrt(-8)    // -2.0
Math.cbrt(1000)  // 10.0
```

---

## Utility Functions

### `Math.min(a, b, ...)`

Returns the smallest of the given values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `a, b, ...` | `Int \| Float` | One or more numbers to compare |

**Returns:** `Int | Float` - The minimum value (preserves Int type when possible)

**Example:**

```stratum
Math.min(3, 1, 4)       // 1
Math.min(-5, 0, 5)      // -5
Math.min(3.14, 2.71)    // 2.71
Math.min(10)            // 10
```

---

### `Math.max(a, b, ...)`

Returns the largest of the given values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `a, b, ...` | `Int \| Float` | One or more numbers to compare |

**Returns:** `Int | Float` - The maximum value (preserves Int type when possible)

**Example:**

```stratum
Math.max(3, 1, 4)       // 4
Math.max(-5, 0, 5)      // 5
Math.max(3.14, 2.71)    // 3.14
Math.max(10)            // 10
```

---

### `Math.clamp(value, min, max)`

Constrains a value to lie within a specified range.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Int \| Float` | The value to clamp |
| `min` | `Int \| Float` | The minimum bound |
| `max` | `Int \| Float` | The maximum bound |

**Returns:** `Int | Float` - The clamped value

**Throws:** Error if `min > max`

**Example:**

```stratum
Math.clamp(5, 0, 10)    // 5 (within range)
Math.clamp(-5, 0, 10)   // 0 (below min)
Math.clamp(15, 0, 10)   // 10 (above max)
Math.clamp(0.5, 0, 1)   // 0.5
```

---

### `Math.hypot(x, y)`

Returns the hypotenuse of a right triangle, equivalent to `sqrt(x² + y²)`. This is computed in a way that avoids overflow for large values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | First side length |
| `y` | `Int \| Float` | Second side length |

**Returns:** `Float` - The hypotenuse

**Example:**

```stratum
Math.hypot(3, 4)    // 5.0
Math.hypot(5, 12)   // 13.0
Math.hypot(1, 1)    // ~1.4142135
```

---

## Angle Conversion Functions

### `Math.to_degrees(radians)`

Converts an angle from radians to degrees.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `radians` | `Int \| Float` | Angle in radians |

**Returns:** `Float` - Angle in degrees

**Aliases:** `Math.degrees(radians)`

**Example:**

```stratum
Math.to_degrees(Math.PI)        // 180.0
Math.to_degrees(Math.PI / 2)    // 90.0
Math.to_degrees(Math.TAU)       // 360.0
```

---

### `Math.to_radians(degrees)`

Converts an angle from degrees to radians.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `degrees` | `Int \| Float` | Angle in degrees |

**Returns:** `Float` - Angle in radians

**Aliases:** `Math.radians(degrees)`

**Example:**

```stratum
Math.to_radians(180)    // ~3.1415926 (π)
Math.to_radians(90)     // ~1.5707963 (π/2)
Math.to_radians(360)    // ~6.2831853 (τ)
```

---

## Validation Functions

### `Math.is_nan(x)`

Checks if a value is NaN (Not a Number).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The value to check |

**Returns:** `Bool` - `true` if the value is NaN, `false` otherwise

**Note:** Integers are never NaN; this always returns `false` for Int values.

**Example:**

```stratum
Math.is_nan(Math.NAN)         // true
Math.is_nan(0.0 / 0.0)        // true
Math.is_nan(Math.sqrt(-1))    // true
Math.is_nan(42)               // false
Math.is_nan(3.14)             // false
```

---

### `Math.is_infinite(x)`

Checks if a value is positive or negative infinity.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The value to check |

**Returns:** `Bool` - `true` if the value is infinite, `false` otherwise

**Note:** Integers are never infinite; this always returns `false` for Int values.

**Example:**

```stratum
Math.is_infinite(Math.INFINITY)       // true
Math.is_infinite(Math.NEG_INFINITY)   // true
Math.is_infinite(1.0 / 0.0)           // true
Math.is_infinite(42)                  // false
Math.is_infinite(3.14)                // false
```

---

### `Math.is_finite(x)`

Checks if a value is finite (not infinity and not NaN).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `x` | `Int \| Float` | The value to check |

**Returns:** `Bool` - `true` if the value is finite, `false` otherwise

**Note:** Integers are always finite; this always returns `true` for Int values.

**Example:**

```stratum
Math.is_finite(42)              // true
Math.is_finite(3.14)            // true
Math.is_finite(Math.INFINITY)   // false
Math.is_finite(Math.NAN)        // false
```

---

## See Also

- [Global Functions](globals.md) - Type conversions like `int()` and `float()`
- [Random](random.md) - Random number generation
