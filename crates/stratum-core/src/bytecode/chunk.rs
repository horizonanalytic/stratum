//! Bytecode chunk - a sequence of instructions with constants and debug info

use super::opcode::OpCode;
use super::value::Value;

/// A chunk of bytecode
///
/// Contains the raw bytecode instructions, a constant pool, and line
/// number information for debugging and error messages.
#[derive(Clone, Default)]
pub struct Chunk {
    /// Raw bytecode instructions
    code: Vec<u8>,

    /// Constant pool
    constants: Vec<Value>,

    /// Line number information (run-length encoded)
    /// Each entry is (line_number, count) meaning `count` bytes at this line
    lines: Vec<(u32, u32)>,

    /// Source file name (for error messages)
    pub source_name: Option<String>,
}

impl Chunk {
    /// Create a new empty chunk
    #[must_use]
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
            source_name: None,
        }
    }

    /// Create a new chunk with a source name
    #[must_use]
    pub fn with_source(source_name: impl Into<String>) -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
            source_name: Some(source_name.into()),
        }
    }

    /// Returns the number of bytes in the chunk
    #[must_use]
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Returns true if the chunk is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Returns the raw bytecode
    #[must_use]
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Returns a mutable reference to the raw bytecode
    pub fn code_mut(&mut self) -> &mut Vec<u8> {
        &mut self.code
    }

    /// Returns the constant pool
    #[must_use]
    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    /// Get a constant by index
    #[must_use]
    pub fn get_constant(&self, index: u16) -> Option<&Value> {
        self.constants.get(index as usize)
    }

    /// Write a single byte to the chunk
    pub fn write_byte(&mut self, byte: u8, line: u32) {
        self.code.push(byte);
        self.add_line(line, 1);
    }

    /// Write an opcode to the chunk
    pub fn write_op(&mut self, op: OpCode, line: u32) {
        self.write_byte(op as u8, line);
    }

    /// Write an opcode with a u8 operand
    pub fn write_op_u8(&mut self, op: OpCode, operand: u8, line: u32) {
        self.write_byte(op as u8, line);
        self.write_byte(operand, line);
    }

    /// Write an opcode with a u16 operand
    pub fn write_op_u16(&mut self, op: OpCode, operand: u16, line: u32) {
        self.write_byte(op as u8, line);
        self.write_u16(operand, line);
    }

    /// Write a u16 value (little-endian)
    pub fn write_u16(&mut self, value: u16, line: u32) {
        self.write_byte((value & 0xFF) as u8, line);
        self.write_byte((value >> 8) as u8, line);
    }

    /// Write an i16 value (little-endian)
    pub fn write_i16(&mut self, value: i16, line: u32) {
        self.write_u16(value as u16, line);
    }

    /// Read a byte at a position
    #[must_use]
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        self.code.get(offset).copied()
    }

    /// Read a u16 at a position (little-endian)
    #[must_use]
    pub fn read_u16(&self, offset: usize) -> Option<u16> {
        let low = self.code.get(offset).copied()? as u16;
        let high = self.code.get(offset + 1).copied()? as u16;
        Some(low | (high << 8))
    }

    /// Read an i16 at a position (little-endian)
    #[must_use]
    pub fn read_i16(&self, offset: usize) -> Option<i16> {
        self.read_u16(offset).map(|u| u as i16)
    }

    /// Patch a u16 value at a position
    pub fn patch_u16(&mut self, offset: usize, value: u16) {
        self.code[offset] = (value & 0xFF) as u8;
        self.code[offset + 1] = (value >> 8) as u8;
    }

    /// Patch an i16 value at a position
    pub fn patch_i16(&mut self, offset: usize, value: i16) {
        self.patch_u16(offset, value as u16);
    }

    /// Add a constant to the pool and return its index
    ///
    /// Returns `None` if the constant pool is full (> 65535 constants).
    pub fn add_constant(&mut self, value: Value) -> Option<u16> {
        // Check for existing identical constant (deduplication)
        for (i, existing) in self.constants.iter().enumerate() {
            if values_identical(existing, &value) {
                return Some(i as u16);
            }
        }

        let index = self.constants.len();
        if index > u16::MAX as usize {
            return None;
        }
        self.constants.push(value);
        Some(index as u16)
    }

    /// Emit a constant instruction
    ///
    /// Writes `OpCode::Const` followed by the constant index.
    /// Returns `None` if the constant pool is full.
    pub fn emit_constant(&mut self, value: Value, line: u32) -> Option<u16> {
        let index = self.add_constant(value)?;
        self.write_op_u16(OpCode::Const, index, line);
        Some(index)
    }

    /// Add line information for `count` bytes
    fn add_line(&mut self, line: u32, count: u32) {
        if let Some(last) = self.lines.last_mut() {
            if last.0 == line {
                // Same line, extend the count
                last.1 += count;
                return;
            }
        }
        // New line
        self.lines.push((line, count));
    }

    /// Get the line number for a bytecode offset
    #[must_use]
    pub fn get_line(&self, offset: usize) -> u32 {
        let mut current_offset = 0;
        for (line, count) in &self.lines {
            current_offset += *count as usize;
            if offset < current_offset {
                return *line;
            }
        }
        // Default to last line or 0
        self.lines.last().map_or(0, |(line, _)| *line)
    }

    /// Get the current bytecode offset (for jump targets)
    #[must_use]
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// Emit a jump instruction and return the offset to patch
    ///
    /// The jump offset is initially set to 0 and should be patched later.
    pub fn emit_jump(&mut self, op: OpCode, line: u32) -> usize {
        self.write_op(op, line);
        let patch_offset = self.code.len();
        self.write_i16(0, line); // Placeholder
        patch_offset
    }

    /// Patch a jump instruction to jump to the current position
    pub fn patch_jump(&mut self, patch_offset: usize) {
        let jump_target = self.code.len();
        let offset = (jump_target as isize - patch_offset as isize - 2) as i16;
        self.patch_i16(patch_offset, offset);
    }

    /// Emit a loop instruction that jumps back to the given offset
    pub fn emit_loop(&mut self, loop_start: usize, line: u32) {
        self.write_op(OpCode::Loop, line);
        // Calculate backwards jump (negative offset)
        let offset = self.code.len() - loop_start + 2;
        self.write_i16(-(offset as i16), line);
    }
}

/// Check if two values are identical (for constant deduplication)
fn values_identical(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        // Don't deduplicate other types
        _ => false,
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("code_len", &self.code.len())
            .field("constants_len", &self.constants.len())
            .field("source_name", &self.source_name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_write_read() {
        let mut chunk = Chunk::new();

        chunk.write_op(OpCode::Const, 1);
        chunk.write_u16(0x1234, 1);

        assert_eq!(chunk.read_byte(0), Some(OpCode::Const as u8));
        assert_eq!(chunk.read_u16(1), Some(0x1234));
    }

    #[test]
    fn chunk_constants() {
        let mut chunk = Chunk::new();

        let idx1 = chunk.add_constant(Value::Int(42)).unwrap();
        let idx2 = chunk.add_constant(Value::Int(42)).unwrap();
        let idx3 = chunk.add_constant(Value::Int(100)).unwrap();

        // Same value should return same index (deduplication)
        assert_eq!(idx1, idx2);
        // Different value gets new index
        assert_eq!(idx3, 1);

        assert_eq!(chunk.get_constant(idx1), Some(&Value::Int(42)));
        assert_eq!(chunk.get_constant(idx3), Some(&Value::Int(100)));
    }

    #[test]
    fn chunk_line_info() {
        let mut chunk = Chunk::new();

        chunk.write_op(OpCode::Const, 1);
        chunk.write_u16(0, 1);
        chunk.write_op(OpCode::Return, 2);

        assert_eq!(chunk.get_line(0), 1); // Const opcode
        assert_eq!(chunk.get_line(1), 1); // First byte of u16
        assert_eq!(chunk.get_line(2), 1); // Second byte of u16
        assert_eq!(chunk.get_line(3), 2); // Return opcode
    }

    #[test]
    fn chunk_jump_patching() {
        let mut chunk = Chunk::new();

        chunk.write_op(OpCode::Null, 1);
        let patch = chunk.emit_jump(OpCode::JumpIfFalse, 1);
        chunk.write_op(OpCode::Null, 2);
        chunk.write_op(OpCode::Pop, 2);
        chunk.patch_jump(patch);
        chunk.write_op(OpCode::Return, 3);

        // Jump should skip 2 bytes (Null + Pop)
        let jump_offset = chunk.read_i16(patch).unwrap();
        assert_eq!(jump_offset, 2);
    }

    #[test]
    fn chunk_loop() {
        let mut chunk = Chunk::new();

        let loop_start = chunk.current_offset();
        chunk.write_op(OpCode::Null, 1);
        chunk.write_op(OpCode::Pop, 1);
        chunk.emit_loop(loop_start, 1);

        // Should be: Null(0), Pop(1), Loop(2), offset(3-4)
        assert_eq!(chunk.read_byte(2), Some(OpCode::Loop as u8));
        // Loop offset should jump back 5 bytes (to offset 0)
        let loop_offset = chunk.read_i16(3).unwrap();
        assert_eq!(loop_offset, -5);
    }
}
