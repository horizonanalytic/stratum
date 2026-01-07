//! Types for representing extracted documentation

use std::collections::HashMap;

/// Parsed documentation comment
#[derive(Debug, Clone, Default)]
pub struct DocComment {
    /// Brief summary (first paragraph)
    pub summary: String,
    /// Full description (everything after the first blank line)
    pub description: Option<String>,
    /// Parameter documentation
    pub params: HashMap<String, ParamDoc>,
    /// Return value documentation
    pub returns: Option<String>,
    /// Example code blocks
    pub examples: Vec<String>,
    /// Thrown exceptions
    pub throws: Vec<String>,
    /// See also references
    pub see_also: Vec<String>,
}

impl DocComment {
    /// Parse a doc comment from raw text
    pub fn parse(text: &str) -> Self {
        let mut doc = DocComment::default();
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return doc;
        }

        let mut current_section = Section::Summary;
        let mut summary_lines = Vec::new();
        let mut description_lines = Vec::new();
        let mut current_example = Vec::new();
        let mut in_code_block = false;

        for line in lines {
            let trimmed = line.trim();

            // Handle code blocks
            if trimmed.starts_with("```") {
                if in_code_block {
                    // End of code block
                    in_code_block = false;
                    if current_section == Section::Example {
                        doc.examples.push(current_example.join("\n"));
                        current_example.clear();
                    }
                } else {
                    // Start of code block
                    in_code_block = true;
                    if trimmed.contains("stratum") || current_section == Section::Example {
                        current_section = Section::Example;
                    }
                }
                continue;
            }

            if in_code_block {
                if current_section == Section::Example {
                    current_example.push(line.to_string());
                }
                continue;
            }

            // Parse section headers
            if let Some(section) = parse_section_header(trimmed) {
                current_section = section;
                continue;
            }

            // Parse inline tags like `- param_name: description`
            if current_section == Section::Arguments {
                if let Some((name, desc)) = parse_param_line(trimmed) {
                    doc.params.insert(name, ParamDoc { description: desc });
                    continue;
                }
            }

            // Accumulate content based on current section
            match current_section {
                Section::Summary => {
                    if trimmed.is_empty() && !summary_lines.is_empty() {
                        current_section = Section::Description;
                    } else if !trimmed.is_empty() {
                        summary_lines.push(trimmed);
                    }
                }
                Section::Description => {
                    description_lines.push(trimmed);
                }
                Section::Returns => {
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        let existing = doc.returns.take().unwrap_or_default();
                        if existing.is_empty() {
                            doc.returns = Some(trimmed.to_string());
                        } else {
                            doc.returns = Some(format!("{} {}", existing, trimmed));
                        }
                    }
                }
                Section::Throws => {
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        doc.throws.push(trimmed.to_string());
                    }
                }
                Section::SeeAlso => {
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        doc.see_also.push(trimmed.to_string());
                    }
                }
                Section::Arguments | Section::Example => {}
            }
        }

        // Finalize summary
        doc.summary = summary_lines.join(" ");

        // Finalize description
        let desc = description_lines.join("\n").trim().to_string();
        if !desc.is_empty() {
            doc.description = Some(desc);
        }

        doc
    }

    /// Check if the doc comment is empty
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty()
            && self.description.is_none()
            && self.params.is_empty()
            && self.returns.is_none()
            && self.examples.is_empty()
    }
}

/// Documentation for a function parameter
#[derive(Debug, Clone)]
pub struct ParamDoc {
    /// Description of the parameter
    pub description: String,
}

/// Section being parsed
#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Summary,
    Description,
    Arguments,
    Returns,
    Example,
    Throws,
    SeeAlso,
}

fn parse_section_header(line: &str) -> Option<Section> {
    let lower = line.to_lowercase();
    if lower.starts_with("## arguments") || lower.starts_with("# arguments") {
        Some(Section::Arguments)
    } else if lower.starts_with("## returns") || lower.starts_with("# returns") {
        Some(Section::Returns)
    } else if lower.starts_with("## example") || lower.starts_with("# example") {
        Some(Section::Example)
    } else if lower.starts_with("## throws") || lower.starts_with("# throws") {
        Some(Section::Throws)
    } else if lower.starts_with("## see also") || lower.starts_with("# see also") {
        Some(Section::SeeAlso)
    } else {
        None
    }
}

fn parse_param_line(line: &str) -> Option<(String, String)> {
    // Parse lines like "- `name`: description" or "- name: description"
    let line = line.strip_prefix('-')?.trim();

    // Handle backtick-wrapped names
    let (name, rest) = if line.starts_with('`') {
        let end = line[1..].find('`')?;
        let name = &line[1..=end];
        let rest = line[end + 2..].trim();
        (name, rest)
    } else {
        // Handle plain names
        let colon_pos = line.find(':')?;
        let name = line[..colon_pos].trim();
        let rest = line[colon_pos + 1..].trim();
        (name, rest)
    };

    let description = rest.strip_prefix(':').unwrap_or(rest).trim();
    Some((name.to_string(), description.to_string()))
}

/// A documented item (function, struct, enum, etc.)
#[derive(Debug, Clone)]
pub struct DocumentedItem {
    /// Name of the item
    pub name: String,
    /// Kind of item
    pub kind: ItemKind,
    /// Documentation comment
    pub doc: Option<DocComment>,
    /// Signature (for display)
    pub signature: String,
    /// Child items (fields for structs, variants for enums, methods for impls)
    pub children: Vec<DocumentedItem>,
}

impl DocumentedItem {
    /// Create a new documented item
    pub fn new(name: String, kind: ItemKind, signature: String) -> Self {
        Self {
            name,
            kind,
            doc: None,
            signature,
            children: Vec::new(),
        }
    }

    /// Set the documentation
    pub fn with_doc(mut self, doc: Option<DocComment>) -> Self {
        self.doc = doc;
        self
    }

    /// Add a child item
    pub fn add_child(&mut self, child: DocumentedItem) {
        self.children.push(child);
    }
}

/// Kind of documented item
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemKind {
    Function,
    Struct,
    Field,
    Enum,
    Variant,
    Interface,
    Method,
    Impl,
    Constant,
}

impl ItemKind {
    /// Get the display name for the item kind
    pub fn display_name(&self) -> &'static str {
        match self {
            ItemKind::Function => "Function",
            ItemKind::Struct => "Struct",
            ItemKind::Field => "Field",
            ItemKind::Enum => "Enum",
            ItemKind::Variant => "Variant",
            ItemKind::Interface => "Interface",
            ItemKind::Method => "Method",
            ItemKind::Impl => "Implementation",
            ItemKind::Constant => "Constant",
        }
    }
}

/// Documentation for a complete module/file
#[derive(Debug, Clone)]
pub struct DocumentedModule {
    /// Module name (usually file stem)
    pub name: String,
    /// Module-level documentation
    pub doc: Option<DocComment>,
    /// Documented items in the module
    pub items: Vec<DocumentedItem>,
}

impl DocumentedModule {
    /// Create a new documented module
    pub fn new(name: String) -> Self {
        Self {
            name,
            doc: None,
            items: Vec::new(),
        }
    }

    /// Add a documented item
    pub fn add_item(&mut self, item: DocumentedItem) {
        self.items.push(item);
    }

    /// Get all functions
    pub fn functions(&self) -> impl Iterator<Item = &DocumentedItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Function)
    }

    /// Get all structs
    pub fn structs(&self) -> impl Iterator<Item = &DocumentedItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Struct)
    }

    /// Get all enums
    pub fn enums(&self) -> impl Iterator<Item = &DocumentedItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Enum)
    }

    /// Get all interfaces
    pub fn interfaces(&self) -> impl Iterator<Item = &DocumentedItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Interface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_doc() {
        let text = "This is a simple function.";
        let doc = DocComment::parse(text);
        assert_eq!(doc.summary, "This is a simple function.");
        assert!(doc.description.is_none());
    }

    #[test]
    fn test_parse_doc_with_description() {
        let text = "Brief summary.\n\nThis is the longer description\nthat spans multiple lines.";
        let doc = DocComment::parse(text);
        assert_eq!(doc.summary, "Brief summary.");
        assert!(doc.description.is_some());
        assert!(doc.description.unwrap().contains("longer description"));
    }

    #[test]
    fn test_parse_doc_with_params() {
        let text = r#"Greet a user.

## Arguments
- `name`: The user's name
- `greeting`: The greeting to use"#;
        let doc = DocComment::parse(text);
        assert_eq!(doc.summary, "Greet a user.");
        assert_eq!(doc.params.len(), 2);
        assert_eq!(doc.params.get("name").unwrap().description, "The user's name");
    }

    #[test]
    fn test_parse_doc_with_returns() {
        let text = r#"Add two numbers.

## Returns
The sum of a and b"#;
        let doc = DocComment::parse(text);
        assert_eq!(doc.returns, Some("The sum of a and b".to_string()));
    }

    #[test]
    fn test_parse_doc_with_example() {
        let text = r#"Greet a user.

## Example
```stratum
let greeting = greet("World")
```"#;
        let doc = DocComment::parse(text);
        assert_eq!(doc.examples.len(), 1);
        assert!(doc.examples[0].contains("greet"));
    }
}
