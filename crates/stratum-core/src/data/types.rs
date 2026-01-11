//! Type mapping between Stratum and Apache Arrow types

use arrow::datatypes::DataType as ArrowDataType;

use crate::types::Type;

/// Convert a Stratum type to an Arrow data type
///
/// # Returns
/// - `Some(ArrowDataType)` for types that have Arrow equivalents
/// - `None` for types that cannot be represented in Arrow
#[must_use]
pub fn stratum_to_arrow_type(ty: &Type) -> Option<ArrowDataType> {
    match ty {
        Type::Int => Some(ArrowDataType::Int64),
        Type::Float => Some(ArrowDataType::Float64),
        Type::Bool => Some(ArrowDataType::Boolean),
        Type::String => Some(ArrowDataType::Utf8),
        Type::List(inner) => {
            let inner_arrow = stratum_to_arrow_type(inner)?;
            Some(ArrowDataType::List(
                arrow::datatypes::Field::new("item", inner_arrow, true).into(),
            ))
        }
        Type::Nullable(inner) => {
            // Arrow handles nullability at the field level, not type level
            // Return the inner type - nullability is handled separately
            stratum_to_arrow_type(inner)
        }
        // Types that don't have direct Arrow equivalents
        Type::Null
        | Type::Map(..)
        | Type::Function { .. }
        | Type::Tuple(..)
        | Type::Struct { .. }
        | Type::Enum { .. }
        | Type::TypeVar(..)
        | Type::Unit
        | Type::Never
        | Type::Error
        | Type::Any
        | Type::Future(..)
        | Type::Range
        | Type::Namespace(_) => None,
    }
}

/// Convert an Arrow data type to a Stratum type
#[must_use]
pub fn arrow_to_stratum_type(arrow_type: &ArrowDataType) -> Type {
    match arrow_type {
        // Integer types - all map to Stratum Int
        ArrowDataType::Int8
        | ArrowDataType::Int16
        | ArrowDataType::Int32
        | ArrowDataType::Int64
        | ArrowDataType::UInt8
        | ArrowDataType::UInt16
        | ArrowDataType::UInt32
        | ArrowDataType::UInt64 => Type::Int,

        // Float types - all map to Stratum Float
        ArrowDataType::Float16 | ArrowDataType::Float32 | ArrowDataType::Float64 => Type::Float,

        // Boolean
        ArrowDataType::Boolean => Type::Bool,

        // String types
        ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 => Type::String,

        // List types
        ArrowDataType::List(field) | ArrowDataType::LargeList(field) => {
            let inner = arrow_to_stratum_type(field.data_type());
            Type::list(inner)
        }

        // Null type
        ArrowDataType::Null => Type::Null,

        // Date/Time types - represent as Int (timestamp) for now
        ArrowDataType::Date32
        | ArrowDataType::Date64
        | ArrowDataType::Time32(_)
        | ArrowDataType::Time64(_)
        | ArrowDataType::Timestamp(_, _)
        | ArrowDataType::Duration(_)
        | ArrowDataType::Interval(_) => Type::Int,

        // Binary types - represent as List<Int>
        ArrowDataType::Binary | ArrowDataType::LargeBinary | ArrowDataType::FixedSizeBinary(_) => {
            Type::list(Type::Int)
        }

        // Decimal types - represent as Float
        ArrowDataType::Decimal128(_, _) | ArrowDataType::Decimal256(_, _) => Type::Float,

        // Struct types - use Error type for now (would need schema info)
        ArrowDataType::Struct(_) => Type::Error,

        // Dictionary types - use the value type
        ArrowDataType::Dictionary(_, value_type) => arrow_to_stratum_type(value_type),

        // Map types - use Error for now
        ArrowDataType::Map(_, _) => Type::Error,

        // Union types - use Error for now
        ArrowDataType::Union(_, _) => Type::Error,

        // Fixed size list
        ArrowDataType::FixedSizeList(field, _) => {
            let inner = arrow_to_stratum_type(field.data_type());
            Type::list(inner)
        }

        // Other types default to Error
        _ => Type::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stratum_to_arrow_basic_types() {
        assert_eq!(
            stratum_to_arrow_type(&Type::Int),
            Some(ArrowDataType::Int64)
        );
        assert_eq!(
            stratum_to_arrow_type(&Type::Float),
            Some(ArrowDataType::Float64)
        );
        assert_eq!(
            stratum_to_arrow_type(&Type::Bool),
            Some(ArrowDataType::Boolean)
        );
        assert_eq!(
            stratum_to_arrow_type(&Type::String),
            Some(ArrowDataType::Utf8)
        );
    }

    #[test]
    fn test_stratum_to_arrow_list() {
        let list_int = Type::list(Type::Int);
        let arrow_type = stratum_to_arrow_type(&list_int).unwrap();
        match arrow_type {
            ArrowDataType::List(field) => {
                assert_eq!(field.data_type(), &ArrowDataType::Int64);
            }
            _ => panic!("Expected List type"),
        }
    }

    #[test]
    fn test_stratum_to_arrow_nullable() {
        let nullable_int = Type::nullable(Type::Int);
        // Nullable unwraps to the inner type
        assert_eq!(
            stratum_to_arrow_type(&nullable_int),
            Some(ArrowDataType::Int64)
        );
    }

    #[test]
    fn test_arrow_to_stratum_integers() {
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Int8), Type::Int);
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Int16), Type::Int);
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Int32), Type::Int);
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Int64), Type::Int);
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::UInt64), Type::Int);
    }

    #[test]
    fn test_arrow_to_stratum_floats() {
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Float32), Type::Float);
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Float64), Type::Float);
    }

    #[test]
    fn test_arrow_to_stratum_string() {
        assert_eq!(arrow_to_stratum_type(&ArrowDataType::Utf8), Type::String);
        assert_eq!(
            arrow_to_stratum_type(&ArrowDataType::LargeUtf8),
            Type::String
        );
    }

    #[test]
    fn test_roundtrip() {
        // Basic types should roundtrip
        let types = vec![Type::Int, Type::Float, Type::Bool, Type::String];
        for ty in types {
            let arrow = stratum_to_arrow_type(&ty).unwrap();
            let back = arrow_to_stratum_type(&arrow);
            assert_eq!(ty, back, "Roundtrip failed for {:?}", ty);
        }
    }
}
