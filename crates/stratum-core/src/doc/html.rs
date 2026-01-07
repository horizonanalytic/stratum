//! HTML documentation generator

use std::fmt::Write;

use super::types::{DocumentedItem, DocumentedModule, ItemKind};

/// Generates HTML documentation from extracted documentation
pub struct HtmlGenerator;

impl HtmlGenerator {
    /// Generate HTML documentation for a module
    pub fn generate(module: &DocumentedModule) -> String {
        let mut output = String::new();

        // HTML header
        writeln!(output, "<!DOCTYPE html>").unwrap();
        writeln!(output, "<html lang=\"en\">").unwrap();
        writeln!(output, "<head>").unwrap();
        writeln!(output, "  <meta charset=\"UTF-8\">").unwrap();
        writeln!(
            output,
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )
        .unwrap();
        writeln!(output, "  <title>{} - Stratum Documentation</title>", module.name).unwrap();
        Self::write_styles(&mut output);
        writeln!(output, "</head>").unwrap();
        writeln!(output, "<body>").unwrap();

        // Navigation sidebar
        writeln!(output, "<nav class=\"sidebar\">").unwrap();
        writeln!(output, "  <div class=\"sidebar-header\">").unwrap();
        writeln!(output, "    <h2>{}</h2>", module.name).unwrap();
        writeln!(output, "  </div>").unwrap();
        Self::write_nav(&mut output, module);
        writeln!(output, "</nav>").unwrap();

        // Main content
        writeln!(output, "<main class=\"content\">").unwrap();

        // Module header
        writeln!(output, "<header>").unwrap();
        writeln!(output, "  <h1>{}</h1>", module.name).unwrap();
        writeln!(output, "</header>").unwrap();

        // Module documentation
        if let Some(doc) = &module.doc {
            writeln!(output, "<section class=\"module-doc\">").unwrap();
            if !doc.summary.is_empty() {
                writeln!(output, "  <p class=\"summary\">{}</p>", Self::escape_html(&doc.summary))
                    .unwrap();
            }
            if let Some(desc) = &doc.description {
                writeln!(output, "  <div class=\"description\">{}</div>", Self::escape_html(desc))
                    .unwrap();
            }
            writeln!(output, "</section>").unwrap();
        }

        // Functions
        let functions: Vec<_> = module.functions().collect();
        if !functions.is_empty() {
            writeln!(output, "<section id=\"functions\">").unwrap();
            writeln!(output, "  <h2>Functions</h2>").unwrap();
            for item in functions {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        // Structs
        let structs: Vec<_> = module.structs().collect();
        if !structs.is_empty() {
            writeln!(output, "<section id=\"structs\">").unwrap();
            writeln!(output, "  <h2>Structs</h2>").unwrap();
            for item in structs {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        // Enums
        let enums: Vec<_> = module.enums().collect();
        if !enums.is_empty() {
            writeln!(output, "<section id=\"enums\">").unwrap();
            writeln!(output, "  <h2>Enums</h2>").unwrap();
            for item in enums {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        // Interfaces
        let interfaces: Vec<_> = module.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(output, "<section id=\"interfaces\">").unwrap();
            writeln!(output, "  <h2>Interfaces</h2>").unwrap();
            for item in interfaces {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        // Implementations
        let impls: Vec<_> = module.items.iter().filter(|i| i.kind == ItemKind::Impl).collect();
        if !impls.is_empty() {
            writeln!(output, "<section id=\"implementations\">").unwrap();
            writeln!(output, "  <h2>Implementations</h2>").unwrap();
            for item in impls {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        // Constants
        let constants: Vec<_> = module
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Constant)
            .collect();
        if !constants.is_empty() {
            writeln!(output, "<section id=\"constants\">").unwrap();
            writeln!(output, "  <h2>Constants</h2>").unwrap();
            for item in constants {
                Self::write_item(&mut output, item);
            }
            writeln!(output, "</section>").unwrap();
        }

        writeln!(output, "</main>").unwrap();

        // Footer
        writeln!(output, "<footer>").unwrap();
        writeln!(
            output,
            "  <p>Generated by <a href=\"https://stratum-lang.org\">Stratum</a></p>"
        )
        .unwrap();
        writeln!(output, "</footer>").unwrap();

        writeln!(output, "</body>").unwrap();
        writeln!(output, "</html>").unwrap();

        output
    }

    fn write_styles(output: &mut String) {
        writeln!(output, "<style>").unwrap();
        writeln!(
            output,
            r#"
:root {{
  --bg-color: #1a1a2e;
  --text-color: #eaeaea;
  --accent-color: #7b68ee;
  --code-bg: #16213e;
  --sidebar-bg: #0f0f23;
  --border-color: #333;
}}

* {{
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}}

body {{
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  background-color: var(--bg-color);
  color: var(--text-color);
  line-height: 1.6;
  display: flex;
}}

.sidebar {{
  width: 280px;
  background-color: var(--sidebar-bg);
  height: 100vh;
  position: fixed;
  overflow-y: auto;
  border-right: 1px solid var(--border-color);
}}

.sidebar-header {{
  padding: 1.5rem;
  border-bottom: 1px solid var(--border-color);
}}

.sidebar-header h2 {{
  color: var(--accent-color);
  font-size: 1.25rem;
}}

.sidebar nav {{
  padding: 1rem;
}}

.sidebar h3 {{
  font-size: 0.875rem;
  text-transform: uppercase;
  color: #888;
  margin: 1rem 0 0.5rem;
}}

.sidebar ul {{
  list-style: none;
}}

.sidebar a {{
  display: block;
  padding: 0.25rem 0;
  color: var(--text-color);
  text-decoration: none;
  font-size: 0.9rem;
}}

.sidebar a:hover {{
  color: var(--accent-color);
}}

.content {{
  margin-left: 280px;
  padding: 2rem 3rem;
  max-width: 900px;
}}

header h1 {{
  font-size: 2.5rem;
  color: var(--accent-color);
  margin-bottom: 1rem;
}}

section {{
  margin: 2rem 0;
}}

section h2 {{
  font-size: 1.5rem;
  color: var(--accent-color);
  border-bottom: 2px solid var(--border-color);
  padding-bottom: 0.5rem;
  margin-bottom: 1.5rem;
}}

.item {{
  margin: 1.5rem 0;
  padding: 1rem;
  background: var(--code-bg);
  border-radius: 8px;
  border-left: 3px solid var(--accent-color);
}}

.item h3 {{
  font-size: 1.1rem;
  margin-bottom: 0.5rem;
}}

.item h3 code {{
  background: none;
  color: var(--accent-color);
  padding: 0;
}}

.signature {{
  background: #0d1117;
  padding: 0.75rem 1rem;
  border-radius: 4px;
  overflow-x: auto;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 0.9rem;
  margin: 0.5rem 0;
}}

.summary {{
  font-size: 1rem;
  margin: 0.75rem 0;
}}

.description {{
  color: #bbb;
  margin: 0.5rem 0;
}}

.params, .returns, .throws {{
  margin: 0.75rem 0;
}}

.params h4, .returns h4, .throws h4 {{
  font-size: 0.9rem;
  color: #888;
  margin-bottom: 0.25rem;
}}

.params ul {{
  list-style: none;
  padding-left: 1rem;
}}

.params li {{
  margin: 0.25rem 0;
}}

.params code {{
  color: var(--accent-color);
}}

.example {{
  margin: 1rem 0;
}}

.example h4 {{
  font-size: 0.9rem;
  color: #888;
  margin-bottom: 0.5rem;
}}

.example pre {{
  background: #0d1117;
  padding: 1rem;
  border-radius: 4px;
  overflow-x: auto;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 0.85rem;
}}

.children {{
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-color);
}}

.children h4 {{
  font-size: 0.9rem;
  color: #888;
  margin-bottom: 0.5rem;
}}

.children ul {{
  list-style: none;
}}

.children li {{
  margin: 0.25rem 0;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 0.85rem;
}}

code {{
  background: var(--code-bg);
  padding: 0.15rem 0.3rem;
  border-radius: 3px;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 0.9em;
}}

footer {{
  position: fixed;
  bottom: 0;
  right: 0;
  padding: 0.5rem 1rem;
  font-size: 0.75rem;
  color: #666;
}}

footer a {{
  color: var(--accent-color);
}}

@media (max-width: 768px) {{
  .sidebar {{
    display: none;
  }}
  .content {{
    margin-left: 0;
    padding: 1rem;
  }}
}}
"#
        )
        .unwrap();
        writeln!(output, "</style>").unwrap();
    }

    fn write_nav(output: &mut String, module: &DocumentedModule) {
        writeln!(output, "  <nav>").unwrap();

        let functions: Vec<_> = module.functions().collect();
        if !functions.is_empty() {
            writeln!(output, "    <h3>Functions</h3>").unwrap();
            writeln!(output, "    <ul>").unwrap();
            for item in functions {
                let anchor = Self::make_anchor(&item.name);
                writeln!(
                    output,
                    "      <li><a href=\"#{}\">{}</a></li>",
                    anchor, item.name
                )
                .unwrap();
            }
            writeln!(output, "    </ul>").unwrap();
        }

        let structs: Vec<_> = module.structs().collect();
        if !structs.is_empty() {
            writeln!(output, "    <h3>Structs</h3>").unwrap();
            writeln!(output, "    <ul>").unwrap();
            for item in structs {
                let anchor = Self::make_anchor(&item.name);
                writeln!(
                    output,
                    "      <li><a href=\"#{}\">{}</a></li>",
                    anchor, item.name
                )
                .unwrap();
            }
            writeln!(output, "    </ul>").unwrap();
        }

        let enums: Vec<_> = module.enums().collect();
        if !enums.is_empty() {
            writeln!(output, "    <h3>Enums</h3>").unwrap();
            writeln!(output, "    <ul>").unwrap();
            for item in enums {
                let anchor = Self::make_anchor(&item.name);
                writeln!(
                    output,
                    "      <li><a href=\"#{}\">{}</a></li>",
                    anchor, item.name
                )
                .unwrap();
            }
            writeln!(output, "    </ul>").unwrap();
        }

        let interfaces: Vec<_> = module.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(output, "    <h3>Interfaces</h3>").unwrap();
            writeln!(output, "    <ul>").unwrap();
            for item in interfaces {
                let anchor = Self::make_anchor(&item.name);
                writeln!(
                    output,
                    "      <li><a href=\"#{}\">{}</a></li>",
                    anchor, item.name
                )
                .unwrap();
            }
            writeln!(output, "    </ul>").unwrap();
        }

        writeln!(output, "  </nav>").unwrap();
    }

    fn write_item(output: &mut String, item: &DocumentedItem) {
        let anchor = Self::make_anchor(&item.name);

        writeln!(output, "  <div class=\"item\" id=\"{}\">", anchor).unwrap();
        writeln!(
            output,
            "    <h3><code>{}</code></h3>",
            Self::escape_html(&item.name)
        )
        .unwrap();

        // Signature
        writeln!(output, "    <div class=\"signature\">").unwrap();
        writeln!(output, "      {}", Self::escape_html(&item.signature)).unwrap();
        writeln!(output, "    </div>").unwrap();

        // Documentation
        if let Some(doc) = &item.doc {
            if !doc.summary.is_empty() {
                writeln!(
                    output,
                    "    <p class=\"summary\">{}</p>",
                    Self::escape_html(&doc.summary)
                )
                .unwrap();
            }

            if let Some(desc) = &doc.description {
                writeln!(
                    output,
                    "    <div class=\"description\">{}</div>",
                    Self::escape_html(desc)
                )
                .unwrap();
            }

            // Parameters
            if !doc.params.is_empty() {
                writeln!(output, "    <div class=\"params\">").unwrap();
                writeln!(output, "      <h4>Arguments</h4>").unwrap();
                writeln!(output, "      <ul>").unwrap();
                for (name, param) in &doc.params {
                    writeln!(
                        output,
                        "        <li><code>{}</code>: {}</li>",
                        Self::escape_html(name),
                        Self::escape_html(&param.description)
                    )
                    .unwrap();
                }
                writeln!(output, "      </ul>").unwrap();
                writeln!(output, "    </div>").unwrap();
            }

            // Returns
            if let Some(returns) = &doc.returns {
                writeln!(output, "    <div class=\"returns\">").unwrap();
                writeln!(output, "      <h4>Returns</h4>").unwrap();
                writeln!(output, "      <p>{}</p>", Self::escape_html(returns)).unwrap();
                writeln!(output, "    </div>").unwrap();
            }

            // Throws
            if !doc.throws.is_empty() {
                writeln!(output, "    <div class=\"throws\">").unwrap();
                writeln!(output, "      <h4>Throws</h4>").unwrap();
                writeln!(output, "      <ul>").unwrap();
                for throw in &doc.throws {
                    writeln!(output, "        <li>{}</li>", Self::escape_html(throw)).unwrap();
                }
                writeln!(output, "      </ul>").unwrap();
                writeln!(output, "    </div>").unwrap();
            }

            // Examples
            if !doc.examples.is_empty() {
                writeln!(output, "    <div class=\"example\">").unwrap();
                writeln!(output, "      <h4>Example</h4>").unwrap();
                for example in &doc.examples {
                    writeln!(output, "      <pre>{}</pre>", Self::escape_html(example)).unwrap();
                }
                writeln!(output, "    </div>").unwrap();
            }
        }

        // Children
        if !item.children.is_empty() {
            let child_type = match item.kind {
                ItemKind::Struct => "Fields",
                ItemKind::Enum => "Variants",
                ItemKind::Interface | ItemKind::Impl => "Methods",
                _ => "Members",
            };

            writeln!(output, "    <div class=\"children\">").unwrap();
            writeln!(output, "      <h4>{}</h4>", child_type).unwrap();
            writeln!(output, "      <ul>").unwrap();
            for child in &item.children {
                writeln!(
                    output,
                    "        <li><code>{}</code></li>",
                    Self::escape_html(&child.signature)
                )
                .unwrap();
            }
            writeln!(output, "      </ul>").unwrap();
            writeln!(output, "    </div>").unwrap();
        }

        writeln!(output, "  </div>").unwrap();
    }

    fn make_anchor(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect()
    }

    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::DocExtractor;
    use crate::Parser;

    #[test]
    fn test_generate_html() {
        let source = r#"
/// A simple greeting function.
fx greet(name: String) -> String {
    "Hello, {name}!"
}
"#;

        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "greeting");
        let html = HtmlGenerator::generate(&doc_module);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>greeting - Stratum Documentation</title>"));
        assert!(html.contains("greet"));
        assert!(html.contains("A simple greeting function"));
    }
}
