# Stratum Examples

A collection of example programs demonstrating the Stratum programming language.

## Running Examples

```bash
# Run any example
stratum run examples/simple/hello_world.strat

# Run with interpreter (fast startup)
stratum run --interpret-all examples/simple/fizzbuzz.strat

# Compile and run (optimized)
stratum build examples/advanced/data_analysis.strat
./data_analysis
```

## Simple Examples

Basic programs to learn Stratum syntax and concepts.

| Example | Description |
|---------|-------------|
| `hello_world.strat` | The classic first program |
| `variables.strat` | Variables, type inference, collections |
| `fizzbuzz.strat` | Pattern matching and loops |
| `fibonacci.strat` | Recursion and iteration |
| `factorial.strat` | Multiple approaches to the same problem |

## Intermediate Examples

Programs that demonstrate more complex features.

| Example | Description |
|---------|-------------|
| `word_count.strat` | String processing, maps, sorting |
| `json_api.strat` | HTTP requests, async/await, JSON parsing |
| `temperature_converter.strat` | Enums with data, pattern matching |

## Advanced Examples

Full applications showcasing Stratum's power.

| Example | Description |
|---------|-------------|
| `data_analysis.strat` | DataFrame operations with the Iris dataset |
| `csv_processor.strat` | Read, transform, and analyze tabular data |
| `todo_app.strat` | Complete GUI application with state management |
| `web_scraper.strat` | Concurrent HTTP requests, data aggregation |

## Key Language Features Demonstrated

### Pipeline Operator (`|>`)
```stratum
let result = data
    |> filter(.amount > 100)
    |> group_by(.region)
    |> aggregate(total: sum(.amount))
```

### Pattern Matching
```stratum
let result = match value {
    Some(x) => "Got {x}",
    None => "Nothing"
}
```

### Async/Await
```stratum
async fx fetch_data() -> Data {
    let response = await http.get(url)
    json.parse(response.text())
}
```

### Type Inference
```stratum
let numbers = [1, 2, 3]      // List<Int> inferred
let name = "Stratum"          // String inferred
let scores = {"a": 1, "b": 2} // Map<String, Int> inferred
```

### Compilation Directives
```stratum
#![interpret]  // File runs interpreted (fast startup)
#![compile]    // File is compiled (max performance)

#[compile]     // This function is always compiled
fx hot_path() { ... }
```

## External Data Sources

Some examples fetch data from public APIs:

- **JSONPlaceholder**: https://jsonplaceholder.typicode.com
- **GitHub API**: https://api.github.com
- **Hacker News API**: https://hacker-news.firebaseio.com
- **Iris Dataset**: https://github.com/mwaskom/seaborn-data
