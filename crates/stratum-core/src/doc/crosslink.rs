//! Cross-linking for documentation
//!
//! This module resolves type references in documentation and signatures
//! to clickable links.

use std::collections::HashSet;

use super::project::ProjectDoc;

/// Configuration for cross-linking
#[derive(Debug, Clone)]
pub struct CrossLinkConfig {
    /// The current module name (for relative links)
    pub current_module: String,
    /// Whether to link to external types (stdlib, etc.)
    pub link_external: bool,
}

impl Default for CrossLinkConfig {
    fn default() -> Self {
        Self {
            current_module: String::new(),
            link_external: true,
        }
    }
}

/// Cross-linker for resolving type references to links
pub struct CrossLinker<'a> {
    project: &'a ProjectDoc,
    config: CrossLinkConfig,
    /// Built-in types that shouldn't be linked
    builtin_types: HashSet<&'static str>,
}

impl<'a> CrossLinker<'a> {
    /// Create a new cross-linker
    pub fn new(project: &'a ProjectDoc, config: CrossLinkConfig) -> Self {
        let mut builtin_types = HashSet::new();
        // Primitive types
        builtin_types.insert("Int");
        builtin_types.insert("Float");
        builtin_types.insert("String");
        builtin_types.insert("Bool");
        builtin_types.insert("Char");
        builtin_types.insert("Byte");
        // Common generic containers are NOT builtin - they should be linked if documented

        Self {
            project,
            config,
            builtin_types,
        }
    }

    /// Process a signature string and add cross-links
    pub fn link_signature(&self, signature: &str) -> String {
        self.add_links_to_text(signature)
    }

    /// Process description text and add cross-links
    pub fn link_description(&self, text: &str) -> String {
        self.add_links_to_text(text)
    }

    /// Add links to type names in text
    fn add_links_to_text(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        let mut current_word = String::new();

        while let Some(c) = chars.next() {
            if c.is_alphabetic() || c == '_' {
                current_word.push(c);
                // Continue collecting alphanumeric characters
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '_' {
                        current_word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Check if this word is a type we should link
                if self.should_link(&current_word) {
                    if let Some(symbol) = self
                        .project
                        .lookup_in_module(&current_word, &self.config.current_module)
                    {
                        let link = self
                            .project
                            .link_to_symbol(symbol, &self.config.current_module);
                        result.push_str(&format!(
                            "<a href=\"{}\" class=\"type-link\">{}</a>",
                            link, current_word
                        ));
                    } else {
                        result.push_str(&current_word);
                    }
                } else {
                    result.push_str(&current_word);
                }
                current_word.clear();
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Check if a word should be linked
    fn should_link(&self, word: &str) -> bool {
        // Don't link builtin types
        if self.builtin_types.contains(word.as_ref() as &str) {
            return false;
        }

        // Don't link keywords
        if is_keyword(word) {
            return false;
        }

        // Only link words that start with uppercase (type names)
        // or are in the project's symbol index
        word.chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            || self.project.lookup(word).is_some()
    }

    /// Resolve a `See Also` reference to a link
    pub fn resolve_see_also(&self, reference: &str) -> Option<(String, String)> {
        // Parse references like "Module.function" or just "TypeName"
        let parts: Vec<&str> = reference.split('.').collect();

        let name = parts.last()?;

        // Try to find the symbol
        if let Some(symbol) = self
            .project
            .lookup_in_module(name, &self.config.current_module)
        {
            let link = self
                .project
                .link_to_symbol(symbol, &self.config.current_module);
            Some((reference.to_string(), link))
        } else {
            None
        }
    }
}

/// Check if a word is a Stratum keyword
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "fx" | "let"
            | "if"
            | "else"
            | "while"
            | "for"
            | "in"
            | "match"
            | "return"
            | "break"
            | "continue"
            | "struct"
            | "enum"
            | "interface"
            | "impl"
            | "import"
            | "as"
            | "true"
            | "false"
            | "nil"
            | "self"
            | "async"
            | "await"
            | "try"
            | "catch"
            | "throw"
            | "pub"
            | "mut"
    )
}

/// Parse type references from a signature
/// Returns a list of type names found in the signature
pub fn extract_type_names(signature: &str) -> Vec<String> {
    let mut types = Vec::new();
    let mut current = String::new();
    let mut in_word = false;

    for c in signature.chars() {
        if c.is_alphabetic() || (in_word && (c.is_alphanumeric() || c == '_')) {
            current.push(c);
            in_word = true;
        } else {
            if !current.is_empty() {
                // Check if it looks like a type (starts with uppercase)
                if current
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    types.push(current.clone());
                }
                current.clear();
            }
            in_word = false;
        }
    }

    // Don't forget the last word
    if !current.is_empty()
        && current
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    {
        types.push(current);
    }

    types
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::project::ProjectDoc;
    use crate::doc::types::{DocumentedItem, DocumentedModule, ItemKind};

    fn create_test_project() -> ProjectDoc {
        let mut project = ProjectDoc::new("test");

        let mut module = DocumentedModule::new("utils".to_string());
        module.add_item(DocumentedItem::new(
            "User".to_string(),
            ItemKind::Struct,
            "struct User".to_string(),
        ));
        module.add_item(DocumentedItem::new(
            "greet".to_string(),
            ItemKind::Function,
            "fx greet(user: User) -> String".to_string(),
        ));

        project.add_module(module);
        project
    }

    #[test]
    fn test_link_signature() {
        let project = create_test_project();
        let linker = CrossLinker::new(
            &project,
            CrossLinkConfig {
                current_module: "utils".to_string(),
                link_external: true,
            },
        );

        let result = linker.link_signature("fx greet(user: User) -> String");
        assert!(result.contains("<a href=\"#user\" class=\"type-link\">User</a>"));
        // String is builtin, should not be linked
        assert!(!result.contains("href=\"#string\""));
    }

    #[test]
    fn test_extract_type_names() {
        let sig = "fx process(input: DataFrame, filter: Filter<T>) -> Result<Output>";
        let types = extract_type_names(sig);

        assert!(types.contains(&"DataFrame".to_string()));
        assert!(types.contains(&"Filter".to_string()));
        assert!(types.contains(&"T".to_string()));
        assert!(types.contains(&"Result".to_string()));
        assert!(types.contains(&"Output".to_string()));
    }

    #[test]
    fn test_builtin_not_linked() {
        let project = create_test_project();
        let linker = CrossLinker::new(
            &project,
            CrossLinkConfig {
                current_module: "utils".to_string(),
                link_external: true,
            },
        );

        let result = linker.link_signature("Int -> Float");
        // Builtins should not have links
        assert!(!result.contains("<a"));
        assert!(result.contains("Int"));
        assert!(result.contains("Float"));
    }
}
