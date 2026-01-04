//! Bytecode instruction set for the Stratum virtual machine

/// Bytecode operation codes
///
/// This is a stack-based instruction set. Most operations pop operands from
/// the stack and push results back onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpCode {
    // ===== Stack Operations =====
    /// Push a constant from the constant pool onto the stack
    /// Operand: u16 constant index
    Const,

    /// Push null onto the stack
    Null,

    /// Push true onto the stack
    True,

    /// Push false onto the stack
    False,

    /// Pop and discard the top of stack
    Pop,

    /// Duplicate the top of stack
    Dup,

    /// Pop N values from below the top of stack, keeping the top value
    /// Operand: u8 count
    /// Used for block expressions to clean up locals while preserving the result
    PopBelow,

    // ===== Local Variables =====
    /// Load a local variable onto the stack
    /// Operand: u16 local slot index
    LoadLocal,

    /// Store top of stack into a local variable (does not pop)
    /// Operand: u16 local slot index
    StoreLocal,

    // ===== Global Variables =====
    /// Load a global variable onto the stack
    /// Operand: u16 constant index (name)
    LoadGlobal,

    /// Store top of stack into a global variable (does not pop)
    /// Operand: u16 constant index (name)
    StoreGlobal,

    /// Define a global variable with initial value from stack
    /// Operand: u16 constant index (name)
    DefineGlobal,

    // ===== Upvalues (Closures) =====
    /// Load an upvalue (captured variable) onto the stack
    /// Operand: u8 upvalue index
    LoadUpvalue,

    /// Store top of stack into an upvalue
    /// Operand: u8 upvalue index
    StoreUpvalue,

    /// Close upvalues up to and including the stack slot
    CloseUpvalue,

    // ===== Arithmetic Operations =====
    /// Add: pop two values, push their sum
    Add,

    /// Subtract: pop two values (right, left), push left - right
    Sub,

    /// Multiply: pop two values, push their product
    Mul,

    /// Divide: pop two values (right, left), push left / right
    Div,

    /// Modulo: pop two values (right, left), push left % right
    Mod,

    /// Negate: pop one value, push its negation
    Neg,

    // ===== Comparison Operations =====
    /// Equal: pop two values, push true if equal
    Eq,

    /// Not equal: pop two values, push true if not equal
    Ne,

    /// Less than: pop two values, push true if left < right
    Lt,

    /// Less than or equal: pop two values, push true if left <= right
    Le,

    /// Greater than: pop two values, push true if left > right
    Gt,

    /// Greater than or equal: pop two values, push true if left >= right
    Ge,

    // ===== Logical Operations =====
    /// Logical NOT: pop one value, push its logical negation
    Not,

    // ===== Control Flow =====
    /// Unconditional jump
    /// Operand: i16 offset (relative to current position)
    Jump,

    /// Jump if top of stack is falsy (pops the condition)
    /// Operand: i16 offset
    JumpIfFalse,

    /// Jump if top of stack is truthy (pops the condition)
    /// Operand: i16 offset
    JumpIfTrue,

    /// Jump if top of stack is null (does NOT pop)
    /// Used for null coalescing
    /// Operand: i16 offset
    JumpIfNull,

    /// Jump if top of stack is NOT null (does NOT pop)
    /// Used for null-safe access
    /// Operand: i16 offset
    JumpIfNotNull,

    /// Pop and jump if non-null, otherwise continue with null on stack
    /// Used for optional chaining
    PopJumpIfNull,

    // ===== Loops =====
    /// Loop back (like Jump but for loop constructs, helps with break/continue)
    /// Operand: i16 offset (always negative, jumping backwards)
    Loop,

    // ===== Function Calls =====
    /// Call a function
    /// Operand: u8 argument count
    Call,

    /// Return from current function
    /// If stack has a value, it becomes the return value
    Return,

    // ===== Closures =====
    /// Create a closure from a function constant
    /// Operand: u16 constant index (Function)
    /// Followed by upvalue descriptors (pairs of u8: is_local, index)
    Closure,

    // ===== Object Operations =====
    /// Get a field from a struct/object
    /// Operand: u16 constant index (field name)
    GetField,

    /// Set a field on a struct/object
    /// Stack: [object, value] -> [value]
    /// Operand: u16 constant index (field name)
    SetField,

    /// Get a property (field or method)
    /// Similar to GetField but also handles bound methods
    /// Operand: u16 constant index (property name)
    GetProperty,

    // ===== Index Operations =====
    /// Get by index: pop index and collection, push element
    GetIndex,

    /// Set by index: pop value, index, and collection, push value
    SetIndex,

    // ===== Collection Literals =====
    /// Create a new list
    /// Operand: u16 element count (elements are on stack)
    NewList,

    /// Create a new map
    /// Operand: u16 entry count (key-value pairs on stack)
    NewMap,

    /// Create a new struct instance
    /// Operand: u16 constant index (struct type)
    /// Stack: field values in order
    NewStruct,

    // ===== Iteration =====
    /// Get an iterator from an iterable
    /// Pops iterable, pushes iterator
    GetIter,

    /// Advance iterator and push next value
    /// If exhausted, pushes sentinel and jumps
    /// Operand: i16 offset to jump when exhausted
    IterNext,

    // ===== Exception Handling =====
    /// Throw an exception
    /// Pops the exception value from stack
    Throw,

    /// Push an exception handler
    /// Operand: i16 offset to handler, i16 offset to finally (or 0)
    PushHandler,

    /// Pop an exception handler (on normal exit from try block)
    PopHandler,

    // ===== String Operations =====
    /// Concatenate strings for interpolation
    /// Operand: u16 part count
    StringConcat,

    // ===== Range Operations =====
    /// Create an exclusive range (start..end)
    NewRange,

    /// Create an inclusive range (start..=end)
    NewRangeInclusive,

    // ===== Type Operations =====
    /// Check if value is null
    IsNull,

    /// Check if value is an instance of a type
    /// Operand: u16 constant index (type)
    IsInstance,

    // ===== Method Invocation =====
    /// Invoke a method directly (optimization for obj.method() calls)
    /// Operand: u16 constant index (method name), u8 argument count
    Invoke,

    // ===== Enum Operations =====
    /// Create an enum variant
    /// Operand: u16 constant index (variant info)
    NewEnumVariant,

    /// Match an enum variant, pushes the extracted data or fails
    /// Operand: u16 constant index (variant to match)
    MatchVariant,

    // ===== Null-Safe Operations =====
    /// Null-safe field access (obj?.field)
    /// If obj is null, pushes null; otherwise gets field
    /// Operand: u16 constant index (field name)
    NullSafeGetField,

    /// Null-safe index access (obj?.[index])
    /// If obj is null, pushes null; otherwise indexes
    NullSafeGetIndex,

    // ===== Async Operations (for future use) =====
    /// Await a future/promise
    Await,

    // ===== Debugging =====
    /// Breakpoint for debugger (no-op in normal execution)
    Breakpoint,
}

impl OpCode {
    /// Returns the size of the instruction including operands
    #[must_use]
    pub const fn size(self) -> usize {
        match self {
            // No operand instructions (1 byte)
            OpCode::Null
            | OpCode::True
            | OpCode::False
            | OpCode::Pop
            | OpCode::Dup
            | OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::Neg
            | OpCode::Eq
            | OpCode::Ne
            | OpCode::Lt
            | OpCode::Le
            | OpCode::Gt
            | OpCode::Ge
            | OpCode::Not
            | OpCode::Return
            | OpCode::GetIndex
            | OpCode::SetIndex
            | OpCode::GetIter
            | OpCode::Throw
            | OpCode::PopHandler
            | OpCode::NewRange
            | OpCode::NewRangeInclusive
            | OpCode::IsNull
            | OpCode::Await
            | OpCode::CloseUpvalue
            | OpCode::Breakpoint => 1,

            // Single u8 operand (2 bytes)
            OpCode::Call | OpCode::LoadUpvalue | OpCode::StoreUpvalue | OpCode::PopBelow => 2,

            // Single u16 operand (3 bytes)
            OpCode::Const
            | OpCode::LoadLocal
            | OpCode::StoreLocal
            | OpCode::LoadGlobal
            | OpCode::StoreGlobal
            | OpCode::DefineGlobal
            | OpCode::Jump
            | OpCode::JumpIfFalse
            | OpCode::JumpIfTrue
            | OpCode::JumpIfNull
            | OpCode::JumpIfNotNull
            | OpCode::PopJumpIfNull
            | OpCode::Loop
            | OpCode::Closure
            | OpCode::GetField
            | OpCode::SetField
            | OpCode::GetProperty
            | OpCode::NewList
            | OpCode::NewMap
            | OpCode::NewStruct
            | OpCode::IterNext
            | OpCode::StringConcat
            | OpCode::IsInstance
            | OpCode::NewEnumVariant
            | OpCode::MatchVariant
            | OpCode::NullSafeGetField
            | OpCode::NullSafeGetIndex => 3,

            // u16 + u8 operand (4 bytes)
            OpCode::Invoke => 4,

            // i16 + i16 operand (5 bytes)
            OpCode::PushHandler => 5,
        }
    }

    /// Returns a human-readable name for the opcode
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            OpCode::Const => "CONST",
            OpCode::Null => "NULL",
            OpCode::True => "TRUE",
            OpCode::False => "FALSE",
            OpCode::Pop => "POP",
            OpCode::Dup => "DUP",
            OpCode::PopBelow => "POP_BELOW",
            OpCode::LoadLocal => "LOAD_LOCAL",
            OpCode::StoreLocal => "STORE_LOCAL",
            OpCode::LoadGlobal => "LOAD_GLOBAL",
            OpCode::StoreGlobal => "STORE_GLOBAL",
            OpCode::DefineGlobal => "DEFINE_GLOBAL",
            OpCode::LoadUpvalue => "LOAD_UPVALUE",
            OpCode::StoreUpvalue => "STORE_UPVALUE",
            OpCode::CloseUpvalue => "CLOSE_UPVALUE",
            OpCode::Add => "ADD",
            OpCode::Sub => "SUB",
            OpCode::Mul => "MUL",
            OpCode::Div => "DIV",
            OpCode::Mod => "MOD",
            OpCode::Neg => "NEG",
            OpCode::Eq => "EQ",
            OpCode::Ne => "NE",
            OpCode::Lt => "LT",
            OpCode::Le => "LE",
            OpCode::Gt => "GT",
            OpCode::Ge => "GE",
            OpCode::Not => "NOT",
            OpCode::Jump => "JUMP",
            OpCode::JumpIfFalse => "JUMP_IF_FALSE",
            OpCode::JumpIfTrue => "JUMP_IF_TRUE",
            OpCode::JumpIfNull => "JUMP_IF_NULL",
            OpCode::JumpIfNotNull => "JUMP_IF_NOT_NULL",
            OpCode::PopJumpIfNull => "POP_JUMP_IF_NULL",
            OpCode::Loop => "LOOP",
            OpCode::Call => "CALL",
            OpCode::Return => "RETURN",
            OpCode::Closure => "CLOSURE",
            OpCode::GetField => "GET_FIELD",
            OpCode::SetField => "SET_FIELD",
            OpCode::GetProperty => "GET_PROPERTY",
            OpCode::GetIndex => "GET_INDEX",
            OpCode::SetIndex => "SET_INDEX",
            OpCode::NewList => "NEW_LIST",
            OpCode::NewMap => "NEW_MAP",
            OpCode::NewStruct => "NEW_STRUCT",
            OpCode::GetIter => "GET_ITER",
            OpCode::IterNext => "ITER_NEXT",
            OpCode::Throw => "THROW",
            OpCode::PushHandler => "PUSH_HANDLER",
            OpCode::PopHandler => "POP_HANDLER",
            OpCode::StringConcat => "STRING_CONCAT",
            OpCode::NewRange => "NEW_RANGE",
            OpCode::NewRangeInclusive => "NEW_RANGE_INCLUSIVE",
            OpCode::IsNull => "IS_NULL",
            OpCode::IsInstance => "IS_INSTANCE",
            OpCode::Invoke => "INVOKE",
            OpCode::NewEnumVariant => "NEW_ENUM_VARIANT",
            OpCode::MatchVariant => "MATCH_VARIANT",
            OpCode::NullSafeGetField => "NULL_SAFE_GET_FIELD",
            OpCode::NullSafeGetIndex => "NULL_SAFE_GET_INDEX",
            OpCode::Await => "AWAIT",
            OpCode::Breakpoint => "BREAKPOINT",
        }
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TryFrom<u8> for OpCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Match all variants by their discriminant
        match value {
            0 => Ok(OpCode::Const),
            1 => Ok(OpCode::Null),
            2 => Ok(OpCode::True),
            3 => Ok(OpCode::False),
            4 => Ok(OpCode::Pop),
            5 => Ok(OpCode::Dup),
            6 => Ok(OpCode::PopBelow),
            7 => Ok(OpCode::LoadLocal),
            8 => Ok(OpCode::StoreLocal),
            9 => Ok(OpCode::LoadGlobal),
            10 => Ok(OpCode::StoreGlobal),
            11 => Ok(OpCode::DefineGlobal),
            12 => Ok(OpCode::LoadUpvalue),
            13 => Ok(OpCode::StoreUpvalue),
            14 => Ok(OpCode::CloseUpvalue),
            15 => Ok(OpCode::Add),
            16 => Ok(OpCode::Sub),
            17 => Ok(OpCode::Mul),
            18 => Ok(OpCode::Div),
            19 => Ok(OpCode::Mod),
            20 => Ok(OpCode::Neg),
            21 => Ok(OpCode::Eq),
            22 => Ok(OpCode::Ne),
            23 => Ok(OpCode::Lt),
            24 => Ok(OpCode::Le),
            25 => Ok(OpCode::Gt),
            26 => Ok(OpCode::Ge),
            27 => Ok(OpCode::Not),
            28 => Ok(OpCode::Jump),
            29 => Ok(OpCode::JumpIfFalse),
            30 => Ok(OpCode::JumpIfTrue),
            31 => Ok(OpCode::JumpIfNull),
            32 => Ok(OpCode::JumpIfNotNull),
            33 => Ok(OpCode::PopJumpIfNull),
            34 => Ok(OpCode::Loop),
            35 => Ok(OpCode::Call),
            36 => Ok(OpCode::Return),
            37 => Ok(OpCode::Closure),
            38 => Ok(OpCode::GetField),
            39 => Ok(OpCode::SetField),
            40 => Ok(OpCode::GetProperty),
            41 => Ok(OpCode::GetIndex),
            42 => Ok(OpCode::SetIndex),
            43 => Ok(OpCode::NewList),
            44 => Ok(OpCode::NewMap),
            45 => Ok(OpCode::NewStruct),
            46 => Ok(OpCode::GetIter),
            47 => Ok(OpCode::IterNext),
            48 => Ok(OpCode::Throw),
            49 => Ok(OpCode::PushHandler),
            50 => Ok(OpCode::PopHandler),
            51 => Ok(OpCode::StringConcat),
            52 => Ok(OpCode::NewRange),
            53 => Ok(OpCode::NewRangeInclusive),
            54 => Ok(OpCode::IsNull),
            55 => Ok(OpCode::IsInstance),
            56 => Ok(OpCode::Invoke),
            57 => Ok(OpCode::NewEnumVariant),
            58 => Ok(OpCode::MatchVariant),
            59 => Ok(OpCode::NullSafeGetField),
            60 => Ok(OpCode::NullSafeGetIndex),
            61 => Ok(OpCode::Await),
            62 => Ok(OpCode::Breakpoint),
            _ => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_size_consistency() {
        // Every opcode should have a valid size >= 1
        for i in 0..=61 {
            if let Ok(op) = OpCode::try_from(i) {
                assert!(op.size() >= 1, "OpCode {:?} has invalid size", op);
            }
        }
    }

    #[test]
    fn opcode_roundtrip() {
        // All opcodes should round-trip through u8
        for i in 0..=61 {
            if let Ok(op) = OpCode::try_from(i) {
                assert_eq!(op as u8, i, "OpCode {:?} has wrong discriminant", op);
            }
        }
    }

    #[test]
    fn opcode_names() {
        assert_eq!(OpCode::Add.name(), "ADD");
        assert_eq!(OpCode::LoadLocal.name(), "LOAD_LOCAL");
        assert_eq!(OpCode::JumpIfFalse.name(), "JUMP_IF_FALSE");
    }
}
