//! Search index generation for documentation
//!
//! This module generates a JSON search index and JavaScript search
//! functionality for client-side documentation search.

use super::project::ProjectDoc;
use super::types::ItemKind;

/// A single entry in the search index
#[derive(Debug, Clone)]
pub struct SearchEntry {
    /// Symbol name
    pub name: String,
    /// Item type (function, struct, enum, etc.)
    pub kind: &'static str,
    /// Module path
    pub module: String,
    /// Brief description
    pub description: String,
    /// Link to the item
    pub link: String,
}

/// Generate a JSON search index from project documentation
pub fn generate_search_index(project: &ProjectDoc) -> String {
    let mut entries = Vec::new();

    for symbol in project.all_symbols() {
        entries.push(SearchEntry {
            name: symbol.name.clone(),
            kind: kind_to_str(symbol.kind),
            module: symbol.module.clone(),
            description: symbol.description.clone(),
            link: format!("{}.html#{}", symbol.module, symbol.anchor),
        });
    }

    // Generate JSON
    let mut json = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            json.push_str(",\n");
        }
        json.push_str(&format!(
            r#"  {{"n":"{}","k":"{}","m":"{}","d":"{}","l":"{}"}}"#,
            escape_json(&entry.name),
            entry.kind,
            escape_json(&entry.module),
            escape_json(&entry.description),
            escape_json(&entry.link)
        ));
    }
    json.push_str("\n]");

    json
}

/// Generate the JavaScript search implementation
pub fn generate_search_js() -> &'static str {
    r#"// Stratum Documentation Search
(function() {
    'use strict';

    let searchIndex = [];
    let searchInput = null;
    let searchResults = null;

    // Load search index
    function loadSearchIndex() {
        fetch('search-index.json')
            .then(r => r.json())
            .then(data => { searchIndex = data; })
            .catch(e => console.warn('Search index not available:', e));
    }

    // Fuzzy match score (lower is better, -1 means no match)
    function fuzzyMatch(query, text) {
        query = query.toLowerCase();
        text = text.toLowerCase();

        // Exact match is best
        if (text === query) return 0;

        // Starts with is very good
        if (text.startsWith(query)) return 1;

        // Contains is good
        if (text.includes(query)) return 2;

        // Fuzzy match: all query chars must appear in order
        let qi = 0;
        let score = 0;
        let lastMatch = -1;

        for (let i = 0; i < text.length && qi < query.length; i++) {
            if (text[i] === query[qi]) {
                // Penalize gaps
                if (lastMatch >= 0) {
                    score += (i - lastMatch - 1) * 0.1;
                }
                lastMatch = i;
                qi++;
            }
        }

        // All query chars must be found
        if (qi < query.length) return -1;

        // Add base score for fuzzy matches
        return 3 + score;
    }

    // Search and display results
    function search(query) {
        if (!searchResults) return;

        query = query.trim();
        if (query.length < 2) {
            searchResults.innerHTML = '';
            searchResults.classList.remove('visible');
            return;
        }

        const results = [];
        for (const entry of searchIndex) {
            // Search in name (primary) and description (secondary)
            const nameScore = fuzzyMatch(query, entry.n);
            const descScore = fuzzyMatch(query, entry.d);

            // Take best score, prefer name matches
            let score = nameScore;
            if (score < 0 && descScore >= 0) {
                score = descScore + 10; // Penalize description-only matches
            }

            if (score >= 0) {
                results.push({ entry, score });
            }
        }

        // Sort by score (ascending - lower is better)
        results.sort((a, b) => a.score - b.score);

        // Display top 15 results
        const topResults = results.slice(0, 15);

        if (topResults.length === 0) {
            searchResults.innerHTML = '<div class="search-no-results">No results found</div>';
        } else {
            searchResults.innerHTML = topResults.map(r => {
                const e = r.entry;
                const kindClass = 'kind-' + e.k.toLowerCase();
                return `<a href="${e.l}" class="search-result">
                    <span class="search-result-kind ${kindClass}">${e.k}</span>
                    <span class="search-result-name">${highlight(e.n, query)}</span>
                    <span class="search-result-module">${e.m}</span>
                    <span class="search-result-desc">${truncate(e.d, 60)}</span>
                </a>`;
            }).join('');
        }

        searchResults.classList.add('visible');
    }

    // Highlight matching text
    function highlight(text, query) {
        const lower = text.toLowerCase();
        const queryLower = query.toLowerCase();
        const idx = lower.indexOf(queryLower);
        if (idx >= 0) {
            return text.substring(0, idx) +
                   '<mark>' + text.substring(idx, idx + query.length) + '</mark>' +
                   text.substring(idx + query.length);
        }
        return text;
    }

    // Truncate text
    function truncate(text, maxLen) {
        if (text.length <= maxLen) return text;
        return text.substring(0, maxLen) + '...';
    }

    // Initialize search
    function initSearch() {
        searchInput = document.getElementById('search-input');
        searchResults = document.getElementById('search-results');

        if (!searchInput || !searchResults) return;

        loadSearchIndex();

        // Debounce search input
        let debounceTimer;
        searchInput.addEventListener('input', function() {
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => search(this.value), 150);
        });

        // Keyboard navigation
        searchInput.addEventListener('keydown', function(e) {
            if (e.key === 'Escape') {
                this.value = '';
                searchResults.classList.remove('visible');
            }
        });

        // Close results when clicking outside
        document.addEventListener('click', function(e) {
            if (!e.target.closest('.search-container')) {
                searchResults.classList.remove('visible');
            }
        });

        // Focus search on '/' key
        document.addEventListener('keydown', function(e) {
            if (e.key === '/' && document.activeElement !== searchInput) {
                e.preventDefault();
                searchInput.focus();
            }
        });
    }

    // Run on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initSearch);
    } else {
        initSearch();
    }
})();
"#
}

/// Generate CSS for search UI
pub fn generate_search_css() -> &'static str {
    r#"
/* Search Styles */
.search-container {
    position: relative;
    margin: 1rem 0;
}

#search-input {
    width: 100%;
    padding: 0.75rem 1rem;
    padding-left: 2.5rem;
    font-size: 1rem;
    background: var(--code-bg);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-color);
    outline: none;
}

#search-input:focus {
    border-color: var(--accent-color);
    box-shadow: 0 0 0 2px rgba(123, 104, 238, 0.2);
}

#search-input::placeholder {
    color: #666;
}

.search-icon {
    position: absolute;
    left: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    color: #666;
    pointer-events: none;
}

#search-results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    margin-top: 0.5rem;
    max-height: 400px;
    overflow-y: auto;
    z-index: 1000;
    display: none;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

#search-results.visible {
    display: block;
}

.search-result {
    display: block;
    padding: 0.75rem 1rem;
    text-decoration: none;
    color: var(--text-color);
    border-bottom: 1px solid var(--border-color);
}

.search-result:last-child {
    border-bottom: none;
}

.search-result:hover {
    background: var(--code-bg);
}

.search-result-kind {
    display: inline-block;
    padding: 0.15rem 0.4rem;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    border-radius: 3px;
    margin-right: 0.5rem;
}

.kind-function { background: #2d5a27; color: #7ec876; }
.kind-struct { background: #5a4427; color: #d4a656; }
.kind-enum { background: #27485a; color: #56b4d4; }
.kind-interface { background: #4a275a; color: #c476d4; }
.kind-method { background: #275a4a; color: #56d4b4; }
.kind-constant { background: #5a2727; color: #d47676; }

.search-result-name {
    font-weight: 600;
    color: var(--accent-color);
}

.search-result-name mark {
    background: rgba(123, 104, 238, 0.3);
    color: inherit;
    padding: 0 2px;
    border-radius: 2px;
}

.search-result-module {
    margin-left: 0.5rem;
    font-size: 0.85rem;
    color: #888;
}

.search-result-desc {
    display: block;
    font-size: 0.85rem;
    color: #999;
    margin-top: 0.25rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.search-no-results {
    padding: 1rem;
    text-align: center;
    color: #888;
}

/* Keyboard hint */
.search-hint {
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.75rem;
    color: #666;
    background: var(--bg-color);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    border: 1px solid var(--border-color);
}

/* Type link styles */
.type-link {
    color: var(--accent-color);
    text-decoration: none;
    border-bottom: 1px dotted var(--accent-color);
}

.type-link:hover {
    border-bottom-style: solid;
}
"#
}

fn kind_to_str(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Function => "function",
        ItemKind::Struct => "struct",
        ItemKind::Field => "field",
        ItemKind::Enum => "enum",
        ItemKind::Variant => "variant",
        ItemKind::Interface => "interface",
        ItemKind::Method => "method",
        ItemKind::Impl => "impl",
        ItemKind::Constant => "constant",
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::project::ProjectDoc;
    use crate::doc::types::{DocumentedItem, DocumentedModule, ItemKind};

    #[test]
    fn test_generate_search_index() {
        let mut project = ProjectDoc::new("test");
        let mut module = DocumentedModule::new("math".to_string());
        module.add_item(DocumentedItem::new(
            "add".to_string(),
            ItemKind::Function,
            "fx add(a: Int, b: Int) -> Int".to_string(),
        ));
        project.add_module(module);

        let json = generate_search_index(&project);

        assert!(json.contains("\"n\":\"add\""));
        assert!(json.contains("\"k\":\"function\""));
        assert!(json.contains("\"m\":\"math\""));
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("he\"llo"), "he\\\"llo");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }
}
