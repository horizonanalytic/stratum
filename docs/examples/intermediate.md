# Intermediate Examples

These examples demonstrate common patterns and standard library usage in Stratum.

## Temperature Converter

Convert between Celsius, Fahrenheit, and Kelvin temperature scales.

```stratum
// Temperature Converter
// Convert between Celsius, Fahrenheit, and Kelvin

// Conversion functions
fx celsius_to_fahrenheit(c: Float) -> Float {
    c * 9.0 / 5.0 + 32.0
}

fx celsius_to_kelvin(c: Float) -> Float {
    c + 273.15
}

fx fahrenheit_to_celsius(f: Float) -> Float {
    (f - 32.0) * 5.0 / 9.0
}

fx fahrenheit_to_kelvin(f: Float) -> Float {
    fahrenheit_to_celsius(f) + 273.15
}

fx kelvin_to_celsius(k: Float) -> Float {
    k - 273.15
}

fx kelvin_to_fahrenheit(k: Float) -> Float {
    celsius_to_fahrenheit(kelvin_to_celsius(k))
}

fx main() {
    println("Temperature Converter Tests:");

    // Test freezing point of water
    let freezing_c = 0.0;
    assert(celsius_to_fahrenheit(freezing_c) == 32.0);
    assert(celsius_to_kelvin(freezing_c) == 273.15);

    // Test boiling point of water
    let boiling_c = 100.0;
    assert(celsius_to_fahrenheit(boiling_c) == 212.0);
    assert(celsius_to_kelvin(boiling_c) == 373.15);

    // Test body temperature (Fahrenheit to Celsius)
    let body_f = 98.6;
    let body_c = fahrenheit_to_celsius(body_f);
    assert(body_c > 36.9);
    assert(body_c < 37.1);

    // Test absolute zero
    let abs_zero_k = 0.0;
    let abs_zero_c = kelvin_to_celsius(abs_zero_k);
    assert(abs_zero_c == -273.15);

    // Test round-trip conversions
    let temp = 25.0;
    let round_trip = fahrenheit_to_celsius(celsius_to_fahrenheit(temp));
    assert(round_trip > 24.99);
    assert(round_trip < 25.01);

    println("All temperature conversion tests passed!");
}
```

**Key concepts:**
- Function composition (calling functions within functions)
- Float arithmetic
- Range-based assertions for floating-point comparisons
- Unit testing with `assert()`

---

## JSON API

Fetch and parse JSON data from a public REST API.

```stratum
// JSON API Example - Fetching data from a public API
// Uses JSONPlaceholder for testing

fx main() {
    print("Fetching data from JSONPlaceholder API...");

    let response = Http.get("https://jsonplaceholder.typicode.com/users/1");
    print("Response received!");

    // Parse JSON response
    let user = Json.parse(response.body);
    let name = user.name;
    let email = user.email;

    print("User: {name}");
    print("Email: {email}")
}
```

**Output:**
```
Fetching data from JSONPlaceholder API...
Response received!
User: Leanne Graham
Email: Sincere@april.biz
```

**Key concepts:**
- `Http.get()` for HTTP requests
- `Json.parse()` for parsing JSON strings
- Dynamic property access on parsed JSON
- String interpolation with `{variable}`

---

## Word Count

Count word frequencies in text using string processing and maps.

```stratum
// Word Count - Count word frequencies in text
// Demonstrates string processing and maps

fx main() {
    print("Word Frequency Analysis");
    print("=======================");

    // Sample text
    let text = "The quick brown fox jumps over the lazy dog. The dog was not amused by the fox. Quick thinking by the fox saved the day.";
    print("\nText: {text}");

    // Normalize and split
    let lower = text.to_lower();
    let normalized = lower.replace(".", "").replace(",", "");
    let words = normalized.split(" ");

    // Count words
    let counts = {"_init_": 0};
    for word in words {
        if word.len() > 0 {
            let current = counts.get(word) ?? 0;
            counts[word] = current + 1;
        }
    }

    // Print results
    print("\nWord frequencies:");
    let total = 0;
    for word in words {
        if word.len() > 0 {
            total = total + 1;
        }
    }
    print("Total words: {total}");

    let unique = counts.len();
    print("Unique words: {unique}")
}
```

**Output:**
```
Word Frequency Analysis
=======================

Text: The quick brown fox jumps over the lazy dog. The dog was not amused by the fox. Quick thinking by the fox saved the day.

Word frequencies:
Total words: 25
Unique words: 16
```

**Key concepts:**
- String methods: `to_lower()`, `replace()`, `split()`
- `for` loops for iteration
- Map operations: `get()`, index assignment `counts[word]`
- Null coalescing: `??` operator for default values
- String length with `len()`
