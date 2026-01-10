# Simple Examples

These examples demonstrate basic Stratum syntax and language features. Perfect for beginners learning the language.

## Hello World

The simplest Stratum program - prints a greeting to the console.

```stratum
// Hello World - The simplest Stratum program
// Run with: stratum run hello_world.strat

fx main() {
    print("Hello, Stratum!")
}
```

**Key concepts:**
- `fx` declares a function
- `main()` is the entry point
- `print()` outputs text to the console

---

## Variables

Demonstrates variable declaration and type inference.

```stratum
// Variables and Type Inference
// Stratum infers types automatically - you rarely need to write them

fx main() {
    // Basic types - all inferred
    let name = "Alice";              // String
    let age = 30;                    // Int
    let height = 5.7;                // Float
    let is_student = false;          // Bool

    // Collections
    let numbers = [1, 2, 3, 4, 5];           // List<Int>
    let scores = {"math": 95, "science": 87}; // Map<String, Int>

    // Print string values
    println("Name: Alice");
    println("Age: 30");
    println("Height: 5.7");
    println("Is student: false");
    println("Numbers: [1, 2, 3, 4, 5]");
    println("Scores: math=95, science=87");

    // Nullable types with ?
    let middle_name: String? = null;
    let display_name = middle_name ?? "N/A";
    println("Middle name: N/A");

    // Type inference verification
    assert(name == "Alice");
    assert(age == 30);
    assert(numbers.len() == 5);
    assert((scores.get("math") ?? 0) == 95);
}
```

**Key concepts:**
- `let` declares variables (mutable by default)
- Type inference - types are determined automatically
- Lists use `[]` syntax
- Maps use `{}` syntax
- Nullable types use `?` suffix
- `??` is the null coalescing operator

---

## FizzBuzz

A classic programming challenge demonstrating conditionals and loops.

```stratum
// FizzBuzz - A classic programming challenge
// Print numbers 1-100, but:
//   - Print "Fizz" for multiples of 3
//   - Print "Buzz" for multiples of 5
//   - Print "FizzBuzz" for multiples of both

fx main() {
    let i = 1;
    while i <= 20 {  // Just 20 for demo
        let result = if i % 15 == 0 {
            "FizzBuzz"
        } else if i % 3 == 0 {
            "Fizz"
        } else if i % 5 == 0 {
            "Buzz"
        } else {
            "."
        };
        print(result);
        i = i + 1;
    }
    println("");  // Final newline
}
```

**Key concepts:**
- `while` loops
- `if`/`else if`/`else` expressions
- `%` modulo operator
- Expressions return values (no `return` needed for last expression)

---

## Fibonacci

Calculate Fibonacci numbers using recursive and iterative approaches.

```stratum
// Fibonacci Sequence
// Calculate Fibonacci numbers using different approaches

// Recursive approach (simple but slow for large n)
fx fib_recursive(n: Int) -> Int {
    if n <= 1 { n } else { fib_recursive(n - 1) + fib_recursive(n - 2) }
}

// Iterative approach (efficient)
fx fib_iterative(n: Int) -> Int {
    if n <= 1 { n } else {
        let a = 0;
        let b = 1;
        let i = 2;
        let result = 0;
        while i <= n {
            result = a + b;
            a = b;
            b = result;
            i = i + 1;
        }
        b
    }
}

fx main() {
    println("Fibonacci Tests:");

    // Verify values using asserts
    assert(fib_iterative(0) == 0);
    assert(fib_iterative(1) == 1);
    assert(fib_iterative(5) == 5);
    assert(fib_iterative(10) == 55);

    // Test recursive version (for small numbers)
    assert(fib_recursive(6) == 8);
    assert(fib_recursive(7) == 13);

    println("All Fibonacci tests passed!");
}
```

**Key concepts:**
- Function parameters with type annotations (`n: Int`)
- Return types (`-> Int`)
- Recursion
- `assert()` for testing

---

## Factorial

Calculate factorials using multiple approaches including functional style.

```stratum
// Factorial Calculator
// Demonstrates recursion and iteration in Stratum

// Recursive factorial
fx factorial_recursive(n: Int) -> Int {
    if n <= 1 { 1 } else { n * factorial_recursive(n - 1) }
}

// Iterative factorial
fx factorial_iterative(n: Int) -> Int {
    let result = 1;
    let i = 2;
    while i <= n {
        result = result * i;
        i = i + 1;
    }
    result
}

// Using reduce (functional style)
fx factorial_functional(n: Int) -> Int {
    let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    nums.reduce(|acc: Int, x: Int| -> Int {
        if x <= n { acc * x } else { acc }
    }, 1)
}

fx main() {
    println("Factorial Tests:");

    // Test factorial_recursive
    assert(factorial_recursive(0) == 1);
    assert(factorial_recursive(1) == 1);
    assert(factorial_recursive(5) == 120);
    assert(factorial_recursive(7) == 5040);

    // Test factorial_iterative
    assert(factorial_iterative(0) == 1);
    assert(factorial_iterative(1) == 1);
    assert(factorial_iterative(5) == 120);
    assert(factorial_iterative(7) == 5040);
    assert(factorial_iterative(10) == 3628800);

    // Verify both methods match
    assert(factorial_recursive(6) == factorial_iterative(6));

    println("All factorial tests passed!");
}
```

**Key concepts:**
- Multiple function implementations
- `reduce()` for functional programming
- Lambda/closure syntax: `|params| -> ReturnType { body }`
- Comparing different algorithmic approaches
