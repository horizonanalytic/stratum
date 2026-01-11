//! HTML documentation generator

use std::fmt::Write;

use super::crosslink::{CrossLinkConfig, CrossLinker};
use super::project::ProjectDoc;
use super::search::{generate_search_css, generate_search_js};
use super::types::{DocumentedItem, DocumentedModule, ItemKind};

/// Generates HTML documentation from extracted documentation
pub struct HtmlGenerator;

/// Options for HTML generation
#[derive(Debug, Clone, Default)]
pub struct HtmlOptions {
    /// Enable search functionality
    pub enable_search: bool,
    /// Enable cross-linking between types
    pub enable_crosslinks: bool,
}

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
        writeln!(
            output,
            "  <title>{} - Stratum Documentation</title>",
            module.name
        )
        .unwrap();
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
                writeln!(
                    output,
                    "  <p class=\"summary\">{}</p>",
                    Self::escape_html(&doc.summary)
                )
                .unwrap();
            }
            if let Some(desc) = &doc.description {
                writeln!(
                    output,
                    "  <div class=\"description\">{}</div>",
                    Self::escape_html(desc)
                )
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
        let impls: Vec<_> = module
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Impl)
            .collect();
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

    /// Generate HTML documentation for a module with cross-linking and search support
    pub fn generate_with_project(
        module: &DocumentedModule,
        project: &ProjectDoc,
        options: &HtmlOptions,
    ) -> String {
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
        writeln!(
            output,
            "  <title>{} - {} Documentation</title>",
            module.name, project.name
        )
        .unwrap();
        Self::write_styles(&mut output);
        if options.enable_search {
            writeln!(output, "<style>{}</style>", generate_search_css()).unwrap();
        }
        writeln!(output, "</head>").unwrap();
        writeln!(output, "<body>").unwrap();

        // Navigation sidebar
        writeln!(output, "<nav class=\"sidebar\">").unwrap();
        writeln!(output, "  <div class=\"sidebar-header\">").unwrap();
        writeln!(
            output,
            "    <h2><a href=\"index.html\">{}</a></h2>",
            project.name
        )
        .unwrap();
        writeln!(output, "  </div>").unwrap();

        // Search box
        if options.enable_search {
            writeln!(output, "  <div class=\"search-container\">").unwrap();
            writeln!(output, "    <span class=\"search-icon\">&#128269;</span>").unwrap();
            writeln!(
                output,
                "    <input type=\"text\" id=\"search-input\" placeholder=\"Search...\" autocomplete=\"off\">"
            )
            .unwrap();
            writeln!(output, "    <span class=\"search-hint\">/</span>").unwrap();
            writeln!(output, "    <div id=\"search-results\"></div>").unwrap();
            writeln!(output, "  </div>").unwrap();
        }

        // Module list
        writeln!(output, "  <div class=\"module-list\">").unwrap();
        writeln!(output, "    <h3>Modules</h3>").unwrap();
        writeln!(output, "    <ul>").unwrap();
        for m in &project.modules {
            let class = if m.name == module.name {
                " class=\"active\""
            } else {
                ""
            };
            writeln!(
                output,
                "      <li><a href=\"{}.html\"{}>{}</a></li>",
                m.name, class, m.name
            )
            .unwrap();
        }
        writeln!(output, "    </ul>").unwrap();
        writeln!(output, "  </div>").unwrap();

        Self::write_nav(&mut output, module);
        writeln!(output, "</nav>").unwrap();

        // Main content
        writeln!(output, "<main class=\"content\">").unwrap();

        // Module header
        writeln!(output, "<header>").unwrap();
        writeln!(output, "  <h1>{}</h1>", module.name).unwrap();
        writeln!(output, "</header>").unwrap();

        // Create cross-linker if enabled
        let linker = if options.enable_crosslinks {
            Some(CrossLinker::new(
                project,
                CrossLinkConfig {
                    current_module: module.name.clone(),
                    link_external: true,
                },
            ))
        } else {
            None
        };

        // Module documentation
        if let Some(doc) = &module.doc {
            writeln!(output, "<section class=\"module-doc\">").unwrap();
            if !doc.summary.is_empty() {
                let summary = if let Some(ref l) = linker {
                    l.link_description(&doc.summary)
                } else {
                    Self::escape_html(&doc.summary)
                };
                writeln!(output, "  <p class=\"summary\">{}</p>", summary).unwrap();
            }
            if let Some(desc) = &doc.description {
                let desc_html = if let Some(ref l) = linker {
                    l.link_description(desc)
                } else {
                    Self::escape_html(desc)
                };
                writeln!(output, "  <div class=\"description\">{}</div>", desc_html).unwrap();
            }
            writeln!(output, "</section>").unwrap();
        }

        // Functions
        let functions: Vec<_> = module.functions().collect();
        if !functions.is_empty() {
            writeln!(output, "<section id=\"functions\">").unwrap();
            writeln!(output, "  <h2>Functions</h2>").unwrap();
            for item in functions {
                Self::write_item_with_links(&mut output, item, linker.as_ref());
            }
            writeln!(output, "</section>").unwrap();
        }

        // Structs
        let structs: Vec<_> = module.structs().collect();
        if !structs.is_empty() {
            writeln!(output, "<section id=\"structs\">").unwrap();
            writeln!(output, "  <h2>Structs</h2>").unwrap();
            for item in structs {
                Self::write_item_with_links(&mut output, item, linker.as_ref());
            }
            writeln!(output, "</section>").unwrap();
        }

        // Enums
        let enums: Vec<_> = module.enums().collect();
        if !enums.is_empty() {
            writeln!(output, "<section id=\"enums\">").unwrap();
            writeln!(output, "  <h2>Enums</h2>").unwrap();
            for item in enums {
                Self::write_item_with_links(&mut output, item, linker.as_ref());
            }
            writeln!(output, "</section>").unwrap();
        }

        // Interfaces
        let interfaces: Vec<_> = module.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(output, "<section id=\"interfaces\">").unwrap();
            writeln!(output, "  <h2>Interfaces</h2>").unwrap();
            for item in interfaces {
                Self::write_item_with_links(&mut output, item, linker.as_ref());
            }
            writeln!(output, "</section>").unwrap();
        }

        // Implementations
        let impls: Vec<_> = module
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Impl)
            .collect();
        if !impls.is_empty() {
            writeln!(output, "<section id=\"implementations\">").unwrap();
            writeln!(output, "  <h2>Implementations</h2>").unwrap();
            for item in impls {
                Self::write_item_with_links(&mut output, item, linker.as_ref());
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
                Self::write_item_with_links(&mut output, item, linker.as_ref());
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

        // Search script
        if options.enable_search {
            writeln!(output, "<script>{}</script>", generate_search_js()).unwrap();
        }

        writeln!(output, "</body>").unwrap();
        writeln!(output, "</html>").unwrap();

        output
    }

    /// Generate an index page for the project
    pub fn generate_index(project: &ProjectDoc, options: &HtmlOptions) -> String {
        let mut output = String::new();

        writeln!(output, "<!DOCTYPE html>").unwrap();
        writeln!(output, "<html lang=\"en\">").unwrap();
        writeln!(output, "<head>").unwrap();
        writeln!(output, "  <meta charset=\"UTF-8\">").unwrap();
        writeln!(
            output,
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )
        .unwrap();
        writeln!(output, "  <title>{} Documentation</title>", project.name).unwrap();
        Self::write_styles(&mut output);
        if options.enable_search {
            writeln!(output, "<style>{}</style>", generate_search_css()).unwrap();
        }
        writeln!(output, "</head>").unwrap();
        writeln!(output, "<body>").unwrap();

        // Sidebar
        writeln!(output, "<nav class=\"sidebar\">").unwrap();
        writeln!(output, "  <div class=\"sidebar-header\">").unwrap();
        writeln!(output, "    <h2>{}</h2>", project.name).unwrap();
        writeln!(output, "  </div>").unwrap();

        if options.enable_search {
            writeln!(output, "  <div class=\"search-container\">").unwrap();
            writeln!(output, "    <span class=\"search-icon\">&#128269;</span>").unwrap();
            writeln!(
                output,
                "    <input type=\"text\" id=\"search-input\" placeholder=\"Search...\" autocomplete=\"off\">"
            )
            .unwrap();
            writeln!(output, "    <span class=\"search-hint\">/</span>").unwrap();
            writeln!(output, "    <div id=\"search-results\"></div>").unwrap();
            writeln!(output, "  </div>").unwrap();
        }

        writeln!(output, "  <div class=\"module-list\">").unwrap();
        writeln!(output, "    <h3>Modules</h3>").unwrap();
        writeln!(output, "    <ul>").unwrap();
        for m in &project.modules {
            writeln!(
                output,
                "      <li><a href=\"{}.html\">{}</a></li>",
                m.name, m.name
            )
            .unwrap();
        }
        writeln!(output, "    </ul>").unwrap();
        writeln!(output, "  </div>").unwrap();
        writeln!(output, "</nav>").unwrap();

        // Main content
        writeln!(output, "<main class=\"content\">").unwrap();
        writeln!(output, "<header>").unwrap();
        writeln!(output, "  <h1>{}</h1>", project.name).unwrap();
        writeln!(output, "</header>").unwrap();

        // Overview
        writeln!(output, "<section class=\"overview\">").unwrap();
        writeln!(output, "  <h2>Modules</h2>").unwrap();
        writeln!(output, "  <div class=\"module-grid\">").unwrap();
        for m in &project.modules {
            let desc = m
                .doc
                .as_ref()
                .map(|d| d.summary.clone())
                .unwrap_or_default();
            writeln!(
                output,
                "    <a href=\"{}.html\" class=\"module-card\">",
                m.name
            )
            .unwrap();
            writeln!(output, "      <h3>{}</h3>", m.name).unwrap();
            writeln!(output, "      <p>{}</p>", Self::escape_html(&desc)).unwrap();
            // Item counts
            let fn_count = m.functions().count();
            let struct_count = m.structs().count();
            let enum_count = m.enums().count();
            writeln!(output, "      <div class=\"module-stats\">").unwrap();
            if fn_count > 0 {
                writeln!(output, "        <span>{} functions</span>", fn_count).unwrap();
            }
            if struct_count > 0 {
                writeln!(output, "        <span>{} structs</span>", struct_count).unwrap();
            }
            if enum_count > 0 {
                writeln!(output, "        <span>{} enums</span>", enum_count).unwrap();
            }
            writeln!(output, "      </div>").unwrap();
            writeln!(output, "    </a>").unwrap();
        }
        writeln!(output, "  </div>").unwrap();
        writeln!(output, "</section>").unwrap();

        // All symbols summary
        writeln!(output, "<section class=\"all-symbols\">").unwrap();
        writeln!(output, "  <h2>All Symbols</h2>").unwrap();
        writeln!(output, "  <div class=\"symbol-list\">").unwrap();
        for symbol in project.all_symbols().iter().take(50) {
            let kind_class = format!("kind-{}", symbol.kind.display_name().to_lowercase());
            writeln!(
                output,
                "    <a href=\"{}.html#{}\" class=\"symbol-item\">",
                symbol.module, symbol.anchor
            )
            .unwrap();
            writeln!(
                output,
                "      <span class=\"symbol-kind {}\">{}</span>",
                kind_class,
                symbol.kind.display_name()
            )
            .unwrap();
            writeln!(
                output,
                "      <span class=\"symbol-name\">{}</span>",
                symbol.name
            )
            .unwrap();
            writeln!(
                output,
                "      <span class=\"symbol-module\">{}</span>",
                symbol.module
            )
            .unwrap();
            writeln!(output, "    </a>").unwrap();
        }
        if project.all_symbols().len() > 50 {
            writeln!(
                output,
                "    <p class=\"more-symbols\">...and {} more symbols</p>",
                project.all_symbols().len() - 50
            )
            .unwrap();
        }
        writeln!(output, "  </div>").unwrap();
        writeln!(output, "</section>").unwrap();

        writeln!(output, "</main>").unwrap();

        // Footer
        writeln!(output, "<footer>").unwrap();
        writeln!(
            output,
            "  <p>Generated by <a href=\"https://stratum-lang.org\">Stratum</a></p>"
        )
        .unwrap();
        writeln!(output, "</footer>").unwrap();

        if options.enable_search {
            writeln!(output, "<script>{}</script>", generate_search_js()).unwrap();
        }

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

/* Module list in sidebar */
.module-list {{
  padding: 1rem;
  border-bottom: 1px solid var(--border-color);
}}

.module-list h3 {{
  font-size: 0.875rem;
  text-transform: uppercase;
  color: #888;
  margin-bottom: 0.5rem;
}}

.module-list ul {{
  list-style: none;
}}

.module-list li {{
  margin: 0.25rem 0;
}}

.module-list a {{
  color: var(--text-color);
  text-decoration: none;
}}

.module-list a:hover, .module-list a.active {{
  color: var(--accent-color);
}}

/* Module grid on index page */
.module-grid {{
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
}}

.module-card {{
  display: block;
  padding: 1.25rem;
  background: var(--code-bg);
  border-radius: 8px;
  border: 1px solid var(--border-color);
  text-decoration: none;
  color: var(--text-color);
  transition: border-color 0.2s, transform 0.2s;
}}

.module-card:hover {{
  border-color: var(--accent-color);
  transform: translateY(-2px);
}}

.module-card h3 {{
  color: var(--accent-color);
  font-size: 1.2rem;
  margin-bottom: 0.5rem;
}}

.module-card p {{
  font-size: 0.9rem;
  color: #bbb;
  margin-bottom: 0.75rem;
}}

.module-stats {{
  display: flex;
  gap: 1rem;
  font-size: 0.8rem;
  color: #888;
}}

/* All symbols list on index */
.symbol-list {{
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}}

.symbol-item {{
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  text-decoration: none;
  color: var(--text-color);
  border-radius: 4px;
}}

.symbol-item:hover {{
  background: var(--code-bg);
}}

.symbol-kind {{
  display: inline-block;
  padding: 0.1rem 0.4rem;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  border-radius: 3px;
  min-width: 70px;
  text-align: center;
}}

.kind-function {{ background: #2d5a27; color: #7ec876; }}
.kind-struct {{ background: #5a4427; color: #d4a656; }}
.kind-enum {{ background: #27485a; color: #56b4d4; }}
.kind-interface {{ background: #4a275a; color: #c476d4; }}
.kind-method {{ background: #275a4a; color: #56d4b4; }}
.kind-constant {{ background: #5a2727; color: #d47676; }}
.kind-field {{ background: #3d3d3d; color: #999; }}
.kind-variant {{ background: #27485a; color: #56b4d4; }}
.kind-implementation {{ background: #3d4a5a; color: #8ab4d4; }}

.symbol-name {{
  font-weight: 500;
  color: var(--accent-color);
}}

.symbol-module {{
  font-size: 0.85rem;
  color: #888;
  margin-left: auto;
}}

.more-symbols {{
  padding: 0.75rem;
  text-align: center;
  color: #888;
  font-style: italic;
}}

/* See also links */
.see-also {{
  margin: 0.75rem 0;
}}

.see-also h4 {{
  font-size: 0.9rem;
  color: #888;
  margin-bottom: 0.25rem;
}}

.see-also ul {{
  list-style: none;
  padding-left: 1rem;
}}

.see-also a {{
  color: var(--accent-color);
  text-decoration: none;
}}

.see-also a:hover {{
  text-decoration: underline;
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

    /// Write an item with cross-linking support
    fn write_item_with_links(
        output: &mut String,
        item: &DocumentedItem,
        linker: Option<&CrossLinker>,
    ) {
        let anchor = Self::make_anchor(&item.name);

        writeln!(output, "  <div class=\"item\" id=\"{}\">", anchor).unwrap();
        writeln!(
            output,
            "    <h3><code>{}</code></h3>",
            Self::escape_html(&item.name)
        )
        .unwrap();

        // Signature with cross-links
        writeln!(output, "    <div class=\"signature\">").unwrap();
        let sig_html = if let Some(l) = linker {
            l.link_signature(&item.signature)
        } else {
            Self::escape_html(&item.signature)
        };
        writeln!(output, "      {}", sig_html).unwrap();
        writeln!(output, "    </div>").unwrap();

        // Documentation
        if let Some(doc) = &item.doc {
            if !doc.summary.is_empty() {
                let summary = if let Some(l) = linker {
                    l.link_description(&doc.summary)
                } else {
                    Self::escape_html(&doc.summary)
                };
                writeln!(output, "    <p class=\"summary\">{}</p>", summary).unwrap();
            }

            if let Some(desc) = &doc.description {
                let desc_html = if let Some(l) = linker {
                    l.link_description(desc)
                } else {
                    Self::escape_html(desc)
                };
                writeln!(output, "    <div class=\"description\">{}</div>", desc_html).unwrap();
            }

            // Parameters
            if !doc.params.is_empty() {
                writeln!(output, "    <div class=\"params\">").unwrap();
                writeln!(output, "      <h4>Arguments</h4>").unwrap();
                writeln!(output, "      <ul>").unwrap();
                for (name, param) in &doc.params {
                    let desc = if let Some(l) = linker {
                        l.link_description(&param.description)
                    } else {
                        Self::escape_html(&param.description)
                    };
                    writeln!(
                        output,
                        "        <li><code>{}</code>: {}</li>",
                        Self::escape_html(name),
                        desc
                    )
                    .unwrap();
                }
                writeln!(output, "      </ul>").unwrap();
                writeln!(output, "    </div>").unwrap();
            }

            // Returns
            if let Some(returns) = &doc.returns {
                let ret_html = if let Some(l) = linker {
                    l.link_description(returns)
                } else {
                    Self::escape_html(returns)
                };
                writeln!(output, "    <div class=\"returns\">").unwrap();
                writeln!(output, "      <h4>Returns</h4>").unwrap();
                writeln!(output, "      <p>{}</p>", ret_html).unwrap();
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

            // See Also with resolved links
            if !doc.see_also.is_empty() {
                writeln!(output, "    <div class=\"see-also\">").unwrap();
                writeln!(output, "      <h4>See Also</h4>").unwrap();
                writeln!(output, "      <ul>").unwrap();
                for reference in &doc.see_also {
                    if let Some(l) = linker {
                        if let Some((text, link)) = l.resolve_see_also(reference) {
                            writeln!(output, "        <li><a href=\"{}\">{}</a></li>", link, text)
                                .unwrap();
                        } else {
                            writeln!(output, "        <li>{}</li>", Self::escape_html(reference))
                                .unwrap();
                        }
                    } else {
                        writeln!(output, "        <li>{}</li>", Self::escape_html(reference))
                            .unwrap();
                    }
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

        // Children with links
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
                let sig = if let Some(l) = linker {
                    l.link_signature(&child.signature)
                } else {
                    Self::escape_html(&child.signature)
                };
                writeln!(output, "        <li><code>{}</code></li>", sig).unwrap();
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

    #[test]
    fn test_generate_with_project() {
        let source1 = r#"
/// A user type.
struct User {
    name: String,
}

/// Create a new user.
fx create_user(name: String) -> User {
    User { name }
}
"#;

        let source2 = r#"
/// Greet a User by name.
fx greet(user: User) -> String {
    "Hello, {user.name}!"
}
"#;

        // Build project
        let mut project = ProjectDoc::new("TestProject");

        let module1 = Parser::parse_module(source1).unwrap();
        let doc_module1 = DocExtractor::extract(&module1, "users");
        project.add_module(doc_module1);

        let module2 = Parser::parse_module(source2).unwrap();
        let doc_module2 = DocExtractor::extract(&module2, "greeting");
        project.add_module(doc_module2);

        // Generate with project support
        let options = HtmlOptions {
            enable_search: true,
            enable_crosslinks: true,
        };

        let html = HtmlGenerator::generate_with_project(&project.modules[1], &project, &options);

        // Check basic structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>greeting - TestProject Documentation</title>"));

        // Check cross-linking - User should be linked
        assert!(html.contains("type-link"));
        assert!(html.contains("users.html#user"));

        // Check search UI is present
        assert!(html.contains("search-input"));
        assert!(html.contains("search-results"));

        // Check module list
        assert!(html.contains("users"));
        assert!(html.contains("greeting"));
    }

    #[test]
    fn test_generate_index() {
        let source = r#"
/// A math module.

/// Add two numbers.
fx add(a: Int, b: Int) -> Int {
    a + b
}

/// A point in 2D space.
struct Point {
    x: Int,
    y: Int,
}
"#;

        let mut project = ProjectDoc::new("MathLib");
        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "math");
        project.add_module(doc_module);

        let options = HtmlOptions {
            enable_search: true,
            enable_crosslinks: true,
        };

        let html = HtmlGenerator::generate_index(&project, &options);

        // Check structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>MathLib Documentation</title>"));

        // Check module grid
        assert!(html.contains("module-grid"));
        assert!(html.contains("math.html"));

        // Check symbol list
        assert!(html.contains("symbol-list"));
        assert!(html.contains("add"));
        assert!(html.contains("Point"));

        // Check search
        assert!(html.contains("search-input"));
    }
}
