//! Documentation extractor - walks AST and extracts documentation

use crate::ast::{
    EnumDef, Function, ImplDef, InterfaceDef, Item, ItemKind as AstItemKind, Module, StructDef,
    TopLevelItem, TypeAnnotation,
};

use super::types::{DocComment, DocumentedItem, DocumentedModule, ItemKind};

/// Extracts documentation from a parsed AST
pub struct DocExtractor;

impl DocExtractor {
    /// Extract documentation from a module AST
    pub fn extract(module: &Module, name: &str) -> DocumentedModule {
        let mut doc_module = DocumentedModule::new(name.to_string());

        // Extract module-level documentation from leading trivia
        if let Some(doc_text) = module.trivia.doc_text() {
            doc_module.doc = Some(DocComment::parse(&doc_text));
        }

        // Process all top-level items
        for top_level in &module.top_level {
            match top_level {
                TopLevelItem::Item(item) => {
                    if let Some(doc_item) = Self::extract_item(item) {
                        doc_module.add_item(doc_item);
                    }
                }
                TopLevelItem::Let(let_decl) => {
                    // Extract documentation for top-level constants
                    if let Some(doc_text) = let_decl.trivia.doc_text() {
                        let name = Self::pattern_to_name(&let_decl.pattern);
                        let sig = Self::format_let_signature(let_decl);
                        let mut item = DocumentedItem::new(name, ItemKind::Constant, sig);
                        item.doc = Some(DocComment::parse(&doc_text));
                        doc_module.add_item(item);
                    }
                }
                TopLevelItem::Statement(_) => {}
            }
        }

        doc_module
    }

    fn extract_item(item: &Item) -> Option<DocumentedItem> {
        match &item.kind {
            AstItemKind::Function(f) => Some(Self::extract_function(f)),
            AstItemKind::Struct(s) => Some(Self::extract_struct(s)),
            AstItemKind::Enum(e) => Some(Self::extract_enum(e)),
            AstItemKind::Interface(i) => Some(Self::extract_interface(i)),
            AstItemKind::Impl(i) => Some(Self::extract_impl(i)),
            AstItemKind::Import(_) => None,
        }
    }

    fn extract_function(func: &Function) -> DocumentedItem {
        let doc = func.trivia.doc_text().map(|t| DocComment::parse(&t));
        let sig = Self::format_function_signature(func);
        DocumentedItem::new(func.name.name.clone(), ItemKind::Function, sig).with_doc(doc)
    }

    fn extract_struct(s: &StructDef) -> DocumentedItem {
        let doc = s.trivia.doc_text().map(|t| DocComment::parse(&t));
        let sig = Self::format_struct_signature(s);
        let mut item = DocumentedItem::new(s.name.name.clone(), ItemKind::Struct, sig).with_doc(doc);

        // Add fields as children
        for field in &s.fields {
            let field_sig = format!("{}: {}", field.name.name, Self::format_type(&field.ty));
            let field_item = DocumentedItem::new(field.name.name.clone(), ItemKind::Field, field_sig);
            item.add_child(field_item);
        }

        item
    }

    fn extract_enum(e: &EnumDef) -> DocumentedItem {
        let doc = e.trivia.doc_text().map(|t| DocComment::parse(&t));
        let sig = Self::format_enum_signature(e);
        let mut item = DocumentedItem::new(e.name.name.clone(), ItemKind::Enum, sig).with_doc(doc);

        // Add variants as children
        for variant in &e.variants {
            let variant_sig = Self::format_variant_signature(variant);
            let variant_item =
                DocumentedItem::new(variant.name.name.clone(), ItemKind::Variant, variant_sig);
            item.add_child(variant_item);
        }

        item
    }

    fn extract_interface(i: &InterfaceDef) -> DocumentedItem {
        let doc = i.trivia.doc_text().map(|t| DocComment::parse(&t));
        let sig = Self::format_interface_signature(i);
        let mut item =
            DocumentedItem::new(i.name.name.clone(), ItemKind::Interface, sig).with_doc(doc);

        // Add methods as children
        for method in &i.methods {
            let method_sig = Self::format_interface_method_signature(method);
            let method_item =
                DocumentedItem::new(method.name.name.clone(), ItemKind::Method, method_sig);
            item.add_child(method_item);
        }

        item
    }

    fn extract_impl(i: &ImplDef) -> DocumentedItem {
        let doc = i.trivia.doc_text().map(|t| DocComment::parse(&t));
        let sig = Self::format_impl_signature(i);
        let name = Self::format_impl_name(i);
        let mut item = DocumentedItem::new(name, ItemKind::Impl, sig).with_doc(doc);

        // Add methods as children
        for method in &i.methods {
            let method_item = Self::extract_function(method);
            item.add_child(method_item);
        }

        item
    }

    // Formatting helpers

    fn format_function_signature(func: &Function) -> String {
        let mut sig = String::new();

        if func.is_async {
            sig.push_str("async ");
        }
        sig.push_str("fx ");
        sig.push_str(&func.name.name);

        // Type parameters
        if !func.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = func.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        // Parameters
        sig.push('(');
        let params: Vec<_> = func
            .params
            .iter()
            .map(|p| {
                let ty = p
                    .ty
                    .as_ref()
                    .map(|t| format!(": {}", Self::format_type(t)))
                    .unwrap_or_default();
                format!("{}{}", p.name.name, ty)
            })
            .collect();
        sig.push_str(&params.join(", "));
        sig.push(')');

        // Return type
        if let Some(ret) = &func.return_type {
            sig.push_str(" -> ");
            sig.push_str(&Self::format_type(ret));
        }

        sig
    }

    fn format_struct_signature(s: &StructDef) -> String {
        let mut sig = String::from("struct ");
        sig.push_str(&s.name.name);

        if !s.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = s.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        sig
    }

    fn format_enum_signature(e: &EnumDef) -> String {
        let mut sig = String::from("enum ");
        sig.push_str(&e.name.name);

        if !e.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = e.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        sig
    }

    fn format_variant_signature(variant: &crate::ast::EnumVariant) -> String {
        let mut sig = variant.name.name.clone();

        if let Some(data) = &variant.data {
            match data {
                crate::ast::EnumVariantData::Tuple(types) => {
                    sig.push('(');
                    let types: Vec<_> = types.iter().map(Self::format_type).collect();
                    sig.push_str(&types.join(", "));
                    sig.push(')');
                }
                crate::ast::EnumVariantData::Struct(fields) => {
                    sig.push_str(" { ");
                    let fields: Vec<_> = fields
                        .iter()
                        .map(|f| format!("{}: {}", f.name.name, Self::format_type(&f.ty)))
                        .collect();
                    sig.push_str(&fields.join(", "));
                    sig.push_str(" }");
                }
            }
        }

        sig
    }

    fn format_interface_signature(i: &InterfaceDef) -> String {
        let mut sig = String::from("interface ");
        sig.push_str(&i.name.name);

        if !i.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = i.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        sig
    }

    fn format_interface_method_signature(method: &crate::ast::InterfaceMethod) -> String {
        let mut sig = String::new();

        if method.is_async {
            sig.push_str("async ");
        }
        sig.push_str("fx ");
        sig.push_str(&method.name.name);

        // Type parameters
        if !method.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = method.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        // Parameters
        sig.push('(');
        let params: Vec<_> = method
            .params
            .iter()
            .map(|p| {
                let ty = p
                    .ty
                    .as_ref()
                    .map(|t| format!(": {}", Self::format_type(t)))
                    .unwrap_or_default();
                format!("{}{}", p.name.name, ty)
            })
            .collect();
        sig.push_str(&params.join(", "));
        sig.push(')');

        // Return type
        if let Some(ret) = &method.return_type {
            sig.push_str(" -> ");
            sig.push_str(&Self::format_type(ret));
        }

        sig
    }

    fn format_impl_signature(i: &ImplDef) -> String {
        let mut sig = String::from("impl");

        if !i.type_params.is_empty() {
            sig.push('<');
            let params: Vec<_> = i.type_params.iter().map(|p| p.name.name.as_str()).collect();
            sig.push_str(&params.join(", "));
            sig.push('>');
        }

        if let Some(interface) = &i.interface {
            sig.push(' ');
            sig.push_str(&Self::format_type(interface));
            sig.push_str(" for ");
        } else {
            sig.push(' ');
        }

        sig.push_str(&Self::format_type(&i.target));

        sig
    }

    fn format_impl_name(i: &ImplDef) -> String {
        if let Some(interface) = &i.interface {
            format!(
                "{} for {}",
                Self::format_type(interface),
                Self::format_type(&i.target)
            )
        } else {
            Self::format_type(&i.target)
        }
    }

    fn format_let_signature(let_decl: &crate::ast::TopLevelLet) -> String {
        let name = Self::pattern_to_name(&let_decl.pattern);
        let mut sig = format!("let {}", name);

        if let Some(ty) = &let_decl.ty {
            sig.push_str(": ");
            sig.push_str(&Self::format_type(ty));
        }

        sig
    }

    fn pattern_to_name(pattern: &crate::ast::Pattern) -> String {
        match &pattern.kind {
            crate::ast::PatternKind::Ident(ident) => ident.name.clone(),
            _ => "<pattern>".to_string(),
        }
    }

    fn format_type(ty: &TypeAnnotation) -> String {
        use crate::ast::TypeKind;

        match &ty.kind {
            TypeKind::Named { name, args } => {
                if args.is_empty() {
                    name.name.clone()
                } else {
                    let args: Vec<_> = args.iter().map(Self::format_type).collect();
                    format!("{}<{}>", name.name, args.join(", "))
                }
            }
            TypeKind::Nullable(inner) => format!("{}?", Self::format_type(inner)),
            TypeKind::Function { params, ret } => {
                let params: Vec<_> = params.iter().map(Self::format_type).collect();
                format!("({}) -> {}", params.join(", "), Self::format_type(ret))
            }
            TypeKind::Tuple(types) => {
                let types: Vec<_> = types.iter().map(Self::format_type).collect();
                format!("({})", types.join(", "))
            }
            TypeKind::List(inner) => format!("[{}]", Self::format_type(inner)),
            TypeKind::Unit => "()".to_string(),
            TypeKind::Never => "!".to_string(),
            TypeKind::Inferred => "_".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_extract_documented_function() {
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
        let doc_module = DocExtractor::extract(&module, "test");

        assert_eq!(doc_module.items.len(), 1);
        let item = &doc_module.items[0];
        assert_eq!(item.name, "greet");
        assert_eq!(item.kind, ItemKind::Function);

        let doc = item.doc.as_ref().expect("Expected doc comment");
        assert!(doc.summary.contains("Greet a user"));
        assert!(doc.params.contains_key("name"));
        assert!(doc.returns.is_some());
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
/// A point in 2D space.
struct Point {
    x: Int,
    y: Int,
}
"#;

        let module = Parser::parse_module(source).unwrap();
        let doc_module = DocExtractor::extract(&module, "test");

        assert_eq!(doc_module.items.len(), 1);
        let item = &doc_module.items[0];
        assert_eq!(item.name, "Point");
        assert_eq!(item.kind, ItemKind::Struct);
        assert_eq!(item.children.len(), 2);
    }
}
