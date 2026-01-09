//! Code actions implementation for Stratum LSP
//!
//! This module provides quick fixes and refactorings:
//! - Quick fixes for diagnostics (did-you-mean, missing fields, extra fields)
//! - Refactorings (extract variable)

use stratum_core::lexer::LineIndex;
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Position, Range, TextEdit, Url,
    WorkspaceEdit,
};

use std::collections::HashMap;

use crate::cache::CachedData;
use crate::definition::SymbolIndex;

/// Compute code actions using cached data
pub fn compute_code_actions_cached(
    uri: &Url,
    data: &CachedData<'_>,
    range: Range,
    diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Quick fixes based on diagnostics
    for diagnostic in diagnostics {
        if let Some(mut fixes) = compute_quick_fixes_cached(uri, data, diagnostic) {
            actions.append(&mut fixes);
        }
    }

    // Refactorings based on selection
    if let Some(mut refactors) = compute_refactorings(uri, data.content, range) {
        actions.append(&mut refactors);
    }

    actions
}

/// Compute quick fixes using cached data
fn compute_quick_fixes_cached(
    uri: &Url,
    data: &CachedData<'_>,
    diagnostic: &Diagnostic,
) -> Option<Vec<CodeActionOrCommand>> {
    let message = &diagnostic.message;
    let mut actions = Vec::new();

    // Use cached symbol index
    let index = data.symbol_index?;

    // Did-you-mean for undefined variable
    if message.contains("undefined variable") {
        if let Some(name) = extract_name_from_message(message, "undefined variable `", "`") {
            let suggestions = find_similar_symbols(index, &name, data.content, data.line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    // Did-you-mean for undefined function
    if message.contains("undefined function") {
        if let Some(name) = extract_name_from_message(message, "undefined function `", "`") {
            let suggestions = find_similar_symbols(index, &name, data.content, data.line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    // Did-you-mean for undefined type
    if message.contains("undefined type") {
        if let Some(name) = extract_name_from_message(message, "undefined type `", "`") {
            let suggestions = find_similar_symbols(index, &name, data.content, data.line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Compute code actions for the given range and diagnostics (non-cached)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_code_actions(
    uri: &Url,
    source: &str,
    range: Range,
    diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Quick fixes based on diagnostics
    for diagnostic in diagnostics {
        if let Some(mut fixes) = compute_quick_fixes(uri, source, diagnostic) {
            actions.append(&mut fixes);
        }
    }

    // Refactorings based on selection
    if let Some(mut refactors) = compute_refactorings(uri, source, range) {
        actions.append(&mut refactors);
    }

    actions
}

/// Compute quick fixes for a diagnostic
#[allow(dead_code)] // Called by compute_code_actions
fn compute_quick_fixes(
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) -> Option<Vec<CodeActionOrCommand>> {
    let message = &diagnostic.message;
    let mut actions = Vec::new();

    // Parse and type-check to get semantic information
    let module = Parser::parse_module(source).ok()?;
    let index = SymbolIndex::from_module(&module);
    let line_index = LineIndex::new(source);

    // Did-you-mean for undefined variable
    if message.contains("undefined variable") {
        if let Some(name) = extract_name_from_message(message, "undefined variable `", "`") {
            let suggestions = find_similar_symbols(&index, &name, source, &line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    // Did-you-mean for undefined function
    if message.contains("undefined function") {
        if let Some(name) = extract_name_from_message(message, "undefined function `", "`") {
            let suggestions = find_similar_symbols(&index, &name, source, &line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    // Did-you-mean for undefined type
    if message.contains("undefined type") {
        if let Some(name) = extract_name_from_message(message, "undefined type `", "`") {
            let suggestions = find_similar_symbols(&index, &name, source, &line_index, &diagnostic.range);
            for suggestion in suggestions {
                let action = create_replace_action(
                    uri,
                    &format!("Did you mean '{suggestion}'?"),
                    diagnostic.range,
                    &suggestion,
                    diagnostic,
                );
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
    }

    // Missing struct field
    if message.contains("missing field") {
        if let Some(fixes) = compute_missing_field_fix(uri, source, diagnostic, message) {
            actions.extend(fixes);
        }
    }

    // Extra struct field
    if message.contains("unknown field") {
        if let Some(fixes) = compute_extra_field_fix(uri, source, diagnostic) {
            actions.extend(fixes);
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Compute refactorings for a selection
fn compute_refactorings(
    uri: &Url,
    source: &str,
    range: Range,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();

    // Only offer extract variable if there's a non-empty selection
    if range.start != range.end {
        if let Some(action) = compute_extract_variable(uri, source, range) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Extract a name from an error message like "undefined variable `foo`"
fn extract_name_from_message(message: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = message.find(prefix)? + prefix.len();
    let rest = &message[start..];
    let end = rest.find(suffix)?;
    Some(rest[..end].to_string())
}

/// Find symbols similar to the given name using Levenshtein distance
fn find_similar_symbols(
    index: &SymbolIndex,
    name: &str,
    source: &str,
    line_index: &LineIndex,
    range: &Range,
) -> Vec<String> {
    // Convert range to offset for symbol lookup
    let offset = position_to_offset(line_index, range.start, source).unwrap_or(0);

    // Get all symbols visible at this position
    let all_symbols = index.all_symbols_matching("", offset);

    let mut suggestions: Vec<(String, usize)> = all_symbols
        .into_iter()
        .map(|(sym_name, _kind)| {
            let dist = levenshtein_distance(name, &sym_name);
            (sym_name, dist)
        })
        .filter(|(_, dist)| {
            // Only suggest if the distance is reasonable (within 3 edits or 40% of name length)
            let max_dist = (name.len() / 2).max(3);
            *dist <= max_dist
        })
        .collect();

    // Sort by distance
    suggestions.sort_by_key(|(_, dist)| *dist);

    // Return top 3 suggestions
    suggestions.into_iter().take(3).map(|(name, _)| name).collect()
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Create a code action that replaces text at a range
fn create_replace_action(
    uri: &Url,
    title: &str,
    range: Range,
    new_text: &str,
    diagnostic: &Diagnostic,
) -> CodeAction {
    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range,
            new_text: new_text.to_string(),
        }],
    );

    CodeAction {
        title: title.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        is_preferred: None,
        disabled: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        data: None,
    }
}

/// Compute fix for missing struct field
#[allow(dead_code)] // Called by compute_quick_fixes
fn compute_missing_field_fix(
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    message: &str,
) -> Option<Vec<CodeActionOrCommand>> {
    // Extract field name from message like:
    // "missing field `x` in struct `Point`"
    let field_name = extract_name_from_message(message, "missing field `", "`")?;

    // We need to verify this is parseable code
    let _module = Parser::parse_module(source).ok()?;

    // Find the closing brace of the struct init
    // We look for the line with the error and find a good insertion point
    let error_line = diagnostic.range.start.line as usize;
    let lines: Vec<&str> = source.lines().collect();

    if error_line >= lines.len() {
        return None;
    }

    // Find the position to insert (before the closing brace)
    // Look for `}` on this or following lines
    let mut insert_line = error_line;
    let mut insert_col = 0;

    for (i, line) in lines.iter().enumerate().skip(error_line) {
        if let Some(pos) = line.find('}') {
            insert_line = i;
            insert_col = pos;
            break;
        }
    }

    // Determine indentation from the line above
    let indent = if insert_line > 0 && insert_line < lines.len() {
        let prev_line = lines.get(insert_line.saturating_sub(1)).unwrap_or(&"");
        let spaces = prev_line.len() - prev_line.trim_start().len();
        " ".repeat(spaces)
    } else {
        "    ".to_string()
    };

    // Create the field text
    let needs_comma = if insert_line > 0 {
        let prev_line = lines.get(insert_line.saturating_sub(1)).unwrap_or(&"");
        let trimmed = prev_line.trim();
        !trimmed.is_empty() && !trimmed.ends_with(',') && !trimmed.ends_with('{')
    } else {
        false
    };

    let comma = if needs_comma { "," } else { "" };
    let field_text = format!("{comma}\n{indent}{field_name}: ");

    let insert_position = Position {
        line: insert_line as u32,
        character: insert_col as u32,
    };

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: Range {
                start: insert_position,
                end: insert_position,
            },
            new_text: field_text,
        }],
    );

    let action = CodeAction {
        title: format!("Add missing field '{field_name}'"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        is_preferred: Some(true),
        disabled: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        data: None,
    };

    Some(vec![CodeActionOrCommand::CodeAction(action)])
}

/// Compute fix for extra struct field (remove it)
#[allow(dead_code)] // Called by compute_quick_fixes
fn compute_extra_field_fix(
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) -> Option<Vec<CodeActionOrCommand>> {
    let lines: Vec<&str> = source.lines().collect();

    let error_line = diagnostic.range.start.line as usize;
    if error_line >= lines.len() {
        return None;
    }

    let line = lines[error_line];

    // Find the extent of this field assignment (from field name to comma or end)
    // We'll delete the entire line if it's just a field assignment
    let trimmed = line.trim();

    // Check if this line is just a field (possibly with comma)
    if trimmed.contains(':') {
        // Delete the entire line including the newline
        let start = Position {
            line: error_line as u32,
            character: 0,
        };
        let end = Position {
            line: (error_line + 1) as u32,
            character: 0,
        };

        let mut changes = HashMap::new();
        changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: Range { start, end },
                new_text: String::new(),
            }],
        );

        // Extract field name for the title
        let field_name = if let Some(colon_pos) = trimmed.find(':') {
            trimmed[..colon_pos].trim().to_string()
        } else {
            "field".to_string()
        };

        let action = CodeAction {
            title: format!("Remove unknown field '{field_name}'"),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            is_preferred: Some(true),
            disabled: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            data: None,
        };

        return Some(vec![CodeActionOrCommand::CodeAction(action)]);
    }

    None
}

/// Compute extract variable refactoring
fn compute_extract_variable(
    uri: &Url,
    source: &str,
    range: Range,
) -> Option<CodeAction> {
    let line_index = LineIndex::new(source);

    // Get the selected text
    let start_offset = position_to_offset(&line_index, range.start, source)?;
    let end_offset = position_to_offset(&line_index, range.end, source)?;

    if start_offset >= end_offset || end_offset > source.len() as u32 {
        return None;
    }

    let selected_text = &source[start_offset as usize..end_offset as usize];

    // Skip if selection is empty or just whitespace
    if selected_text.trim().is_empty() {
        return None;
    }

    // Skip if selection contains statements (newlines with non-expression content)
    // This is a simple heuristic - only allow single expressions
    if selected_text.contains('\n') && selected_text.contains("let ") {
        return None;
    }

    // Find the line above the selection to insert the variable
    let insert_line = range.start.line;
    let lines: Vec<&str> = source.lines().collect();

    // Get indentation from the current line
    let current_line = lines.get(insert_line as usize).unwrap_or(&"");
    let indent_len = current_line.len() - current_line.trim_start().len();
    let indent = " ".repeat(indent_len);

    // Create the variable binding
    let var_name = "extracted";
    let var_decl = format!("{indent}let {var_name} = {}\n", selected_text.trim());

    // Create edits:
    // 1. Insert variable declaration at the start of the current line
    // 2. Replace selected expression with variable name
    let insert_pos = Position {
        line: insert_line,
        character: 0,
    };

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![
            // Insert variable declaration
            TextEdit {
                range: Range {
                    start: insert_pos,
                    end: insert_pos,
                },
                new_text: var_decl,
            },
            // Replace selection with variable name
            // Note: after the insertion, the range shifts down by 1 line
            TextEdit {
                range: Range {
                    start: Position {
                        line: range.start.line + 1,
                        character: range.start.character,
                    },
                    end: Position {
                        line: range.end.line + 1,
                        character: range.end.character,
                    },
                },
                new_text: var_name.to_string(),
            },
        ],
    );

    Some(CodeAction {
        title: "Extract to variable".to_string(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        diagnostics: None,
        is_preferred: None,
        disabled: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        data: None,
    })
}

/// Convert an LSP Position to a byte offset
fn position_to_offset(line_index: &LineIndex, position: Position, source: &str) -> Option<u32> {
    let line = position.line as usize;
    let character = position.character as usize;

    // Get the byte offset of the start of the line
    let line_start = line_index.line_start(line)?;

    // Add character offset (being careful about UTF-8)
    let lines: Vec<&str> = source.lines().collect();
    if line >= lines.len() {
        return Some(source.len() as u32);
    }

    let line_text = lines[line];

    // Convert character (UTF-16 code units) to byte offset
    let mut byte_offset = 0;
    let mut char_count = 0;
    for c in line_text.chars() {
        if char_count >= character {
            break;
        }
        byte_offset += c.len_utf8();
        char_count += c.len_utf16();
    }

    Some(line_start + byte_offset as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_extract_name_from_message() {
        let msg = "undefined variable `foo`";
        assert_eq!(
            extract_name_from_message(msg, "undefined variable `", "`"),
            Some("foo".to_string())
        );

        let msg = "missing field `x` in struct `Point`";
        assert_eq!(
            extract_name_from_message(msg, "missing field `", "`"),
            Some("x".to_string())
        );
        assert_eq!(
            extract_name_from_message(msg, "in struct `", "`"),
            Some("Point".to_string())
        );
    }

    #[test]
    fn test_did_you_mean_suggestions() {
        let source = r#"
fx main() {
    let count = 10
    let counter = 20
    let result = cont
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Create a diagnostic for "undefined variable `cont`"
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 4, character: 17 },
                end: Position { line: 4, character: 21 },
            },
            severity: None,
            code: None,
            code_description: None,
            source: Some("stratum".to_string()),
            message: "undefined variable `cont`".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let actions = compute_code_actions(
            &uri,
            source,
            diagnostic.range,
            &[diagnostic],
        );

        // Should suggest "count" and "counter"
        assert!(!actions.is_empty());

        let titles: Vec<String> = actions
            .iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(ca) => Some(ca.title.clone()),
                _ => None,
            })
            .collect();

        assert!(titles.iter().any(|t| t.contains("count")));
    }

    #[test]
    fn test_extract_variable_action() {
        let source = r#"
fx main() {
    let result = 1 + 2 + 3
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Select "1 + 2" on line 2
        let range = Range {
            start: Position { line: 2, character: 17 },
            end: Position { line: 2, character: 22 },
        };

        let actions = compute_code_actions(&uri, source, range, &[]);

        // Should have extract variable action
        assert!(!actions.is_empty());

        let has_extract = actions.iter().any(|a| match a {
            CodeActionOrCommand::CodeAction(ca) => ca.title.contains("Extract"),
            _ => false,
        });

        assert!(has_extract);
    }
}
