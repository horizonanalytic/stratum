//! Project-wide documentation collection
//!
//! This module provides tools for collecting documentation across multiple
//! source files and building a unified symbol index for cross-referencing.

use std::collections::HashMap;
use std::path::Path;

use super::types::{DocumentedItem, DocumentedModule, ItemKind};

/// A symbol in the project documentation
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The symbol name
    pub name: String,
    /// The kind of symbol (function, struct, enum, etc.)
    pub kind: ItemKind,
    /// The module this symbol is defined in
    pub module: String,
    /// The anchor ID for linking
    pub anchor: String,
    /// Brief description (first sentence of summary)
    pub description: String,
    /// Full signature
    pub signature: String,
}

/// Project-wide documentation with cross-reference support
#[derive(Debug, Clone)]
pub struct ProjectDoc {
    /// Name of the project
    pub name: String,
    /// All documented modules
    pub modules: Vec<DocumentedModule>,
    /// Symbol index for cross-referencing (name -> list of symbols with that name)
    pub symbol_index: HashMap<String, Vec<SymbolInfo>>,
}

impl ProjectDoc {
    /// Create a new project documentation
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            modules: Vec::new(),
            symbol_index: HashMap::new(),
        }
    }

    /// Add a documented module and index its symbols
    pub fn add_module(&mut self, module: DocumentedModule) {
        self.index_module(&module);
        self.modules.push(module);
    }

    /// Index all symbols in a module for cross-referencing
    fn index_module(&mut self, module: &DocumentedModule) {
        for item in &module.items {
            self.index_item(item, &module.name);
        }
    }

    /// Index a single item and its children
    fn index_item(&mut self, item: &DocumentedItem, module_name: &str) {
        let anchor = Self::make_anchor(&item.name);
        let description = item
            .doc
            .as_ref()
            .map(|d| Self::first_sentence(&d.summary))
            .unwrap_or_default();

        let info = SymbolInfo {
            name: item.name.clone(),
            kind: item.kind,
            module: module_name.to_string(),
            anchor,
            description,
            signature: item.signature.clone(),
        };

        self.symbol_index
            .entry(item.name.clone())
            .or_default()
            .push(info);

        // Index children (methods, fields, variants)
        for child in &item.children {
            // For children, create a compound anchor
            let child_anchor = format!("{}-{}", Self::make_anchor(&item.name), Self::make_anchor(&child.name));
            let child_desc = child
                .doc
                .as_ref()
                .map(|d| Self::first_sentence(&d.summary))
                .unwrap_or_default();

            let child_info = SymbolInfo {
                name: child.name.clone(),
                kind: child.kind,
                module: module_name.to_string(),
                anchor: child_anchor,
                description: child_desc,
                signature: child.signature.clone(),
            };

            self.symbol_index
                .entry(child.name.clone())
                .or_default()
                .push(child_info);
        }
    }

    /// Look up a symbol by name
    pub fn lookup(&self, name: &str) -> Option<&[SymbolInfo]> {
        self.symbol_index.get(name).map(|v| v.as_slice())
    }

    /// Look up a symbol with preference for a specific module
    pub fn lookup_in_module(&self, name: &str, preferred_module: &str) -> Option<&SymbolInfo> {
        self.symbol_index.get(name).and_then(|symbols| {
            // First try to find in preferred module
            symbols
                .iter()
                .find(|s| s.module == preferred_module)
                .or_else(|| symbols.first())
        })
    }

    /// Get all symbols sorted alphabetically
    pub fn all_symbols(&self) -> Vec<&SymbolInfo> {
        let mut symbols: Vec<_> = self.symbol_index.values().flatten().collect();
        symbols.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        symbols
    }

    /// Get symbols by kind
    pub fn symbols_by_kind(&self, kind: ItemKind) -> Vec<&SymbolInfo> {
        self.symbol_index
            .values()
            .flatten()
            .filter(|s| s.kind == kind)
            .collect()
    }

    /// Create an anchor ID from a name
    fn make_anchor(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect()
    }

    /// Extract first sentence from text
    fn first_sentence(text: &str) -> String {
        text.split(|c| c == '.' || c == '\n')
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    }

    /// Get the relative path from one module to another
    pub fn relative_path(from_module: &str, to_module: &str) -> String {
        if from_module == to_module {
            String::new()
        } else {
            format!("{}.html", to_module)
        }
    }

    /// Build a link to a symbol from a given module
    pub fn link_to_symbol(&self, symbol: &SymbolInfo, from_module: &str) -> String {
        let path = Self::relative_path(from_module, &symbol.module);
        if path.is_empty() {
            format!("#{}", symbol.anchor)
        } else {
            format!("{}#{}", path, symbol.anchor)
        }
    }
}

/// Build project documentation from a directory of source files
pub fn build_project_doc<P: AsRef<Path>>(
    project_name: &str,
    files: &[(P, &str)], // (file_path, source_content)
) -> ProjectDoc {
    use super::DocExtractor;
    use crate::Parser;

    let mut project = ProjectDoc::new(project_name);

    for (path, source) in files {
        let path = path.as_ref();
        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Parse and extract documentation
        if let Ok(module) = Parser::parse_module(source) {
            let doc_module = DocExtractor::extract(&module, module_name);
            project.add_module(doc_module);
        }
    }

    project
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::DocExtractor;
    use crate::Parser;

    #[test]
    fn test_project_doc_indexing() {
        let source = r#"
/// A greeting function.
fx greet(name: String) -> String {
    "Hello, {name}!"
}

/// A user struct.
struct User {
    name: String,
    age: Int,
}
"#;

        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "greeting");

        let mut project = ProjectDoc::new("test");
        project.add_module(doc_module);

        // Check symbol indexing
        assert!(project.lookup("greet").is_some());
        assert!(project.lookup("User").is_some());
        assert_eq!(project.lookup("greet").unwrap().len(), 1);

        let greet_info = &project.lookup("greet").unwrap()[0];
        assert_eq!(greet_info.kind, ItemKind::Function);
        assert_eq!(greet_info.module, "greeting");
    }

    #[test]
    fn test_all_symbols() {
        let source = r#"
fx alpha() {}
fx beta() {}
struct Gamma {}
"#;

        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "test");

        let mut project = ProjectDoc::new("test");
        project.add_module(doc_module);

        let symbols = project.all_symbols();
        assert_eq!(symbols.len(), 3);
        // Should be sorted alphabetically
        assert_eq!(symbols[0].name, "alpha");
        assert_eq!(symbols[1].name, "beta");
        assert_eq!(symbols[2].name, "Gamma");
    }
}
