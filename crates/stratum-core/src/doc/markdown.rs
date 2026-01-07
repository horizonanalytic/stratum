//! Markdown documentation generator

use std::fmt::Write;

use super::types::{DocumentedItem, DocumentedModule, ItemKind};

/// Generates Markdown documentation from extracted documentation
pub struct MarkdownGenerator;

impl MarkdownGenerator {
    /// Generate Markdown documentation for a module
    pub fn generate(module: &DocumentedModule) -> String {
        let mut output = String::new();

        // Module header
        writeln!(output, "# {}", module.name).unwrap();
        writeln!(output).unwrap();

        // Module-level documentation
        if let Some(doc) = &module.doc {
            if !doc.summary.is_empty() {
                writeln!(output, "{}", doc.summary).unwrap();
                writeln!(output).unwrap();
            }
            if let Some(desc) = &doc.description {
                writeln!(output, "{}", desc).unwrap();
                writeln!(output).unwrap();
            }
        }

        // Table of contents
        if !module.items.is_empty() {
            writeln!(output, "## Contents").unwrap();
            writeln!(output).unwrap();

            Self::write_toc(&mut output, module);
            writeln!(output).unwrap();
        }

        // Functions
        let functions: Vec<_> = module.functions().collect();
        if !functions.is_empty() {
            writeln!(output, "## Functions").unwrap();
            writeln!(output).unwrap();

            for item in functions {
                Self::write_item(&mut output, item, 3);
            }
        }

        // Structs
        let structs: Vec<_> = module.structs().collect();
        if !structs.is_empty() {
            writeln!(output, "## Structs").unwrap();
            writeln!(output).unwrap();

            for item in structs {
                Self::write_item(&mut output, item, 3);
            }
        }

        // Enums
        let enums: Vec<_> = module.enums().collect();
        if !enums.is_empty() {
            writeln!(output, "## Enums").unwrap();
            writeln!(output).unwrap();

            for item in enums {
                Self::write_item(&mut output, item, 3);
            }
        }

        // Interfaces
        let interfaces: Vec<_> = module.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(output, "## Interfaces").unwrap();
            writeln!(output).unwrap();

            for item in interfaces {
                Self::write_item(&mut output, item, 3);
            }
        }

        // Impl blocks
        let impls: Vec<_> = module.items.iter().filter(|i| i.kind == ItemKind::Impl).collect();
        if !impls.is_empty() {
            writeln!(output, "## Implementations").unwrap();
            writeln!(output).unwrap();

            for item in impls {
                Self::write_item(&mut output, item, 3);
            }
        }

        // Constants
        let constants: Vec<_> = module
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Constant)
            .collect();
        if !constants.is_empty() {
            writeln!(output, "## Constants").unwrap();
            writeln!(output).unwrap();

            for item in constants {
                Self::write_item(&mut output, item, 3);
            }
        }

        output
    }

    fn write_toc(output: &mut String, module: &DocumentedModule) {
        let mut categories = Vec::new();

        let functions: Vec<_> = module.functions().collect();
        if !functions.is_empty() {
            categories.push(("Functions", functions));
        }

        let structs: Vec<_> = module.structs().collect();
        if !structs.is_empty() {
            categories.push(("Structs", structs));
        }

        let enums: Vec<_> = module.enums().collect();
        if !enums.is_empty() {
            categories.push(("Enums", enums));
        }

        let interfaces: Vec<_> = module.interfaces().collect();
        if !interfaces.is_empty() {
            categories.push(("Interfaces", interfaces));
        }

        for (category, items) in categories {
            writeln!(output, "### {}", category).unwrap();
            for item in items {
                let anchor = item.name.to_lowercase().replace(' ', "-");
                writeln!(output, "- [`{}`](#{})", item.name, anchor).unwrap();
            }
            writeln!(output).unwrap();
        }
    }

    fn write_item(output: &mut String, item: &DocumentedItem, heading_level: usize) {
        let hashes = "#".repeat(heading_level);

        // Item header with anchor
        writeln!(output, "{} `{}`", hashes, item.name).unwrap();
        writeln!(output).unwrap();

        // Signature
        writeln!(output, "```stratum").unwrap();
        writeln!(output, "{}", item.signature).unwrap();
        writeln!(output, "```").unwrap();
        writeln!(output).unwrap();

        // Documentation
        if let Some(doc) = &item.doc {
            if !doc.summary.is_empty() {
                writeln!(output, "{}", doc.summary).unwrap();
                writeln!(output).unwrap();
            }

            if let Some(desc) = &doc.description {
                writeln!(output, "{}", desc).unwrap();
                writeln!(output).unwrap();
            }

            // Parameters
            if !doc.params.is_empty() {
                writeln!(output, "**Arguments:**").unwrap();
                writeln!(output).unwrap();
                for (name, param) in &doc.params {
                    writeln!(output, "- `{}`: {}", name, param.description).unwrap();
                }
                writeln!(output).unwrap();
            }

            // Returns
            if let Some(returns) = &doc.returns {
                writeln!(output, "**Returns:** {}", returns).unwrap();
                writeln!(output).unwrap();
            }

            // Throws
            if !doc.throws.is_empty() {
                writeln!(output, "**Throws:**").unwrap();
                writeln!(output).unwrap();
                for throw in &doc.throws {
                    writeln!(output, "- {}", throw).unwrap();
                }
                writeln!(output).unwrap();
            }

            // Examples
            if !doc.examples.is_empty() {
                writeln!(output, "**Example:**").unwrap();
                writeln!(output).unwrap();
                for example in &doc.examples {
                    writeln!(output, "```stratum").unwrap();
                    writeln!(output, "{}", example).unwrap();
                    writeln!(output, "```").unwrap();
                    writeln!(output).unwrap();
                }
            }
        }

        // Children (fields, variants, methods)
        if !item.children.is_empty() {
            let child_type = match item.kind {
                ItemKind::Struct => "Fields",
                ItemKind::Enum => "Variants",
                ItemKind::Interface | ItemKind::Impl => "Methods",
                _ => "Members",
            };

            writeln!(output, "**{}:**", child_type).unwrap();
            writeln!(output).unwrap();

            for child in &item.children {
                writeln!(output, "- `{}` - {}", child.signature, Self::child_summary(child)).unwrap();
            }
            writeln!(output).unwrap();
        }

        writeln!(output, "---").unwrap();
        writeln!(output).unwrap();
    }

    fn child_summary(item: &DocumentedItem) -> String {
        item.doc
            .as_ref()
            .map(|d| {
                if d.summary.is_empty() {
                    String::new()
                } else {
                    d.summary.clone()
                }
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::DocExtractor;
    use crate::Parser;

    #[test]
    fn test_generate_markdown() {
        let source = r#"
/// Greet a user by name.
///
/// ## Arguments
/// - `name`: The name of the user
///
/// ## Returns
/// A greeting string
fx greet(name: String) -> String {
    "Hello, {name}!"
}
"#;

        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "greeting");
        let markdown = MarkdownGenerator::generate(&doc_module);

        assert!(markdown.contains("# greeting"));
        assert!(markdown.contains("## Functions"));
        assert!(markdown.contains("`greet`"));
        assert!(markdown.contains("Greet a user"));
        assert!(markdown.contains("**Arguments:**"));
        assert!(markdown.contains("`name`"));
    }
}
