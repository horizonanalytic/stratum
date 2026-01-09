# Xml Module

The `Xml` module provides XML parsing with full XPath 1.0 query support.

## Overview

```stratum
let doc = Xml.parse("<book><title>Hello</title></book>")
let titles = doc.query("//title")
print(titles)  // ["Hello"]
```

## Static Methods

### Xml.parse(content: String) -> XmlDocument

Parse an XML string into a document object.

**Parameters:**
- `content` - The XML content to parse

**Returns:** An `XmlDocument` object

**Example:**
```stratum
let xml = """
<library>
    <book id="1">
        <title>The Great Gatsby</title>
        <author>F. Scott Fitzgerald</author>
    </book>
    <book id="2">
        <title>1984</title>
        <author>George Orwell</author>
    </book>
</library>
"""

let doc = Xml.parse(xml)
```

### Xml.stringify(doc: XmlDocument) -> String

Convert an XmlDocument back to its string representation.

**Parameters:**
- `doc` - The XML document to serialize

**Returns:** The XML content as a string

**Example:**
```stratum
let doc = Xml.parse("<root><item>test</item></root>")
let str = Xml.stringify(doc)
print(str)  // <root><item>test</item></root>
```

## XmlDocument Methods

### doc.query(xpath: String) -> Value

Execute an XPath 1.0 query against the document.

**Parameters:**
- `xpath` - The XPath expression to evaluate

**Returns:**
- A `List` of strings when querying for elements/text
- A `String` when querying for text content
- A `Bool` for boolean expressions
- An `Int` or `Float` for numeric expressions

**XPath Examples:**
```stratum
let doc = Xml.parse(xml)

// Select all book titles
let titles = doc.query("//book/title")
// ["The Great Gatsby", "1984"]

// Select book by attribute
let book1 = doc.query("//book[@id='1']/title")
// ["The Great Gatsby"]

// Count books
let count = doc.query("count(//book)")
// 2

// Get specific element text
let author = doc.query("//book[1]/author/text()")
// ["F. Scott Fitzgerald"]

// Boolean check
let hasBooks = doc.query("count(//book) > 0")
// true
```

### doc.text() -> String

Get all text content from the document, concatenated.

**Returns:** All text nodes joined together (whitespace trimmed)

**Example:**
```stratum
let doc = Xml.parse("<p>Hello <b>World</b>!</p>")
print(doc.text())  // HelloWorld!
```

### doc.root() -> String

Get the name of the root element.

**Returns:** The root element's tag name

**Example:**
```stratum
let doc = Xml.parse("<library><book/></library>")
print(doc.root())  // library
```

### doc.content() -> String

Get the original XML content as a string.

**Returns:** The raw XML string

## XPath 1.0 Reference

The `Xml` module supports the full XPath 1.0 specification:

### Axes
- `child::` - Direct children (default)
- `descendant::` - All descendants
- `parent::` - Parent element
- `ancestor::` - All ancestors
- `following-sibling::` - Following siblings
- `preceding-sibling::` - Preceding siblings
- `attribute::` or `@` - Attributes
- `self::` - Current node

### Predicates
- `[1]` - First element
- `[last()]` - Last element
- `[@attr='value']` - Attribute filter
- `[contains(text(), 'search')]` - Text filter

### Functions
- `count()` - Count nodes
- `sum()` - Sum numeric values
- `concat()` - Concatenate strings
- `contains()` - String contains
- `starts-with()` - String prefix
- `string-length()` - String length
- `normalize-space()` - Trim whitespace
- `not()` - Boolean negation
- `position()` - Node position
- `last()` - Last position

## Common Patterns

### Parsing Configuration Files
```stratum
let config = File.read_text("config.xml")
let doc = Xml.parse(config)

let dbHost = doc.query("//database/host/text()")
let dbPort = doc.query("//database/port/text()")
```

### Processing RSS Feeds
```stratum
let rss = Http.get("https://example.com/feed.xml").text
let doc = Xml.parse(rss)

let titles = doc.query("//item/title")
let links = doc.query("//item/link")

for i in 0..titles.len() {
    print(titles[i] + ": " + links[i])
}
```

### Extracting Data
```stratum
let html = """
<table>
    <tr><td>Alice</td><td>30</td></tr>
    <tr><td>Bob</td><td>25</td></tr>
</table>
"""

let doc = Xml.parse(html)
let names = doc.query("//tr/td[1]")
let ages = doc.query("//tr/td[2]")
```
