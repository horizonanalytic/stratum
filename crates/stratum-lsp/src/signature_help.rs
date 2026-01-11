//! Signature help for Stratum function calls
//!
//! This module provides signature help functionality for the LSP server,
//! showing function parameter information while typing function arguments.

use stratum_core::ast::{Function, ItemKind, Module, TopLevelItem, TypeAnnotation, TypeKind};
use stratum_core::lexer::LineIndex;
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureInformation,
};

use crate::cache::CachedData;

/// Compute signature help using cached data
pub fn compute_signature_help_cached(
    data: &CachedData<'_>,
    position: Position,
) -> Option<SignatureHelp> {
    // Convert LSP position to byte offset
    let offset = position_to_offset(data.line_index, position)?;

    // Find the function call context at this position using text analysis
    let call_context = find_call_context_from_text(data.content, offset)?;

    // Try to find the function definition using cached AST if available
    let signature = if let Some(module) = data.ast() {
        if let Some(sig) = find_signature_from_ast(module, &call_context.function_name) {
            build_signature_info_from_ast(&sig, call_context.active_parameter)
        } else if let Some(sig) =
            find_signature_from_text(data.content, &call_context.function_name)
        {
            sig.with_active_parameter(call_context.active_parameter)
        } else {
            return None;
        }
    } else if let Some(sig) = find_signature_from_text(data.content, &call_context.function_name) {
        sig.with_active_parameter(call_context.active_parameter)
    } else {
        return None;
    };

    Some(SignatureHelp {
        signatures: vec![signature],
        active_signature: Some(0),
        active_parameter: Some(call_context.active_parameter),
    })
}

/// Compute signature help for a position in the source (non-cached)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_signature_help(source: &str, position: Position) -> Option<SignatureHelp> {
    let line_index = LineIndex::new(source);

    // Convert LSP position to byte offset
    let offset = position_to_offset(&line_index, position)?;

    // Find the function call context at this position using text analysis
    // This works even with incomplete code
    let call_context = find_call_context_from_text(source, offset)?;

    // Try to find the function definition
    // First try parsing, then fall back to text-based search
    let signature =
        if let Some(sig) = find_signature_from_parse(source, &call_context.function_name) {
            build_signature_info_from_ast(&sig, call_context.active_parameter)
        } else if let Some(sig) = find_signature_from_text(source, &call_context.function_name) {
            sig.with_active_parameter(call_context.active_parameter)
        } else {
            return None;
        };

    Some(SignatureHelp {
        signatures: vec![signature],
        active_signature: Some(0),
        active_parameter: Some(call_context.active_parameter),
    })
}

/// Context about a function call at the cursor position
struct CallContext {
    /// Name of the function being called
    function_name: String,
    /// Index of the parameter the cursor is on (0-indexed)
    active_parameter: u32,
}

/// Find the function call context using text analysis
/// This works even with incomplete/unparseable code
fn find_call_context_from_text(source: &str, offset: u32) -> Option<CallContext> {
    let offset = offset as usize;
    let text_before = &source[..offset.min(source.len())];

    // Find the last unclosed parenthesis
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut call_start = None;
    let mut comma_count = 0;

    // Scan backwards from cursor to find the function call context
    for (i, ch) in text_before.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                } else {
                    // Found an unclosed paren - this might be our function call
                    call_start = Some(i);
                    break;
                }
            }
            ']' => bracket_depth += 1,
            '[' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '}' => brace_depth += 1,
            '{' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
            ',' => {
                // Only count commas at the current nesting level
                if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                    comma_count += 1;
                }
            }
            _ => {}
        }
    }

    let call_start = call_start?;

    // Extract the function name by scanning backwards from the opening paren
    let text_before_paren = &source[..call_start];
    let function_name = extract_function_name_before_paren(text_before_paren)?;

    Some(CallContext {
        function_name,
        active_parameter: comma_count,
    })
}

/// Extract the function name from text ending just before an opening parenthesis
fn extract_function_name_before_paren(text: &str) -> Option<String> {
    // Trim trailing whitespace
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    // Find the start of the identifier
    let mut name_start = trimmed.len();
    for (i, ch) in trimmed.char_indices().rev() {
        if ch.is_alphanumeric() || ch == '_' {
            name_start = i;
        } else {
            break;
        }
    }

    let name = &trimmed[name_start..];
    if name.is_empty() {
        return None;
    }

    let first_char = name.chars().next()?;
    if !first_char.is_alphabetic() && first_char != '_' {
        return None;
    }

    // Make sure it's not a keyword that can be followed by parens (like if, while, for)
    let keywords = ["if", "while", "for", "match", "return", "throw"];
    if keywords.contains(&name) {
        return None;
    }

    Some(name.to_string())
}

/// Try to find a function signature by parsing the source
#[allow(dead_code)] // Called by compute_signature_help
fn find_signature_from_parse(source: &str, name: &str) -> Option<FunctionSig> {
    let module = Parser::parse_module(source).ok()?;
    let func = find_function_definition(&module, name)?;
    Some(FunctionSig::from_ast(func))
}

/// Find a function signature directly from the AST (for cached version)
fn find_signature_from_ast(module: &Module, name: &str) -> Option<FunctionSig> {
    let func = find_function_definition(module, name)?;
    Some(FunctionSig::from_ast(func))
}

/// Function signature information extracted from the code
struct FunctionSig {
    name: String,
    is_async: bool,
    params: Vec<ParamSig>,
    return_type: Option<String>,
}

/// Parameter signature information
struct ParamSig {
    name: String,
    ty: Option<String>,
}

impl FunctionSig {
    fn from_ast(func: &Function) -> Self {
        FunctionSig {
            name: func.name.name.clone(),
            is_async: func.is_async,
            params: func
                .params
                .iter()
                .map(|p| ParamSig {
                    name: p.name.name.clone(),
                    ty: p.ty.as_ref().map(type_annotation_to_string),
                })
                .collect(),
            return_type: func.return_type.as_ref().map(type_annotation_to_string),
        }
    }

    fn with_active_parameter(self, active_param: u32) -> SignatureInformation {
        // Build parameter strings
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.ty {
                    format!("{}: {}", p.name, ty)
                } else {
                    p.name.clone()
                }
            })
            .collect();

        // Build the full signature label
        let params_str = params.join(", ");
        let return_str = self
            .return_type
            .as_ref()
            .map(|ty| format!(" -> {}", ty))
            .unwrap_or_default();
        let async_str = if self.is_async { "async " } else { "" };
        let label = format!(
            "{}fx {}({}){}",
            async_str, self.name, params_str, return_str
        );

        // Build parameter information with offset labels
        let mut param_infos = Vec::new();
        let prefix_len = format!("{}fx {}(", async_str, self.name).len();
        let mut current_offset = prefix_len;

        for (i, param_str) in params.iter().enumerate() {
            let start = current_offset;
            let end = start + param_str.len();

            param_infos.push(ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None,
            });

            // Account for ", " between parameters
            current_offset = end;
            if i < params.len() - 1 {
                current_offset += 2; // ", "
            }
        }

        SignatureInformation {
            label,
            documentation: None,
            parameters: Some(param_infos),
            active_parameter: Some(active_param),
        }
    }
}

/// Find function signature using text-based search (fallback when parsing fails)
fn find_signature_from_text(source: &str, name: &str) -> Option<FunctionSig> {
    // Look for function definition pattern: (async)? fx <name>(<params>) (-> <type>)?
    // We'll use a simple state machine approach

    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Check for function definition
        let (is_async, rest) = if trimmed.starts_with("async ") {
            (true, trimmed.strip_prefix("async ")?.trim())
        } else {
            (false, trimmed)
        };

        if !rest.starts_with("fx ") {
            continue;
        }

        let after_fx = rest.strip_prefix("fx ")?.trim();

        // Find function name
        let name_end = after_fx.find('(')?;
        let func_name = after_fx[..name_end].trim();

        if func_name != name {
            continue;
        }

        // Found the function! Parse the signature
        let after_name = &after_fx[name_end..];

        // Find the matching closing paren for params
        let params_start = after_name.find('(')?;
        let mut paren_depth = 0;
        let mut params_end = None;

        for (i, ch) in after_name.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        params_end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let params_end = params_end?;
        let params_str = &after_name[params_start + 1..params_end];

        // Parse parameters
        let params = parse_params_from_text(params_str);

        // Parse return type
        let after_params = &after_name[params_end + 1..];
        let return_type = if let Some(arrow_pos) = after_params.find("->") {
            let ret_str = after_params[arrow_pos + 2..].trim();
            // Take until '{' or end of line
            let ret_end = ret_str.find('{').unwrap_or(ret_str.len());
            let ret_type = ret_str[..ret_end].trim();
            if ret_type.is_empty() {
                None
            } else {
                Some(ret_type.to_string())
            }
        } else {
            None
        };

        return Some(FunctionSig {
            name: name.to_string(),
            is_async,
            params,
            return_type,
        });
    }

    None
}

/// Parse parameters from a text string like "a: Int, b: String"
fn parse_params_from_text(params_str: &str) -> Vec<ParamSig> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }

    let mut params = Vec::new();
    let mut current_param = String::new();
    let mut angle_depth = 0;
    let mut paren_depth = 0;

    for ch in params_str.chars() {
        match ch {
            '<' => {
                angle_depth += 1;
                current_param.push(ch);
            }
            '>' => {
                angle_depth -= 1;
                current_param.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current_param.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current_param.push(ch);
            }
            ',' if angle_depth == 0 && paren_depth == 0 => {
                if !current_param.trim().is_empty() {
                    params.push(parse_single_param(&current_param));
                }
                current_param.clear();
            }
            _ => current_param.push(ch),
        }
    }

    if !current_param.trim().is_empty() {
        params.push(parse_single_param(&current_param));
    }

    params
}

/// Parse a single parameter like "name: Type" or just "name"
fn parse_single_param(param_str: &str) -> ParamSig {
    let trimmed = param_str.trim();
    if let Some(colon_pos) = trimmed.find(':') {
        let name = trimmed[..colon_pos].trim().to_string();
        let ty = trimmed[colon_pos + 1..].trim().to_string();
        ParamSig {
            name,
            ty: if ty.is_empty() { None } else { Some(ty) },
        }
    } else {
        ParamSig {
            name: trimmed.to_string(),
            ty: None,
        }
    }
}

/// Find a function definition by name in a parsed module
fn find_function_definition<'a>(module: &'a Module, name: &str) -> Option<&'a Function> {
    for item in &module.top_level {
        if let TopLevelItem::Item(item) = item {
            match &item.kind {
                ItemKind::Function(func) if func.name.name == name => {
                    return Some(func);
                }
                ItemKind::Impl(impl_def) => {
                    for method in &impl_def.methods {
                        if method.name.name == name {
                            return Some(method);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Build SignatureInformation from a function definition
fn build_signature_info_from_ast(sig: &FunctionSig, active_param: u32) -> SignatureInformation {
    sig.clone().with_active_parameter(active_param)
}

impl Clone for FunctionSig {
    fn clone(&self) -> Self {
        FunctionSig {
            name: self.name.clone(),
            is_async: self.is_async,
            params: self
                .params
                .iter()
                .map(|p| ParamSig {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                })
                .collect(),
            return_type: self.return_type.clone(),
        }
    }
}

/// Convert a type annotation to a display string
fn type_annotation_to_string(ty: &TypeAnnotation) -> String {
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if args.is_empty() {
                name.name.clone()
            } else {
                let args_str = args
                    .iter()
                    .map(type_annotation_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name.name, args_str)
            }
        }
        TypeKind::Nullable(inner) => format!("{}?", type_annotation_to_string(inner)),
        TypeKind::List(inner) => format!("[{}]", type_annotation_to_string(inner)),
        TypeKind::Tuple(types) => {
            let types_str = types
                .iter()
                .map(type_annotation_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({types_str})")
        }
        TypeKind::Function { params, ret } => {
            let params_str = params
                .iter()
                .map(type_annotation_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({params_str}) -> {}", type_annotation_to_string(ret))
        }
        TypeKind::Unit => "()".to_string(),
        TypeKind::Never => "!".to_string(),
        TypeKind::Inferred => "_".to_string(),
    }
}

/// Convert an LSP Position to a byte offset
fn position_to_offset(line_index: &LineIndex, position: Position) -> Option<u32> {
    let line = position.line as usize;
    let character = position.character as usize;
    let line_start = line_index.line_start(line)?;
    Some(line_start + character as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_help_simple_function() {
        let source = r#"
fx greet(name: String, times: Int) -> String {
    name
}

fx main() {
    greet(
}
"#;
        // Position inside the greet() call after the opening paren
        let position = Position {
            line: 6,
            character: 10,
        };

        let help = compute_signature_help(source, position);
        assert!(help.is_some(), "Expected signature help but got None");

        let sig_help = help.unwrap();
        assert_eq!(sig_help.signatures.len(), 1);

        let sig = &sig_help.signatures[0];
        assert!(sig.label.contains("greet"), "Label should contain 'greet'");
        assert!(
            sig.label.contains("name: String"),
            "Label should contain 'name: String'"
        );
        assert!(
            sig.label.contains("times: Int"),
            "Label should contain 'times: Int'"
        );
        assert_eq!(sig_help.active_parameter, Some(0));
    }

    #[test]
    fn test_signature_help_second_parameter() {
        let source = r#"
fx add(a: Int, b: Int) -> Int {
    a + b
}

fx main() {
    add(1,
}
"#;
        // Position after the comma (second parameter)
        let position = Position {
            line: 6,
            character: 10,
        };

        let help = compute_signature_help(source, position);
        assert!(help.is_some(), "Expected signature help but got None");

        let sig_help = help.unwrap();
        assert_eq!(sig_help.active_parameter, Some(1));
    }

    #[test]
    fn test_signature_help_no_call() {
        let source = r#"
fx main() {
    let x = 42
}
"#;
        let position = Position {
            line: 2,
            character: 10,
        };

        let help = compute_signature_help(source, position);
        // Should be None because we're not in a function call
        // (we're at "let x = 42" which has no unclosed paren)
        assert!(help.is_none());
    }

    #[test]
    fn test_signature_help_nested_call() {
        let source = r#"
fx inner(x: Int) -> Int { x }
fx outer(y: Int) -> Int { y }

fx main() {
    outer(inner(
}
"#;
        // Position inside inner() call
        let position = Position {
            line: 5,
            character: 16,
        };

        let help = compute_signature_help(source, position);
        assert!(help.is_some(), "Expected signature help but got None");

        let sig_help = help.unwrap();
        // Should show inner's signature because that's the innermost unclosed paren
        assert!(
            sig_help.signatures[0].label.contains("inner"),
            "Should show inner's signature"
        );
    }

    #[test]
    fn test_signature_help_with_async() {
        let source = r#"
async fx fetch(url: String) -> String {
    url
}

fx main() {
    fetch(
}
"#;
        let position = Position {
            line: 6,
            character: 10,
        };

        let help = compute_signature_help(source, position);
        assert!(help.is_some(), "Expected signature help but got None");

        let sig_help = help.unwrap();
        assert!(
            sig_help.signatures[0].label.contains("async"),
            "Should contain 'async'"
        );
    }

    #[test]
    fn test_signature_help_no_params() {
        let source = r#"
fx no_args() -> Int {
    42
}

fx main() {
    no_args(
}
"#;
        let position = Position {
            line: 6,
            character: 12,
        };

        let help = compute_signature_help(source, position);
        assert!(help.is_some(), "Expected signature help but got None");

        let sig_help = help.unwrap();
        assert!(
            sig_help.signatures[0].label.contains("no_args"),
            "Should contain 'no_args'"
        );
        assert_eq!(sig_help.active_parameter, Some(0));
    }

    #[test]
    fn test_signature_help_with_complete_call() {
        let source = r#"
fx add(a: Int, b: Int) -> Int {
    a + b
}

fx main() {
    let x = add(1, 2)
}
"#;
        // Position inside the parentheses (on the '1' argument)
        // Line 6: "    let x = add(1, 2)"
        // Chars:   0         1
        //          0123456789012345678901
        // Position 16 is '1' inside the parens
        let position = Position {
            line: 6,
            character: 16,
        };

        let help = compute_signature_help(source, position);
        // Should still work for complete calls when cursor is inside
        assert!(help.is_some(), "Should work for complete calls");
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(
            extract_function_name_before_paren("    greet"),
            Some("greet".to_string())
        );
        assert_eq!(
            extract_function_name_before_paren("foo.bar"),
            Some("bar".to_string())
        );
        assert_eq!(
            extract_function_name_before_paren("my_func"),
            Some("my_func".to_string())
        );
        assert_eq!(extract_function_name_before_paren("if"), None);
        assert_eq!(extract_function_name_before_paren("while"), None);
    }

    #[test]
    fn test_call_context_simple() {
        let source = "greet(";
        let context = find_call_context_from_text(source, 6);
        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "greet");
        assert_eq!(ctx.active_parameter, 0);
    }

    #[test]
    fn test_call_context_with_comma() {
        let source = "add(1, ";
        let context = find_call_context_from_text(source, 7);
        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "add");
        assert_eq!(ctx.active_parameter, 1);
    }

    #[test]
    fn test_call_context_nested() {
        let source = "outer(inner(";
        let context = find_call_context_from_text(source, 12);
        assert!(context.is_some());
        let ctx = context.unwrap();
        // Should find the innermost unclosed paren (inner)
        assert_eq!(ctx.function_name, "inner");
        assert_eq!(ctx.active_parameter, 0);
    }

    #[test]
    fn test_find_signature_from_text() {
        let source = r#"
fx greet(name: String, times: Int) -> String {
    name
}
"#;
        let sig = find_signature_from_text(source, "greet");
        assert!(sig.is_some());
        let sig = sig.unwrap();
        assert_eq!(sig.name, "greet");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].name, "name");
        assert_eq!(sig.params[0].ty, Some("String".to_string()));
        assert_eq!(sig.params[1].name, "times");
        assert_eq!(sig.params[1].ty, Some("Int".to_string()));
        assert_eq!(sig.return_type, Some("String".to_string()));
    }

    #[test]
    fn test_find_async_signature_from_text() {
        let source = r#"
async fx fetch(url: String) -> String {
    url
}
"#;
        let sig = find_signature_from_text(source, "fetch");
        assert!(sig.is_some());
        let sig = sig.unwrap();
        assert!(sig.is_async);
        assert_eq!(sig.name, "fetch");
    }

    #[test]
    fn test_parse_params_from_text() {
        let params = parse_params_from_text("a: Int, b: String");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[0].ty, Some("Int".to_string()));
        assert_eq!(params[1].name, "b");
        assert_eq!(params[1].ty, Some("String".to_string()));
    }

    #[test]
    fn test_parse_params_with_generics() {
        let params = parse_params_from_text("items: List<Int>, mapper: (Int) -> String");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "items");
        assert_eq!(params[0].ty, Some("List<Int>".to_string()));
    }
}
