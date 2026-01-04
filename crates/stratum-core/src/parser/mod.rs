//! Parser for the Stratum programming language
//!
//! This module implements a Pratt parser (top-down operator precedence) that converts
//! a token stream into an Abstract Syntax Tree (AST).
//!
//! # Example
//!
//! ```
//! use stratum_core::parser::Parser;
//!
//! // Parse a complete module with a function
//! let source = "fx add(a: Int, b: Int) -> Int { a + b }";
//! let result = Parser::parse_module(source);
//! assert!(result.is_ok());
//!
//! // Parse a single expression
//! let expr_result = Parser::parse_expression("1 + 2 * 3");
//! assert!(expr_result.is_ok());
//! ```

mod error;

pub use error::{ExpectedToken, ParseError, ParseErrorKind};

use crate::ast::{
    Attribute, AttributeArg, BinOp, Block, CatchClause, CompoundOp, ElseBranch, EnumDef,
    EnumVariant, EnumVariantData, Expr, ExprKind, FieldInit, FieldPattern, Function, Ident,
    ImplDef, Import, ImportItem, ImportKind, InterfaceDef, InterfaceMethod, Item, ItemKind,
    Literal, MatchArm, Module, Param, Pattern, PatternKind, Stmt, StmtKind, StringPart, StructDef,
    StructField, TypeAnnotation, TypeKind, TypeParam, UnaryOp,
};
use crate::lexer::{Lexer, Span, SpannedError, Token, TokenKind};

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Either a statement or an expression (for block parsing)
enum StmtOrExpr {
    Stmt(Stmt),
    Expr(Expr),
}

/// The Stratum parser
pub struct Parser {
    /// All tokens from the source
    tokens: Vec<Token>,
    /// Current position in the token stream
    position: usize,
    /// Collected parse errors
    errors: Vec<ParseError>,
    /// Lexer errors (passed through)
    lex_errors: Vec<SpannedError>,
    /// Nesting depth for loops (for break/continue validation)
    loop_depth: u32,
    /// Nesting depth for functions (for return validation)
    function_depth: u32,
}

impl Parser {
    /// Create a new parser from source code
    #[must_use]
    pub fn new(source: &str) -> Self {
        let (tokens, lex_errors) = Lexer::tokenize(source);
        Self {
            tokens,
            position: 0,
            errors: Vec::new(),
            lex_errors,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    /// Parse an entire module (source file)
    pub fn parse_module(source: &str) -> Result<Module, Vec<ParseError>> {
        let mut parser = Parser::new(source);
        let module = parser.module();
        if parser.errors.is_empty() {
            Ok(module)
        } else {
            Err(parser.errors)
        }
    }

    /// Parse a single expression (useful for REPL)
    pub fn parse_expression(source: &str) -> Result<Expr, Vec<ParseError>> {
        let mut parser = Parser::new(source);
        match parser.expression() {
            Ok(expr) => {
                if parser.errors.is_empty() {
                    Ok(expr)
                } else {
                    Err(parser.errors)
                }
            }
            Err(e) => {
                parser.errors.push(e);
                Err(parser.errors)
            }
        }
    }

    /// Get all errors (both lex and parse errors)
    #[must_use]
    pub fn all_errors(&self) -> Vec<ParseError> {
        let mut errors = self.errors.clone();
        for lex_err in &self.lex_errors {
            errors.push(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    found: TokenKind::Error,
                    expected: ExpectedToken::Description(lex_err.error.to_string()),
                },
                lex_err.span,
            ));
        }
        errors
    }

    // ==================== Token Management ====================

    /// Get the current token
    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream should have at least EOF")
        })
    }

    /// Get the current token kind
    fn current_kind(&self) -> TokenKind {
        self.current().kind.clone()
    }

    /// Check if we're at end of file
    fn is_eof(&self) -> bool {
        self.current_kind() == TokenKind::Eof
    }

    /// Advance to the next token, skipping trivia
    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        self.position += 1;
        self.skip_trivia();
        token
    }

    /// Skip trivia tokens (comments, newlines)
    fn skip_trivia(&mut self) {
        while self.position < self.tokens.len() && self.current().kind.is_trivia() {
            self.position += 1;
        }
    }

    /// Check if the current token matches a kind
    fn check(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    /// Check if the current token matches any of the given kinds
    fn check_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.iter().any(|k| self.check(k.clone()))
    }

    /// Consume a token if it matches, returning it
    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    /// Expect and consume a specific token, or error
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(kind.clone()) {
            Ok(self.advance())
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    found: self.current_kind(),
                    expected: ExpectedToken::Token(kind),
                },
                self.current().span,
            ))
        }
    }

    /// Expect an identifier token
    fn expect_ident(&mut self) -> ParseResult<Ident> {
        let token = self.current().clone();
        if matches!(token.kind, TokenKind::Ident | TokenKind::UnicodeIdent) {
            self.advance();
            Ok(Ident::new(token.lexeme, token.span))
        } else {
            Err(ParseError::new(
                ParseErrorKind::ExpectedIdentifier,
                token.span,
            ))
        }
    }

    /// Peek at the next non-trivia token
    fn peek(&self) -> Option<&Token> {
        let mut pos = self.position + 1;
        while pos < self.tokens.len() {
            let token = &self.tokens[pos];
            if !token.kind.is_trivia() {
                return Some(token);
            }
            pos += 1;
        }
        None
    }

    /// Record an error but continue parsing
    fn error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    // ==================== Module Parsing ====================

    /// Parse a complete module
    fn module(&mut self) -> Module {
        self.skip_trivia();
        let start = self.current().span.start;

        let mut items = Vec::new();
        while !self.is_eof() {
            match self.item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.error(e);
                    self.synchronize();
                }
            }
        }

        let end = self.current().span.end;
        Module::new(items, Span::new(start, end))
    }

    // ==================== Item Parsing ====================

    /// Parse a top-level item
    fn item(&mut self) -> ParseResult<Item> {
        let start = self.current().span.start;

        // Parse any attributes before the item
        let attributes = self.attributes()?;

        let kind = match self.current_kind() {
            TokenKind::Fx | TokenKind::Async => self.function_item(attributes)?,
            TokenKind::Struct => {
                if !attributes.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: TokenKind::Hash,
                            expected: ExpectedToken::Description(
                                "attributes not supported on structs yet".to_string(),
                            ),
                        },
                        attributes[0].span,
                    ));
                }
                self.struct_item()?
            }
            TokenKind::Enum => {
                if !attributes.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: TokenKind::Hash,
                            expected: ExpectedToken::Description(
                                "attributes not supported on enums yet".to_string(),
                            ),
                        },
                        attributes[0].span,
                    ));
                }
                self.enum_item()?
            }
            TokenKind::Interface => {
                if !attributes.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: TokenKind::Hash,
                            expected: ExpectedToken::Description(
                                "attributes not supported on interfaces yet".to_string(),
                            ),
                        },
                        attributes[0].span,
                    ));
                }
                self.interface_item()?
            }
            TokenKind::Impl => {
                if !attributes.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: TokenKind::Hash,
                            expected: ExpectedToken::Description(
                                "attributes not supported on impl blocks yet".to_string(),
                            ),
                        },
                        attributes[0].span,
                    ));
                }
                self.impl_item()?
            }
            TokenKind::Import => {
                if !attributes.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: TokenKind::Hash,
                            expected: ExpectedToken::Description(
                                "attributes not supported on imports".to_string(),
                            ),
                        },
                        attributes[0].span,
                    ));
                }
                self.import_item()?
            }
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken {
                        found: self.current_kind(),
                        expected: ExpectedToken::Description("top-level item".to_string()),
                    },
                    self.current().span,
                ));
            }
        };

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Item::new(kind, Span::new(start, end)))
    }

    /// Parse a list of attributes: #[attr1] #[attr2(args)]
    fn attributes(&mut self) -> ParseResult<Vec<Attribute>> {
        let mut attrs = Vec::new();
        while self.check(TokenKind::Hash) {
            attrs.push(self.attribute()?);
        }
        Ok(attrs)
    }

    /// Parse a single attribute: #[name] or #[name(args)]
    fn attribute(&mut self) -> ParseResult<Attribute> {
        let start = self.current().span.start;

        self.expect(TokenKind::Hash)?;
        self.expect(TokenKind::LBracket)?;

        let name = self.expect_ident()?;

        // Optional arguments
        let args = if self.eat(TokenKind::LParen).is_some() {
            let args = self.attribute_args()?;
            self.expect(TokenKind::RParen)?;
            args
        } else {
            Vec::new()
        };

        let end_token = self.expect(TokenKind::RBracket)?;
        let end = end_token.span.end;

        Ok(Attribute::new(name, args, Span::new(start, end)))
    }

    /// Parse attribute arguments: ident, ident = expr, ...
    fn attribute_args(&mut self) -> ParseResult<Vec<AttributeArg>> {
        let mut args = Vec::new();

        while !self.check(TokenKind::RParen) && !self.is_eof() {
            let name = self.expect_ident()?;

            let arg = if self.eat(TokenKind::Eq).is_some() {
                // Name = value form
                let value = self.expression()?;
                AttributeArg::NameValue {
                    name,
                    value: Box::new(value),
                }
            } else {
                // Just an identifier
                AttributeArg::Ident(name)
            };

            args.push(arg);

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        Ok(args)
    }

    /// Parse a function definition
    fn function_item(&mut self, attributes: Vec<Attribute>) -> ParseResult<ItemKind> {
        let func = self.function(attributes)?;
        Ok(ItemKind::Function(func))
    }

    /// Parse a function
    fn function(&mut self, attributes: Vec<Attribute>) -> ParseResult<Function> {
        let start = self.current().span.start;

        // Check for async modifier
        let is_async = self.eat(TokenKind::Async).is_some();

        // Expect 'fx' keyword
        self.expect(TokenKind::Fx)?;

        // Function name
        let name = self.expect_ident()?;

        // Optional type parameters
        let type_params = if self.check(TokenKind::Lt) {
            self.type_params()?
        } else {
            Vec::new()
        };

        // Parameters
        self.expect(TokenKind::LParen)?;
        let params = self.param_list()?;
        self.expect(TokenKind::RParen)?;

        // Optional return type
        let return_type = if self.eat(TokenKind::Arrow).is_some() {
            Some(self.type_annotation()?)
        } else {
            None
        };

        // Function body
        self.function_depth += 1;
        let body = self.block()?;
        self.function_depth -= 1;

        let end = body.span.end;

        Ok(Function::new(
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            attributes,
            Span::new(start, end),
        ))
    }

    /// Parse a parameter list
    fn param_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        while !self.check(TokenKind::RParen) && !self.is_eof() {
            let param = self.param()?;
            params.push(param);

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        Ok(params)
    }

    /// Parse a single parameter
    fn param(&mut self) -> ParseResult<Param> {
        let start = self.current().span.start;
        let name = self.expect_ident()?;

        // Optional type annotation
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.type_annotation()?)
        } else {
            None
        };

        // Optional default value
        let default = if self.eat(TokenKind::Eq).is_some() {
            Some(self.expression()?)
        } else {
            None
        };

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Param::new(name, ty, default, Span::new(start, end)))
    }

    /// Parse a struct definition
    fn struct_item(&mut self) -> ParseResult<ItemKind> {
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;

        // Optional type parameters
        let type_params = if self.check(TokenKind::Lt) {
            self.type_params()?
        } else {
            Vec::new()
        };

        // Fields
        self.expect(TokenKind::LBrace)?;
        let fields = self.struct_fields()?;
        self.expect(TokenKind::RBrace)?;

        let span = Span::new(
            name.span.start,
            self.tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(name.span.start),
        );

        Ok(ItemKind::Struct(StructDef::new(
            name,
            type_params,
            fields,
            span,
        )))
    }

    /// Parse struct fields
    fn struct_fields(&mut self) -> ParseResult<Vec<StructField>> {
        let mut fields = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let start = self.current().span.start;
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.type_annotation()?;
            let end = ty.span.end;

            fields.push(StructField::new(name, ty, true, Span::new(start, end)));

            // Optional comma
            self.eat(TokenKind::Comma);
        }

        Ok(fields)
    }

    /// Parse an enum definition
    fn enum_item(&mut self) -> ParseResult<ItemKind> {
        let start = self.current().span.start;
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;

        // Optional type parameters
        let type_params = if self.check(TokenKind::Lt) {
            self.type_params()?
        } else {
            Vec::new()
        };

        // Variants
        self.expect(TokenKind::LBrace)?;
        let variants = self.enum_variants()?;
        self.expect(TokenKind::RBrace)?;

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(ItemKind::Enum(EnumDef::new(
            name,
            type_params,
            variants,
            Span::new(start, end),
        )))
    }

    /// Parse enum variants
    fn enum_variants(&mut self) -> ParseResult<Vec<EnumVariant>> {
        let mut variants = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let start = self.current().span.start;
            let name = self.expect_ident()?;

            // Optional data
            let data = if self.check(TokenKind::LParen) {
                self.expect(TokenKind::LParen)?;
                let mut types = Vec::new();
                while !self.check(TokenKind::RParen) && !self.is_eof() {
                    types.push(self.type_annotation()?);
                    if !self.eat(TokenKind::Comma).is_some() {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
                Some(EnumVariantData::Tuple(types))
            } else if self.check(TokenKind::LBrace) {
                self.expect(TokenKind::LBrace)?;
                let fields = self.struct_fields()?;
                self.expect(TokenKind::RBrace)?;
                Some(EnumVariantData::Struct(fields))
            } else {
                None
            };

            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            variants.push(EnumVariant::new(name, data, Span::new(start, end)));

            // Optional comma
            self.eat(TokenKind::Comma);
        }

        Ok(variants)
    }

    /// Parse an interface definition
    fn interface_item(&mut self) -> ParseResult<ItemKind> {
        let start = self.current().span.start;
        self.expect(TokenKind::Interface)?;
        let name = self.expect_ident()?;

        // Optional type parameters
        let type_params = if self.check(TokenKind::Lt) {
            self.type_params()?
        } else {
            Vec::new()
        };

        // Methods
        self.expect(TokenKind::LBrace)?;
        let methods = self.interface_methods()?;
        self.expect(TokenKind::RBrace)?;

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(ItemKind::Interface(InterfaceDef::new(
            name,
            type_params,
            methods,
            Span::new(start, end),
        )))
    }

    /// Parse interface methods
    fn interface_methods(&mut self) -> ParseResult<Vec<InterfaceMethod>> {
        let mut methods = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let start = self.current().span.start;

            // Check for async modifier
            let is_async = self.eat(TokenKind::Async).is_some();

            self.expect(TokenKind::Fx)?;
            let name = self.expect_ident()?;

            // Optional type parameters
            let type_params = if self.check(TokenKind::Lt) {
                self.type_params()?
            } else {
                Vec::new()
            };

            // Parameters
            self.expect(TokenKind::LParen)?;
            let params = self.param_list()?;
            self.expect(TokenKind::RParen)?;

            // Optional return type
            let return_type = if self.eat(TokenKind::Arrow).is_some() {
                Some(self.type_annotation()?)
            } else {
                None
            };

            // Optional default body
            let default_body = if self.check(TokenKind::LBrace) {
                Some(self.block()?)
            } else {
                None
            };

            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            methods.push(InterfaceMethod::new(
                name,
                type_params,
                params,
                return_type,
                is_async,
                default_body,
                Span::new(start, end),
            ));
        }

        Ok(methods)
    }

    /// Parse an impl block
    fn impl_item(&mut self) -> ParseResult<ItemKind> {
        let start = self.current().span.start;
        self.expect(TokenKind::Impl)?;

        // Optional type parameters
        let type_params = if self.check(TokenKind::Lt) {
            self.type_params()?
        } else {
            Vec::new()
        };

        // First type (could be interface or target)
        let first_type = self.type_annotation()?;

        // Check for 'for' to determine if it's interface impl
        let (interface, target) = if self.eat(TokenKind::For).is_some() {
            let target = self.type_annotation()?;
            (Some(first_type), target)
        } else {
            (None, first_type)
        };

        // Methods
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        self.function_depth += 1;
        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let attrs = self.attributes()?;
            methods.push(self.function(attrs)?);
        }
        self.function_depth -= 1;
        self.expect(TokenKind::RBrace)?;

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(ItemKind::Impl(ImplDef::new(
            type_params,
            interface,
            target,
            methods,
            Span::new(start, end),
        )))
    }

    /// Parse an import statement
    fn import_item(&mut self) -> ParseResult<ItemKind> {
        let start = self.current().span.start;
        self.expect(TokenKind::Import)?;

        // Parse path segments
        let mut path = vec![self.expect_ident()?];
        while self.eat(TokenKind::Dot).is_some() {
            if self.check(TokenKind::Star) {
                self.advance();
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                return Ok(ItemKind::Import(Import::new(
                    path,
                    ImportKind::Glob,
                    Span::new(start, end),
                )));
            } else if self.check(TokenKind::LBrace) {
                // Import list
                self.expect(TokenKind::LBrace)?;
                let mut items = Vec::new();
                while !self.check(TokenKind::RBrace) && !self.is_eof() {
                    let item_start = self.current().span.start;
                    let name = self.expect_ident()?;
                    let alias = if self.eat(TokenKind::Ident).is_some()
                        && self
                            .tokens
                            .get(self.position.saturating_sub(1))
                            .map(|t| t.lexeme.as_str())
                            == Some("as")
                    {
                        Some(self.expect_ident()?)
                    } else {
                        None
                    };
                    let item_end = self
                        .tokens
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(item_start);
                    items.push(ImportItem::new(
                        name,
                        alias,
                        Span::new(item_start, item_end),
                    ));
                    if !self.eat(TokenKind::Comma).is_some() {
                        break;
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                return Ok(ItemKind::Import(Import::new(
                    path,
                    ImportKind::List(items),
                    Span::new(start, end),
                )));
            } else {
                path.push(self.expect_ident()?);
            }
        }

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(ItemKind::Import(Import::new(
            path,
            ImportKind::Item,
            Span::new(start, end),
        )))
    }

    /// Parse type parameters (<T, U: Bound>)
    fn type_params(&mut self) -> ParseResult<Vec<TypeParam>> {
        self.expect(TokenKind::Lt)?;
        let mut params = Vec::new();

        while !self.check(TokenKind::Gt) && !self.is_eof() {
            let start = self.current().span.start;
            let name = self.expect_ident()?;

            // Optional bounds
            let bounds = if self.eat(TokenKind::Colon).is_some() {
                let mut bounds = vec![self.expect_ident()?];
                while self.eat(TokenKind::Plus).is_some() {
                    bounds.push(self.expect_ident()?);
                }
                bounds
            } else {
                Vec::new()
            };

            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            params.push(TypeParam::with_bounds(name, bounds, Span::new(start, end)));

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::Gt)?;
        Ok(params)
    }

    // ==================== Type Parsing ====================

    /// Parse a type annotation
    fn type_annotation(&mut self) -> ParseResult<TypeAnnotation> {
        let start = self.current().span.start;
        let mut ty = self.primary_type()?;

        // Check for nullable suffix
        if self.eat(TokenKind::Question).is_some() {
            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            ty = TypeAnnotation::new(TypeKind::Nullable(Box::new(ty)), Span::new(start, end));
        }

        Ok(ty)
    }

    /// Parse a primary type (without nullable suffix)
    fn primary_type(&mut self) -> ParseResult<TypeAnnotation> {
        let start = self.current().span.start;

        match self.current_kind() {
            TokenKind::Ident | TokenKind::UnicodeIdent => {
                let name = self.expect_ident()?;

                // Check for generic arguments
                let args = if self.check(TokenKind::Lt) {
                    self.expect(TokenKind::Lt)?;
                    let mut args = Vec::new();
                    while !self.check(TokenKind::Gt) && !self.is_eof() {
                        args.push(self.type_annotation()?);
                        if !self.eat(TokenKind::Comma).is_some() {
                            break;
                        }
                    }
                    self.expect(TokenKind::Gt)?;
                    args
                } else {
                    Vec::new()
                };

                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);

                Ok(TypeAnnotation::new(
                    TypeKind::Named { name, args },
                    Span::new(start, end),
                ))
            }
            TokenKind::LParen => {
                // Tuple or function type or unit
                self.expect(TokenKind::LParen)?;

                if self.check(TokenKind::RParen) {
                    self.expect(TokenKind::RParen)?;
                    let end = self
                        .tokens
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start);
                    return Ok(TypeAnnotation::new(TypeKind::Unit, Span::new(start, end)));
                }

                let mut types = vec![self.type_annotation()?];
                while self.eat(TokenKind::Comma).is_some() {
                    types.push(self.type_annotation()?);
                }
                self.expect(TokenKind::RParen)?;

                // Check for function type
                if self.eat(TokenKind::Arrow).is_some() {
                    let ret = self.type_annotation()?;
                    let end = ret.span.end;
                    return Ok(TypeAnnotation::new(
                        TypeKind::Function {
                            params: types,
                            ret: Box::new(ret),
                        },
                        Span::new(start, end),
                    ));
                }

                // It's a tuple
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);

                if types.len() == 1 {
                    // Single element - just return the inner type
                    Ok(types.into_iter().next().unwrap())
                } else {
                    Ok(TypeAnnotation::new(
                        TypeKind::Tuple(types),
                        Span::new(start, end),
                    ))
                }
            }
            TokenKind::LBracket => {
                // List shorthand [T]
                self.expect(TokenKind::LBracket)?;
                let inner = self.type_annotation()?;
                self.expect(TokenKind::RBracket)?;
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                Ok(TypeAnnotation::new(
                    TypeKind::List(Box::new(inner)),
                    Span::new(start, end),
                ))
            }
            TokenKind::Not => {
                // Never type
                self.advance();
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                Ok(TypeAnnotation::new(TypeKind::Never, Span::new(start, end)))
            }
            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedType,
                self.current().span,
            )),
        }
    }

    // ==================== Statement Parsing ====================

    /// Parse a block ({ stmts; expr? })
    fn block(&mut self) -> ParseResult<Block> {
        let start = self.current().span.start;
        self.expect(TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            match self.statement_or_expr() {
                Ok(StmtOrExpr::Stmt(stmt)) => stmts.push(stmt),
                Ok(StmtOrExpr::Expr(expr)) => {
                    // This could be a trailing expression or need semicolon
                    if self.check(TokenKind::RBrace) {
                        trailing_expr = Some(expr);
                    } else if self.eat(TokenKind::Semicolon).is_some() {
                        stmts.push(Stmt::new(
                            StmtKind::Expr(expr),
                            Span::new(
                                start,
                                self.tokens
                                    .get(self.position.saturating_sub(1))
                                    .map(|t| t.span.end)
                                    .unwrap_or(start),
                            ),
                        ));
                    } else {
                        // Error: expected semicolon
                        let span = expr.span;
                        stmts.push(Stmt::new(StmtKind::Expr(expr), span));
                        self.error(ParseError::new(
                            ParseErrorKind::ExpectedAfter {
                                expected: ";",
                                context: "expression",
                            },
                            self.current().span,
                        ));
                    }
                }
                Err(e) => {
                    self.error(e);
                    self.synchronize_in_block();
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Block::new(stmts, trailing_expr, Span::new(start, end)))
    }

    /// Parse a statement or expression
    fn statement_or_expr(&mut self) -> ParseResult<StmtOrExpr> {
        match self.current_kind() {
            TokenKind::Let => {
                let stmt = self.let_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::Return => {
                let stmt = self.return_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::For => {
                let stmt = self.for_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::While => {
                let stmt = self.while_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::Break => {
                let stmt = self.break_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::Continue => {
                let stmt = self.continue_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            TokenKind::Try => {
                let stmt = self.try_stmt()?;
                Ok(StmtOrExpr::Stmt(stmt))
            }
            _ => {
                let expr = self.expression()?;
                // Check for assignment
                if self.check(TokenKind::Eq) {
                    let stmt = self.assignment(expr)?;
                    Ok(StmtOrExpr::Stmt(stmt))
                } else if self.check_any(&[
                    TokenKind::Plus,
                    TokenKind::Minus,
                    TokenKind::Star,
                    TokenKind::Slash,
                    TokenKind::Percent,
                ]) {
                    // Look ahead for compound assignment
                    if let Some(next) = self.peek() {
                        if next.kind == TokenKind::Eq {
                            let stmt = self.compound_assignment(expr)?;
                            return Ok(StmtOrExpr::Stmt(stmt));
                        }
                    }
                    Ok(StmtOrExpr::Expr(expr))
                } else {
                    Ok(StmtOrExpr::Expr(expr))
                }
            }
        }
    }

    /// Parse a let statement
    fn let_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.current().span.start;
        self.expect(TokenKind::Let)?;

        let pattern = self.pattern()?;

        // Optional type annotation
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.type_annotation()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.expression()?;

        let end = value.span.end;
        self.eat(TokenKind::Semicolon);

        Ok(Stmt::new(
            StmtKind::Let { pattern, ty, value },
            Span::new(start, end),
        ))
    }

    /// Parse a return statement
    fn return_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.current().span.start;
        self.expect(TokenKind::Return)?;

        if self.function_depth == 0 {
            return Err(ParseError::new(
                ParseErrorKind::ReturnOutsideFunction,
                Span::new(start, self.current().span.end),
            ));
        }

        let value = if !self.check(TokenKind::Semicolon)
            && !self.check(TokenKind::RBrace)
            && !self.is_eof()
        {
            Some(self.expression()?)
        } else {
            None
        };

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        self.eat(TokenKind::Semicolon);

        Ok(Stmt::new(StmtKind::Return(value), Span::new(start, end)))
    }

    /// Parse a for loop
    fn for_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.current().span.start;
        self.expect(TokenKind::For)?;

        let pattern = self.pattern()?;
        self.expect(TokenKind::In)?;
        let iter = self.expression()?;

        self.loop_depth += 1;
        let body = self.block()?;
        self.loop_depth -= 1;

        let end = body.span.end;

        Ok(Stmt::new(
            StmtKind::For {
                pattern,
                iter,
                body,
            },
            Span::new(start, end),
        ))
    }

    /// Parse a while loop
    fn while_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.current().span.start;
        self.expect(TokenKind::While)?;

        let cond = self.expression()?;

        self.loop_depth += 1;
        let body = self.block()?;
        self.loop_depth -= 1;

        let end = body.span.end;

        Ok(Stmt::new(
            StmtKind::While { cond, body },
            Span::new(start, end),
        ))
    }

    /// Parse a break statement
    fn break_stmt(&mut self) -> ParseResult<Stmt> {
        let token = self.expect(TokenKind::Break)?;

        if self.loop_depth == 0 {
            return Err(ParseError::new(
                ParseErrorKind::BreakOutsideLoop,
                token.span,
            ));
        }

        self.eat(TokenKind::Semicolon);
        Ok(Stmt::new(StmtKind::Break, token.span))
    }

    /// Parse a continue statement
    fn continue_stmt(&mut self) -> ParseResult<Stmt> {
        let token = self.expect(TokenKind::Continue)?;

        if self.loop_depth == 0 {
            return Err(ParseError::new(
                ParseErrorKind::ContinueOutsideLoop,
                token.span,
            ));
        }

        self.eat(TokenKind::Semicolon);
        Ok(Stmt::new(StmtKind::Continue, token.span))
    }

    /// Parse a try-catch statement
    fn try_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.current().span.start;
        self.expect(TokenKind::Try)?;

        let try_block = self.block()?;

        // Parse catch clauses
        let mut catches = Vec::new();
        while self.check(TokenKind::Catch) {
            let catch_start = self.current().span.start;
            self.expect(TokenKind::Catch)?;

            // Optional exception type and binding
            let (exception_type, binding) = if !self.check(TokenKind::LBrace) {
                let ty = Some(self.type_annotation()?);
                let binding = if self.check(TokenKind::Ident) || self.check(TokenKind::UnicodeIdent)
                {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                (ty, binding)
            } else {
                (None, None)
            };

            let body = self.block()?;
            let catch_end = body.span.end;

            catches.push(CatchClause::new(
                exception_type,
                binding,
                body,
                Span::new(catch_start, catch_end),
            ));
        }

        // Optional finally
        let finally = if self.eat(TokenKind::Ident).is_some()
            && self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.lexeme.as_str())
                == Some("finally")
        {
            Some(self.block()?)
        } else {
            None
        };

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Stmt::new(
            StmtKind::TryCatch {
                try_block,
                catches,
                finally,
            },
            Span::new(start, end),
        ))
    }

    /// Parse an assignment statement
    fn assignment(&mut self, target: Expr) -> ParseResult<Stmt> {
        let start = target.span.start;
        self.expect(TokenKind::Eq)?;
        let value = self.expression()?;
        let end = value.span.end;
        self.eat(TokenKind::Semicolon);

        Ok(Stmt::new(
            StmtKind::Assign { target, value },
            Span::new(start, end),
        ))
    }

    /// Parse a compound assignment statement
    fn compound_assignment(&mut self, target: Expr) -> ParseResult<Stmt> {
        let start = target.span.start;

        let op = match self.current_kind() {
            TokenKind::Plus => {
                self.advance();
                self.expect(TokenKind::Eq)?;
                CompoundOp::Add
            }
            TokenKind::Minus => {
                self.advance();
                self.expect(TokenKind::Eq)?;
                CompoundOp::Sub
            }
            TokenKind::Star => {
                self.advance();
                self.expect(TokenKind::Eq)?;
                CompoundOp::Mul
            }
            TokenKind::Slash => {
                self.advance();
                self.expect(TokenKind::Eq)?;
                CompoundOp::Div
            }
            TokenKind::Percent => {
                self.advance();
                self.expect(TokenKind::Eq)?;
                CompoundOp::Mod
            }
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken {
                        found: self.current_kind(),
                        expected: ExpectedToken::Description(
                            "compound assignment operator".to_string(),
                        ),
                    },
                    self.current().span,
                ))
            }
        };

        let value = self.expression()?;
        let end = value.span.end;
        self.eat(TokenKind::Semicolon);

        Ok(Stmt::new(
            StmtKind::CompoundAssign { target, op, value },
            Span::new(start, end),
        ))
    }

    // ==================== Pattern Parsing ====================

    /// Parse a pattern
    fn pattern(&mut self) -> ParseResult<Pattern> {
        let start = self.current().span.start;

        match self.current_kind() {
            TokenKind::Ident | TokenKind::UnicodeIdent => {
                let ident = self.expect_ident()?;

                // Check for qualified enum variant (Enum::Variant)
                if self.check(TokenKind::ColonColon) {
                    self.expect(TokenKind::ColonColon)?;
                    let variant = self.expect_ident()?;

                    // Check for variant data (Enum::Variant(data))
                    let data = if self.check(TokenKind::LParen) {
                        self.expect(TokenKind::LParen)?;
                        let inner = if self.check(TokenKind::RParen) {
                            None
                        } else {
                            Some(Box::new(self.pattern()?))
                        };
                        self.expect(TokenKind::RParen)?;
                        inner
                    } else {
                        None
                    };

                    let end = self
                        .tokens
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start);

                    return Ok(Pattern::new(
                        PatternKind::Variant {
                            enum_name: Some(ident),
                            variant,
                            data,
                        },
                        Span::new(start, end),
                    ));
                }

                // Check for struct pattern
                if self.check(TokenKind::LBrace) {
                    return self.struct_pattern(ident);
                }

                // Check for enum variant pattern (unqualified, e.g., Some(x))
                if self.check(TokenKind::LParen) {
                    return self.variant_pattern(None, ident);
                }

                Ok(Pattern::new(PatternKind::Ident(ident.clone()), ident.span))
            }
            TokenKind::Int
            | TokenKind::Float
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null => {
                let lit = self.literal()?;
                let span = lit.span;
                if let ExprKind::Literal(l) = lit.kind {
                    Ok(Pattern::new(PatternKind::Literal(l), span))
                } else {
                    Err(ParseError::new(ParseErrorKind::ExpectedPattern, span))
                }
            }
            TokenKind::StringStart => {
                // String literal pattern
                let lit = self.string_literal()?;
                let span = lit.span;
                if let ExprKind::Literal(l) = lit.kind {
                    Ok(Pattern::new(PatternKind::Literal(l), span))
                } else {
                    Err(ParseError::new(ParseErrorKind::ExpectedPattern, span))
                }
            }
            TokenKind::LBracket => {
                // List pattern
                self.expect(TokenKind::LBracket)?;
                let mut elements = Vec::new();
                let mut rest = None;

                while !self.check(TokenKind::RBracket) && !self.is_eof() {
                    if self.eat(TokenKind::DotDot).is_some() {
                        if !self.check(TokenKind::RBracket) && !self.check(TokenKind::Comma) {
                            rest = Some(Box::new(self.pattern()?));
                        }
                        break;
                    }
                    elements.push(self.pattern()?);
                    if !self.eat(TokenKind::Comma).is_some() {
                        break;
                    }
                }

                self.expect(TokenKind::RBracket)?;
                let end = self
                    .tokens
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);

                Ok(Pattern::new(
                    PatternKind::List { elements, rest },
                    Span::new(start, end),
                ))
            }
            _ if self.current().lexeme == "_" => {
                let token = self.advance();
                Ok(Pattern::new(PatternKind::Wildcard, token.span))
            }
            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedPattern,
                self.current().span,
            )),
        }
    }

    /// Parse a struct pattern
    fn struct_pattern(&mut self, name: Ident) -> ParseResult<Pattern> {
        let start = name.span.start;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let field_start = self.current().span.start;
            let field_name = self.expect_ident()?;

            let pattern = if self.eat(TokenKind::Colon).is_some() {
                Some(self.pattern()?)
            } else {
                None
            };

            let field_end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(field_start);

            fields.push(FieldPattern {
                name: field_name,
                pattern,
                span: Span::new(field_start, field_end),
            });

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Pattern::new(
            PatternKind::Struct { name, fields },
            Span::new(start, end),
        ))
    }

    /// Parse a variant pattern
    fn variant_pattern(
        &mut self,
        enum_name: Option<Ident>,
        variant: Ident,
    ) -> ParseResult<Pattern> {
        let start = enum_name
            .as_ref()
            .map(|n| n.span.start)
            .unwrap_or(variant.span.start);
        self.expect(TokenKind::LParen)?;

        let data = if !self.check(TokenKind::RParen) {
            Some(Box::new(self.pattern()?))
        } else {
            None
        };

        self.expect(TokenKind::RParen)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Pattern::new(
            PatternKind::Variant {
                enum_name,
                variant,
                data,
            },
            Span::new(start, end),
        ))
    }

    // ==================== Expression Parsing (Pratt Parser) ====================

    /// Parse an expression
    pub fn expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(0)
    }

    /// Parse expression with given minimum precedence
    fn parse_precedence(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut left = self.prefix_expr()?;

        while let Some((op, prec)) = self.infix_op() {
            if prec < min_prec {
                break;
            }

            // Handle right associativity
            let assoc_adjust = if op.is_left_associative() { 1 } else { 0 };

            self.advance(); // consume operator
            let right = self.parse_precedence(prec + assoc_adjust)?;

            let span = Span::new(left.span.start, right.span.end);
            left = Expr::new(
                ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    /// Get current infix operator and its precedence
    fn infix_op(&self) -> Option<(BinOp, u8)> {
        let op = match self.current_kind() {
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod,
            TokenKind::EqEq => BinOp::Eq,
            TokenKind::NotEq => BinOp::Ne,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::LtEq => BinOp::Le,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::GtEq => BinOp::Ge,
            TokenKind::And => BinOp::And,
            TokenKind::Or => BinOp::Or,
            TokenKind::PipeGt => BinOp::Pipe,
            TokenKind::DoubleQuestion => BinOp::NullCoalesce,
            TokenKind::DotDot => BinOp::Range,
            TokenKind::DotDotEq => BinOp::RangeInclusive,
            _ => return None,
        };
        Some((op, op.precedence()))
    }

    /// Parse a prefix expression (unary, primary, or postfix)
    fn prefix_expr(&mut self) -> ParseResult<Expr> {
        match self.current_kind() {
            TokenKind::Minus => {
                let op_token = self.advance();
                let expr = self.prefix_expr()?;
                let span = Span::new(op_token.span.start, expr.span.end);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            TokenKind::Not => {
                let op_token = self.advance();
                let expr = self.prefix_expr()?;
                let span = Span::new(op_token.span.start, expr.span.end);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            TokenKind::Await => {
                let op_token = self.advance();
                let expr = self.prefix_expr()?;
                let span = Span::new(op_token.span.start, expr.span.end);
                Ok(Expr::new(ExprKind::Await(Box::new(expr)), span))
            }
            _ => self.postfix_expr(),
        }
    }

    /// Parse postfix expressions (calls, field access, index)
    fn postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary_expr()?;

        loop {
            let start = expr.span.start;
            match self.current_kind() {
                TokenKind::LParen => {
                    // Function call
                    self.expect(TokenKind::LParen)?;
                    let args = self.arg_list()?;
                    self.expect(TokenKind::RParen)?;
                    let end = self
                        .tokens
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start);
                    expr = Expr::new(
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                        Span::new(start, end),
                    );
                }
                TokenKind::LBracket => {
                    // Index access
                    self.expect(TokenKind::LBracket)?;
                    let index = self.expression()?;
                    self.expect(TokenKind::RBracket)?;
                    let end = self
                        .tokens
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start);
                    expr = Expr::new(
                        ExprKind::Index {
                            expr: Box::new(expr),
                            index: Box::new(index),
                        },
                        Span::new(start, end),
                    );
                }
                TokenKind::Dot => {
                    // Field access
                    self.advance();
                    let field = self.expect_ident()?;
                    let end = field.span.end;
                    expr = Expr::new(
                        ExprKind::Field {
                            expr: Box::new(expr),
                            field,
                        },
                        Span::new(start, end),
                    );
                }
                TokenKind::QuestionDot => {
                    // Null-safe field access (?.field) or null-safe index access (?.[index])
                    self.advance();

                    if self.check(TokenKind::LBracket) {
                        // Null-safe index access: ?.[index]
                        self.expect(TokenKind::LBracket)?;
                        let index = self.expression()?;
                        self.expect(TokenKind::RBracket)?;
                        let end = self
                            .tokens
                            .get(self.position.saturating_sub(1))
                            .map(|t| t.span.end)
                            .unwrap_or(start);
                        expr = Expr::new(
                            ExprKind::NullSafeIndex {
                                expr: Box::new(expr),
                                index: Box::new(index),
                            },
                            Span::new(start, end),
                        );
                    } else {
                        // Null-safe field access: ?.field
                        let field = self.expect_ident()?;
                        let end = field.span.end;
                        expr = Expr::new(
                            ExprKind::NullSafeField {
                                expr: Box::new(expr),
                                field,
                            },
                            Span::new(start, end),
                        );
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse argument list
    fn arg_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();

        while !self.check(TokenKind::RParen) && !self.is_eof() {
            args.push(self.expression()?);
            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        Ok(args)
    }

    /// Parse a primary expression
    fn primary_expr(&mut self) -> ParseResult<Expr> {
        match self.current_kind() {
            // Literals
            TokenKind::Int | TokenKind::HexInt | TokenKind::BinaryInt | TokenKind::OctalInt => {
                self.integer_literal()
            }
            TokenKind::Float => self.float_literal(),
            TokenKind::True | TokenKind::False => self.bool_literal(),
            TokenKind::Null => self.null_literal(),
            TokenKind::StringStart => self.string_literal(),

            // Identifiers and struct init
            TokenKind::Ident | TokenKind::UnicodeIdent => self.ident_or_struct_init(),

            // Parenthesized or lambda
            TokenKind::LParen => self.paren_or_lambda(),

            // Block expression
            TokenKind::LBrace => self.block_or_map(),

            // List literal
            TokenKind::LBracket => self.list_literal(),

            // If expression
            TokenKind::If => self.if_expr(),

            // Match expression
            TokenKind::Match => self.match_expr(),

            // Lambda with pipe syntax
            TokenKind::Pipe => self.lambda_expr(),

            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedExpression,
                self.current().span,
            )),
        }
    }

    /// Parse an integer literal
    fn integer_literal(&mut self) -> ParseResult<Expr> {
        let token = self.advance();
        let value = parse_int(&token.lexeme, &token.kind)
            .map_err(|e| ParseError::new(ParseErrorKind::InvalidNumber(e), token.span))?;
        Ok(Expr::new(
            ExprKind::Literal(Literal::Int(value)),
            token.span,
        ))
    }

    /// Parse a float literal
    fn float_literal(&mut self) -> ParseResult<Expr> {
        let token = self.advance();
        let clean = token.lexeme.replace('_', "");
        let value: f64 = clean.parse().map_err(|_| {
            ParseError::new(
                ParseErrorKind::InvalidNumber(format!("invalid float: {}", token.lexeme)),
                token.span,
            )
        })?;
        Ok(Expr::new(
            ExprKind::Literal(Literal::Float(value)),
            token.span,
        ))
    }

    /// Parse a boolean literal
    fn bool_literal(&mut self) -> ParseResult<Expr> {
        let token = self.advance();
        let value = token.kind == TokenKind::True;
        Ok(Expr::new(
            ExprKind::Literal(Literal::Bool(value)),
            token.span,
        ))
    }

    /// Parse a null literal
    fn null_literal(&mut self) -> ParseResult<Expr> {
        let token = self.advance();
        Ok(Expr::new(ExprKind::Literal(Literal::Null), token.span))
    }

    /// Parse a literal (for patterns)
    fn literal(&mut self) -> ParseResult<Expr> {
        match self.current_kind() {
            TokenKind::Int | TokenKind::HexInt | TokenKind::BinaryInt | TokenKind::OctalInt => {
                self.integer_literal()
            }
            TokenKind::Float => self.float_literal(),
            TokenKind::True | TokenKind::False => self.bool_literal(),
            TokenKind::Null => self.null_literal(),
            TokenKind::StringStart => self.string_literal(),
            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedExpression,
                self.current().span,
            )),
        }
    }

    /// Parse a string literal (with interpolation support)
    fn string_literal(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::StringStart)?;

        let mut parts = Vec::new();

        loop {
            match self.current_kind() {
                TokenKind::StringPart => {
                    let token = self.advance();
                    parts.push(StringPart::Literal(token.lexeme));
                }
                TokenKind::InterpolationStart => {
                    self.advance();
                    let expr = self.expression()?;
                    parts.push(StringPart::Expr(expr));
                    self.expect(TokenKind::InterpolationEnd)?;
                }
                TokenKind::StringEnd => {
                    let end_token = self.advance();
                    let end = end_token.span.end;

                    // Simple string (no interpolation)
                    if parts.len() == 1 {
                        if let StringPart::Literal(s) = &parts[0] {
                            return Ok(Expr::new(
                                ExprKind::Literal(Literal::String(s.clone())),
                                Span::new(start, end),
                            ));
                        }
                    }

                    // Empty string
                    if parts.is_empty() {
                        return Ok(Expr::new(
                            ExprKind::Literal(Literal::String(String::new())),
                            Span::new(start, end),
                        ));
                    }

                    return Ok(Expr::new(
                        ExprKind::StringInterp { parts },
                        Span::new(start, end),
                    ));
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedEof,
                        self.current().span,
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            found: self.current_kind(),
                            expected: ExpectedToken::Description("string content".to_string()),
                        },
                        self.current().span,
                    ));
                }
            }
        }
    }

    /// Parse identifier, struct init, or enum variant
    fn ident_or_struct_init(&mut self) -> ParseResult<Expr> {
        let ident = self.expect_ident()?;

        // Check for enum variant (Enum::Variant)
        if self.check(TokenKind::ColonColon) {
            return self.enum_variant_expr(ident);
        }

        // Check for struct init - need to disambiguate from block expressions
        // Struct init: Foo { field: value } or Foo { field }
        // Block: identifier followed by { ... } where ... is not field syntax
        if self.check(TokenKind::LBrace) && self.looks_like_struct_init() {
            return self.struct_init(ident);
        }

        Ok(Expr::new(ExprKind::Ident(ident.clone()), ident.span))
    }

    /// Parse an enum variant expression (Enum::Variant or Enum::Variant(data))
    fn enum_variant_expr(&mut self, enum_name: Ident) -> ParseResult<Expr> {
        let start = enum_name.span.start;
        self.expect(TokenKind::ColonColon)?;
        let variant = self.expect_ident()?;

        // Check for variant data (Enum::Variant(data))
        let data = if self.check(TokenKind::LParen) {
            self.expect(TokenKind::LParen)?;
            if self.check(TokenKind::RParen) {
                // Empty parens - not typical for tuple variants
                self.expect(TokenKind::RParen)?;
                None
            } else {
                let expr = self.expression()?;
                self.expect(TokenKind::RParen)?;
                Some(Box::new(expr))
            }
        } else if self.check(TokenKind::LBrace) && self.looks_like_struct_init() {
            // Struct variant: Enum::Variant { field: value }
            return self.struct_variant_init(enum_name, variant);
        } else {
            None
        };

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(
            ExprKind::EnumVariant {
                enum_name: Some(enum_name),
                variant,
                data,
            },
            Span::new(start, end),
        ))
    }

    /// Parse struct variant initialization (Enum::Variant { field: value })
    fn struct_variant_init(&mut self, enum_name: Ident, variant: Ident) -> ParseResult<Expr> {
        let start = enum_name.span.start;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let field_start = self.current().span.start;
            let field_name = self.expect_ident()?;

            let value = if self.eat(TokenKind::Colon).is_some() {
                Some(self.expression()?)
            } else {
                None // Shorthand syntax
            };

            let field_end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(field_start);

            fields.push(FieldInit {
                name: field_name,
                value,
                span: Span::new(field_start, field_end),
            });

            if !self.eat(TokenKind::Comma).is_some() && !self.check(TokenKind::RBrace) {
                break;
            }
        }

        self.expect(TokenKind::RBrace)?;

        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        // For struct variants, we encode field info in the data field
        // by creating a StructInit expression as the data
        let struct_init = Expr::new(
            ExprKind::StructInit {
                name: variant.clone(),
                fields,
            },
            Span::new(variant.span.start, end),
        );

        Ok(Expr::new(
            ExprKind::EnumVariant {
                enum_name: Some(enum_name),
                variant,
                data: Some(Box::new(struct_init)),
            },
            Span::new(start, end),
        ))
    }

    /// Look ahead to determine if this is a struct init or a block
    fn looks_like_struct_init(&self) -> bool {
        // Skip the opening brace
        let mut pos = self.position + 1;

        // Skip trivia
        while pos < self.tokens.len() && self.tokens[pos].kind.is_trivia() {
            pos += 1;
        }

        if pos >= self.tokens.len() {
            return false;
        }

        // Empty braces {} - ambiguous, treat as block
        if self.tokens[pos].kind == TokenKind::RBrace {
            return false;
        }

        // If we see an identifier, check what follows
        if matches!(
            self.tokens[pos].kind,
            TokenKind::Ident | TokenKind::UnicodeIdent
        ) {
            pos += 1;

            // Skip trivia after identifier
            while pos < self.tokens.len() && self.tokens[pos].kind.is_trivia() {
                pos += 1;
            }

            if pos >= self.tokens.len() {
                return false;
            }

            // If followed by : or , or }, it's struct init
            // ident: value or ident, (shorthand) or ident } (shorthand last field)
            matches!(
                self.tokens[pos].kind,
                TokenKind::Colon | TokenKind::Comma | TokenKind::RBrace
            )
        } else {
            // Not an identifier after { - not struct init
            false
        }
    }

    /// Parse struct initialization
    fn struct_init(&mut self, name: Ident) -> ParseResult<Expr> {
        let start = name.span.start;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let field_start = self.current().span.start;
            let field_name = self.expect_ident()?;

            let value = if self.eat(TokenKind::Colon).is_some() {
                Some(self.expression()?)
            } else {
                None // Shorthand: { x } means { x: x }
            };

            let field_end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(field_start);

            fields.push(FieldInit {
                name: field_name,
                value,
                span: Span::new(field_start, field_end),
            });

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(
            ExprKind::StructInit { name, fields },
            Span::new(start, end),
        ))
    }

    /// Parse parenthesized expression or lambda
    fn paren_or_lambda(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::LParen)?;

        // Empty parens - check for lambda
        if self.check(TokenKind::RParen) {
            self.expect(TokenKind::RParen)?;

            // Check for arrow (lambda with no params)
            if self.check(TokenKind::FatArrow) || self.check(TokenKind::Arrow) {
                return self.complete_lambda(Vec::new(), start);
            }

            // Unit value (empty tuple)
            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            return Ok(Expr::new(
                ExprKind::Literal(Literal::Null), // TODO: proper unit type
                Span::new(start, end),
            ));
        }

        // Parse first element
        let first = self.expression()?;

        // If there's a comma, could be tuple or lambda params
        if self.check(TokenKind::Comma) {
            // Could be tuple or multi-param lambda
            // For now, parse as expression and handle later
            // This is a simplification - full implementation would need lookahead
        }

        self.expect(TokenKind::RParen)?;

        // Check for lambda arrow
        if self.check(TokenKind::FatArrow) || self.check(TokenKind::Arrow) {
            // Convert expression to param list - this only works for simple identifiers
            let params = self.expr_to_params(first)?;
            return self.complete_lambda(params, start);
        }

        // Regular parenthesized expression
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(
            ExprKind::Paren(Box::new(first)),
            Span::new(start, end),
        ))
    }

    /// Convert expression to parameter list (for lambda parsing)
    fn expr_to_params(&self, expr: Expr) -> ParseResult<Vec<Param>> {
        match expr.kind {
            ExprKind::Ident(ident) => Ok(vec![Param::simple(ident.name, ident.span)]),
            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedIdentifier,
                expr.span,
            )),
        }
    }

    /// Complete parsing a lambda after params
    fn complete_lambda(&mut self, params: Vec<Param>, start: u32) -> ParseResult<Expr> {
        // Optional return type
        let return_type = if self.eat(TokenKind::Arrow).is_some() {
            Some(self.type_annotation()?)
        } else {
            self.eat(TokenKind::FatArrow);
            None
        };

        // Body
        let body = if self.check(TokenKind::LBrace) {
            let block = self.block()?;
            Expr::new(ExprKind::Block(block.clone()), block.span)
        } else {
            self.expression()?
        };

        let end = body.span.end;

        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                return_type,
                body: Box::new(body),
            },
            Span::new(start, end),
        ))
    }

    /// Parse lambda with pipe syntax (|x| ...)
    fn lambda_expr(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::Pipe)?;

        // Parse parameters
        let mut params = Vec::new();
        while !self.check(TokenKind::Pipe) && !self.is_eof() {
            params.push(self.param()?);
            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::Pipe)?;

        // Optional return type
        let return_type = if self.eat(TokenKind::Arrow).is_some() {
            Some(self.type_annotation()?)
        } else {
            None
        };

        // Body
        let body = if self.check(TokenKind::LBrace) {
            let block = self.block()?;
            Expr::new(ExprKind::Block(block.clone()), block.span)
        } else {
            self.expression()?
        };

        let end = body.span.end;

        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                return_type,
                body: Box::new(body),
            },
            Span::new(start, end),
        ))
    }

    /// Parse block expression or map literal
    fn block_or_map(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::LBrace)?;

        // Empty braces - empty block
        if self.check(TokenKind::RBrace) {
            self.expect(TokenKind::RBrace)?;
            let end = self
                .tokens
                .get(self.position.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            return Ok(Expr::new(
                ExprKind::Block(Block::empty(Span::new(start, end))),
                Span::new(start, end),
            ));
        }

        // Try to detect if this is a map literal
        // Map literals have the form { key: value, ... } where key is usually a string
        if self.check(TokenKind::StringStart) {
            // Likely a map literal
            return self.map_literal_after_brace(start);
        }

        // Check for Ident: which could be map or block with struct init
        // For now, assume block unless we see string key
        // This is a simplification

        // Parse as block
        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            match self.statement_or_expr() {
                Ok(StmtOrExpr::Stmt(stmt)) => stmts.push(stmt),
                Ok(StmtOrExpr::Expr(expr)) => {
                    if self.check(TokenKind::RBrace) {
                        trailing_expr = Some(expr);
                    } else if self.eat(TokenKind::Semicolon).is_some() {
                        stmts.push(Stmt::expr(
                            expr,
                            Span::new(
                                start,
                                self.tokens
                                    .get(self.position.saturating_sub(1))
                                    .map(|t| t.span.end)
                                    .unwrap_or(start),
                            ),
                        ));
                    } else {
                        let span = expr.span;
                        stmts.push(Stmt::expr(expr, span));
                        break;
                    }
                }
                Err(e) => {
                    self.error(e);
                    self.synchronize_in_block();
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        let block = Block::new(stmts, trailing_expr, Span::new(start, end));
        Ok(Expr::new(ExprKind::Block(block), Span::new(start, end)))
    }

    /// Parse map literal after opening brace
    fn map_literal_after_brace(&mut self, start: u32) -> ParseResult<Expr> {
        let mut pairs = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let key = self.expression()?;
            self.expect(TokenKind::Colon)?;
            let value = self.expression()?;
            pairs.push((key, value));

            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(ExprKind::Map(pairs), Span::new(start, end)))
    }

    /// Parse list literal
    fn list_literal(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::LBracket)?;

        let mut elements = Vec::new();
        while !self.check(TokenKind::RBracket) && !self.is_eof() {
            elements.push(self.expression()?);
            if !self.eat(TokenKind::Comma).is_some() {
                break;
            }
        }

        self.expect(TokenKind::RBracket)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(ExprKind::List(elements), Span::new(start, end)))
    }

    /// Parse if expression
    fn if_expr(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::If)?;

        let cond = self.expression()?;
        let then_branch = self.block()?;

        let else_branch = if self.eat(TokenKind::Else).is_some() {
            if self.check(TokenKind::If) {
                // else if
                let else_if = self.if_expr()?;
                Some(ElseBranch::ElseIf(Box::new(else_if)))
            } else {
                // else block
                let else_block = self.block()?;
                Some(ElseBranch::Block(else_block))
            }
        } else {
            None
        };

        let end = match &else_branch {
            Some(ElseBranch::Block(b)) => b.span.end,
            Some(ElseBranch::ElseIf(e)) => e.span.end,
            None => then_branch.span.end,
        };

        Ok(Expr::new(
            ExprKind::If {
                cond: Box::new(cond),
                then_branch,
                else_branch,
            },
            Span::new(start, end),
        ))
    }

    /// Parse match expression
    fn match_expr(&mut self) -> ParseResult<Expr> {
        let start = self.current().span.start;
        self.expect(TokenKind::Match)?;

        let expr = self.expression()?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_eof() {
            let arm_start = self.current().span.start;
            let pattern = self.pattern()?;

            // Optional guard
            let guard = if self.eat(TokenKind::If).is_some() {
                Some(self.expression()?)
            } else {
                None
            };

            self.expect(TokenKind::FatArrow)?;

            let body = if self.check(TokenKind::LBrace) {
                let block = self.block()?;
                Expr::new(ExprKind::Block(block.clone()), block.span)
            } else {
                self.expression()?
            };

            let arm_end = body.span.end;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: Span::new(arm_start, arm_end),
            });

            // Optional comma
            self.eat(TokenKind::Comma);
        }

        self.expect(TokenKind::RBrace)?;
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(Expr::new(
            ExprKind::Match {
                expr: Box::new(expr),
                arms,
            },
            Span::new(start, end),
        ))
    }

    // ==================== Error Recovery ====================

    /// Synchronize parser state after an error
    fn synchronize(&mut self) {
        while !self.is_eof() {
            // Stop at statement boundaries
            if self.current_kind() == TokenKind::Semicolon {
                self.advance();
                return;
            }

            // Stop at keywords that start new statements/items
            match self.current_kind() {
                TokenKind::Fx
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Interface
                | TokenKind::Impl
                | TokenKind::Import
                | TokenKind::Let
                | TokenKind::For
                | TokenKind::While
                | TokenKind::If
                | TokenKind::Return
                | TokenKind::Try => return,
                _ => {}
            }

            self.advance();
        }
    }

    /// Synchronize within a block
    fn synchronize_in_block(&mut self) {
        while !self.is_eof() && !self.check(TokenKind::RBrace) {
            if self.current_kind() == TokenKind::Semicolon {
                self.advance();
                return;
            }

            match self.current_kind() {
                TokenKind::Let
                | TokenKind::For
                | TokenKind::While
                | TokenKind::If
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Try => return,
                _ => {}
            }

            self.advance();
        }
    }
}

// ==================== Helper Functions ====================

/// Parse an integer from a lexeme
fn parse_int(lexeme: &str, kind: &TokenKind) -> Result<i64, String> {
    let clean = lexeme.replace('_', "");
    match kind {
        TokenKind::Int => clean
            .parse()
            .map_err(|_| format!("invalid integer: {lexeme}")),
        TokenKind::HexInt => {
            let hex = clean.trim_start_matches("0x").trim_start_matches("0X");
            i64::from_str_radix(hex, 16).map_err(|_| format!("invalid hex integer: {lexeme}"))
        }
        TokenKind::BinaryInt => {
            let bin = clean.trim_start_matches("0b").trim_start_matches("0B");
            i64::from_str_radix(bin, 2).map_err(|_| format!("invalid binary integer: {lexeme}"))
        }
        TokenKind::OctalInt => {
            let oct = clean.trim_start_matches("0o").trim_start_matches("0O");
            i64::from_str_radix(oct, 8).map_err(|_| format!("invalid octal integer: {lexeme}"))
        }
        _ => Err(format!("not an integer token: {lexeme}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_expr(source: &str) -> Result<Expr, Vec<ParseError>> {
        Parser::parse_expression(source)
    }

    fn parse_module(source: &str) -> Result<Module, Vec<ParseError>> {
        Parser::parse_module(source)
    }

    #[test]
    fn parse_integer_literals() {
        let expr = parse_expr("42").unwrap();
        assert!(matches!(expr.kind, ExprKind::Literal(Literal::Int(42))));

        let expr = parse_expr("0xFF").unwrap();
        assert!(matches!(expr.kind, ExprKind::Literal(Literal::Int(255))));

        let expr = parse_expr("0b1010").unwrap();
        assert!(matches!(expr.kind, ExprKind::Literal(Literal::Int(10))));
    }

    #[test]
    fn parse_float_literals() {
        let expr = parse_expr("3.14").unwrap();
        if let ExprKind::Literal(Literal::Float(f)) = expr.kind {
            assert!((f - 3.14).abs() < 0.001);
        } else {
            panic!("expected float literal");
        }
    }

    #[test]
    fn parse_string_literals() {
        let expr = parse_expr(r#""hello""#).unwrap();
        assert!(matches!(expr.kind, ExprKind::Literal(Literal::String(s)) if s == "hello"));
    }

    #[test]
    fn parse_binary_expressions() {
        let expr = parse_expr("1 + 2").unwrap();
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinOp::Add, .. }));

        let expr = parse_expr("1 + 2 * 3").unwrap();
        // Should parse as 1 + (2 * 3) due to precedence
        if let ExprKind::Binary { op, right, .. } = expr.kind {
            assert_eq!(op, BinOp::Add);
            assert!(matches!(
                right.kind,
                ExprKind::Binary { op: BinOp::Mul, .. }
            ));
        } else {
            panic!("expected binary expression");
        }
    }

    #[test]
    fn parse_unary_expressions() {
        let expr = parse_expr("-42").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));

        let expr = parse_expr("!true").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn parse_function_calls() {
        let expr = parse_expr("foo()").unwrap();
        assert!(matches!(expr.kind, ExprKind::Call { args, .. } if args.is_empty()));

        let expr = parse_expr("foo(1, 2, 3)").unwrap();
        assert!(matches!(expr.kind, ExprKind::Call { args, .. } if args.len() == 3));
    }

    #[test]
    fn parse_field_access() {
        let expr = parse_expr("obj.field").unwrap();
        assert!(matches!(expr.kind, ExprKind::Field { .. }));

        let expr = parse_expr("obj?.field").unwrap();
        assert!(matches!(expr.kind, ExprKind::NullSafeField { .. }));
    }

    #[test]
    fn parse_if_expression() {
        let expr = parse_expr("if x { 1 } else { 2 }").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::If {
                else_branch: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parse_list_literal() {
        let expr = parse_expr("[1, 2, 3]").unwrap();
        assert!(matches!(expr.kind, ExprKind::List(v) if v.len() == 3));
    }

    #[test]
    fn parse_function_definition() {
        let module = parse_module("fx add(a: Int, b: Int) -> Int { a + b }").unwrap();
        assert_eq!(module.items.len(), 1);
        assert!(matches!(module.items[0].kind, ItemKind::Function(_)));
    }

    #[test]
    fn parse_struct_definition() {
        let module = parse_module("struct Point { x: Int, y: Int }").unwrap();
        assert_eq!(module.items.len(), 1);
        assert!(matches!(module.items[0].kind, ItemKind::Struct(_)));
    }

    #[test]
    fn parse_let_statement() {
        let module = parse_module("fx main() { let x = 42 }").unwrap();
        if let ItemKind::Function(f) = &module.items[0].kind {
            assert!(!f.body.stmts.is_empty() || f.body.expr.is_some());
        }
    }

    #[test]
    fn parse_lambda_expression() {
        let expr = parse_expr("|x| x + 1").unwrap();
        assert!(matches!(expr.kind, ExprKind::Lambda { .. }));
    }

    #[test]
    fn parse_range_expression() {
        let expr = parse_expr("0..10").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinOp::Range,
                ..
            }
        ));

        let expr = parse_expr("0..=10").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinOp::RangeInclusive,
                ..
            }
        ));
    }

    #[test]
    fn parse_pipeline() {
        let expr = parse_expr("x |> foo |> bar").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinOp::Pipe,
                ..
            }
        ));
    }

    #[test]
    fn parse_null_coalescing() {
        let expr = parse_expr("x ?? default").unwrap();
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinOp::NullCoalesce,
                ..
            }
        ));
    }
}
