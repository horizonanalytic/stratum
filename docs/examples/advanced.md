# Advanced Examples

These examples demonstrate real-world use cases and more complex Stratum applications.

## Todo App

A simple console-based todo list application.

```stratum
// Todo Application - Simple console version
// Demonstrates basic state management

fx main() {
    print("Todo App Demo");
    print("=============");

    // Simple todo list stored as strings
    let todos = ["Buy groceries", "Write code", "Read book"];

    print("\nMy Todos:");
    let i = 1;
    for todo in todos {
        print("  {i}. {todo}");
        i = i + 1;
    }
}
```

**Output:**
```
Todo App Demo
=============

My Todos:
  1. Buy groceries
  2. Write code
  3. Read book
```

**Key concepts:**
- List iteration with `for` loops
- Manual index tracking
- String interpolation in output

---

## Data Analysis

Basic statistical operations on numeric data.

```stratum
// Data Analysis - Basic statistical operations
// Demonstrates working with numeric data

fx main() {
    print("Data Analysis Demo");
    print("==================");

    // Sample data
    let values = [4.5, 3.2, 5.1, 4.8, 3.9, 5.5, 4.1];

    print("\nValues:");
    for v in values {
        print("  {v}");
    }

    // Calculate sum using a loop
    let sum = 0.0;
    for v in values {
        sum = sum + v;
    }
    print("\nSum: {sum}");

    // Calculate average
    let count = 7;
    let avg = sum / count;
    print("Average: {avg}")
}
```

**Output:**
```
Data Analysis Demo
==================

Values:
  4.5
  3.2
  5.1
  4.8
  3.9
  5.5
  4.1

Sum: 31.1
Average: 4.442857142857143
```

**Key concepts:**
- Working with float lists
- Accumulator pattern for sum calculation
- Basic statistical computations

---

## CSV Processor

Process and analyze tabular data.

```stratum
// CSV Processor - Data processing demo

fx main() {
    print("Data Processing Example");
    print("=======================");

    // Create simple list data
    let regions = ["North", "South", "East", "West"];
    let sales = [100, 75, 120, 50];

    print("\nRegions:");
    for r in regions {
        print("  - {r}");
    }

    print("\nSales:");
    for s in sales {
        print("  {s}");
    }

    // Calculate total
    let total = 0;
    for s in sales {
        total = total + s;
    }
    print("\nTotal sales: {total}");

    print("\nDone!")
}
```

**Output:**
```
Data Processing Example
=======================

Regions:
  - North
  - South
  - East
  - West

Sales:
  100
  75
  120
  50

Total sales: 345

Done!
```

**Key concepts:**
- Parallel lists for related data
- Multiple iterations over different data sets
- Aggregation calculations

---

## Web Scraper

Fetch and process data from web APIs.

```stratum
// Web Scraper - Fetch data from APIs
// Demonstrates HTTP requests and JSON parsing

fx main() {
    print("Web Data Fetcher");
    print("================");

    // Fetch a user from JSONPlaceholder
    print("\nFetching user data...");
    let response = Http.get("https://jsonplaceholder.typicode.com/users/1");
    let user = Json.parse(response.body);

    print("\nUser Info:");
    let name = user.name;
    let email = user.email;
    let company = user.company.name;
    print("  Name: {name}");
    print("  Email: {email}");
    print("  Company: {company}");

    // Fetch posts for this user (limited to 3)
    print("\nFetching posts...");
    let posts_response = Http.get("https://jsonplaceholder.typicode.com/posts?userId=1&_limit=3");
    let posts = Json.parse(posts_response.body);

    print("\nUser's Posts:");
    for post in posts {
        let title = post.title;
        print("  - {title}");
    }

    print("\nDone!");
}
```

**Output:**
```
Web Data Fetcher
================

Fetching user data...

User Info:
  Name: Leanne Graham
  Email: Sincere@april.biz
  Company: Romaguera-Crona

Fetching posts...

User's Posts:
  - sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  - qui est esse
  - ea molestias quasi exercitationem repellat qui ipsa sit aut

Done!
```

**Key concepts:**
- Multiple HTTP requests in sequence
- Nested JSON property access (`user.company.name`)
- Iterating over JSON arrays with `for` loops
- Query parameters in URLs (`?userId=1&_limit=3`)
- Real-world API integration patterns
