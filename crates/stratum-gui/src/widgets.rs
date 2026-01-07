//! Widget abstractions for Stratum GUI
//!
//! This module provides Stratum-friendly widget types that wrap iced widgets.
//! These will be exposed to Stratum code as native GUI primitives.

use stratum_core::bytecode::Value;

/// Text styling options
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    /// Font size in pixels
    pub font_size: Option<f32>,
    /// Whether text is bold
    pub bold: bool,
    /// Text color (r, g, b, a)
    pub color: Option<(u8, u8, u8, u8)>,
}

/// Layout spacing configuration
#[derive(Debug, Clone, Default)]
pub struct LayoutConfig {
    /// Spacing between children
    pub spacing: f32,
    /// Padding around content
    pub padding: f32,
}

/// Convert a Stratum Value to a display string
#[must_use]
pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(),
        _ => format!("{value:?}"),
    }
}

/// Extract an integer from a Stratum Value
pub fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Int(i) => Some(*i),
        Value::Float(f) => Some(*f as i64),
        _ => None,
    }
}

/// Extract a float from a Stratum Value
pub fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    }
}

/// Extract a boolean from a Stratum Value
pub fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// Represents a resolved state binding that can be used for two-way binding
#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    /// The path to the state field (e.g., "state.count" or "user.name")
    pub path: String,
    /// The current value at this path
    pub value: Value,
}

impl ResolvedBinding {
    /// Create a new resolved binding
    #[must_use]
    pub fn new(path: String, value: Value) -> Self {
        Self { path, value }
    }

    /// Get the binding path
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get a reference to the current value
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// Check if a value is a state binding
#[must_use]
pub fn is_state_binding(value: &Value) -> bool {
    matches!(value, Value::StateBinding(_))
}

/// Extract the binding path from a state binding value
#[must_use]
pub fn get_binding_path(value: &Value) -> Option<&str> {
    match value {
        Value::StateBinding(path) => Some(path.as_str()),
        _ => None,
    }
}

/// Resolve a value that may be a state binding.
///
/// If the value is a `StateBinding`, this returns information about the binding.
/// If not, returns `None` and the value should be used directly.
#[must_use]
pub fn resolve_binding(value: &Value) -> Option<String> {
    match value {
        Value::StateBinding(path) => Some(path.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&Value::Null), "null");
        assert_eq!(value_to_string(&Value::Int(42)), "42");
        assert_eq!(value_to_string(&Value::Float(3.14)), "3.14");
        assert_eq!(value_to_string(&Value::Bool(true)), "true");
        assert_eq!(
            value_to_string(&Value::String(Rc::new("hello".to_string()))),
            "hello"
        );
    }

    #[test]
    fn test_value_to_i64() {
        assert_eq!(value_to_i64(&Value::Int(42)), Some(42));
        assert_eq!(value_to_i64(&Value::Float(3.9)), Some(3));
        assert_eq!(value_to_i64(&Value::Null), None);
    }

    #[test]
    fn test_value_to_f64() {
        assert_eq!(value_to_f64(&Value::Float(3.14)), Some(3.14));
        assert_eq!(value_to_f64(&Value::Int(42)), Some(42.0));
        assert_eq!(value_to_f64(&Value::Null), None);
    }

    #[test]
    fn test_value_to_bool() {
        assert_eq!(value_to_bool(&Value::Bool(true)), Some(true));
        assert_eq!(value_to_bool(&Value::Bool(false)), Some(false));
        assert_eq!(value_to_bool(&Value::Int(1)), None);
    }
}
