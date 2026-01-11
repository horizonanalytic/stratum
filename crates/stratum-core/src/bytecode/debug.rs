//! Bytecode disassembler for debugging

use super::chunk::Chunk;
use super::opcode::OpCode;
use super::value::Value;
use std::fmt::Write;

/// Disassemble a chunk to a string
pub fn disassemble_chunk(chunk: &Chunk, name: &str) -> String {
    let mut output = String::new();

    writeln!(output, "== {name} ==").unwrap();

    let mut offset = 0;
    while offset < chunk.len() {
        offset = disassemble_instruction_to_string(chunk, offset, &mut output);
    }

    output
}

/// Disassemble a single instruction to stdout
pub fn disassemble_instruction(chunk: &Chunk, offset: usize) {
    let mut output = String::new();
    disassemble_instruction_to_string(chunk, offset, &mut output);
    print!("{output}");
}

/// Disassemble a single instruction to a string, returning the next offset
fn disassemble_instruction_to_string(chunk: &Chunk, offset: usize, output: &mut String) -> usize {
    // Print offset
    write!(output, "{offset:04} ").unwrap();

    // Print line number (or | if same as previous)
    let line = chunk.get_line(offset);
    if offset > 0 && line == chunk.get_line(offset - 1) {
        write!(output, "   | ").unwrap();
    } else {
        write!(output, "{line:4} ").unwrap();
    }

    // Read opcode
    let Some(byte) = chunk.read_byte(offset) else {
        writeln!(output, "Invalid offset").unwrap();
        return offset + 1;
    };

    let Ok(opcode) = OpCode::try_from(byte) else {
        writeln!(output, "Unknown opcode {byte}").unwrap();
        return offset + 1;
    };

    // Print instruction based on type
    match opcode {
        // No operand
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
        | OpCode::Breakpoint => {
            writeln!(output, "{}", opcode.name()).unwrap();
            offset + 1
        }

        // u8 operand
        OpCode::Call | OpCode::LoadUpvalue | OpCode::StoreUpvalue | OpCode::PopBelow => {
            let operand = chunk.read_byte(offset + 1).unwrap_or(0);
            writeln!(output, "{:16} {}", opcode.name(), operand).unwrap();
            offset + 2
        }

        // u16 constant operand
        OpCode::Const => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let constant = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {}",
                opcode.name(),
                idx,
                format_constant(constant)
            )
            .unwrap();
            offset + 3
        }

        // u16 local slot operand
        OpCode::LoadLocal | OpCode::StoreLocal => {
            let slot = chunk.read_u16(offset + 1).unwrap_or(0);
            writeln!(output, "{:16} {}", opcode.name(), slot).unwrap();
            offset + 3
        }

        // u16 global name operand
        OpCode::LoadGlobal | OpCode::StoreGlobal | OpCode::DefineGlobal => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let name = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {}",
                opcode.name(),
                idx,
                format_constant(name)
            )
            .unwrap();
            offset + 3
        }

        // i16 jump offset
        OpCode::Jump
        | OpCode::JumpIfFalse
        | OpCode::JumpIfTrue
        | OpCode::JumpIfNull
        | OpCode::JumpIfNotNull
        | OpCode::PopJumpIfNull
        | OpCode::Loop => {
            let jump = chunk.read_i16(offset + 1).unwrap_or(0);
            let target = (offset as isize + 3 + jump as isize) as usize;
            writeln!(output, "{:16} {:4} -> {}", opcode.name(), jump, target).unwrap();
            offset + 3
        }

        // u16 field name/path operand
        OpCode::GetField
        | OpCode::SetField
        | OpCode::GetProperty
        | OpCode::NullSafeGetField
        | OpCode::NullSafeGetIndex
        | OpCode::StateBinding => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let name = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {}",
                opcode.name(),
                idx,
                format_constant(name)
            )
            .unwrap();
            offset + 3
        }

        // u16 count operand
        OpCode::NewList | OpCode::NewMap | OpCode::NewSet | OpCode::StringConcat => {
            let count = chunk.read_u16(offset + 1).unwrap_or(0);
            writeln!(output, "{:16} {}", opcode.name(), count).unwrap();
            offset + 3
        }

        // u16 type/struct name operand
        OpCode::IsInstance | OpCode::NewEnumVariant | OpCode::MatchVariant => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let name = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {}",
                opcode.name(),
                idx,
                format_constant(name)
            )
            .unwrap();
            offset + 3
        }

        // NewStruct: u16 type name index, u16 field count
        OpCode::NewStruct => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let count = chunk.read_u16(offset + 3).unwrap_or(0);
            let name = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {} (fields: {})",
                opcode.name(),
                idx,
                format_constant(name),
                count
            )
            .unwrap();
            offset + 5
        }

        // i16 offset for iterator
        OpCode::IterNext => {
            let jump = chunk.read_i16(offset + 1).unwrap_or(0);
            let target = (offset as isize + 3 + jump as isize) as usize;
            writeln!(output, "{:16} {:4} -> {}", opcode.name(), jump, target).unwrap();
            offset + 3
        }

        // Closure (u16 constant + upvalue descriptors)
        OpCode::Closure => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let constant = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {}",
                opcode.name(),
                idx,
                format_constant(constant)
            )
            .unwrap();

            // Print upvalue descriptors
            let mut off = offset + 3;
            if let Some(Value::Function(func)) = constant {
                for i in 0..func.upvalue_count {
                    let is_local = chunk.read_byte(off).unwrap_or(0);
                    let index = chunk.read_byte(off + 1).unwrap_or(0);
                    writeln!(
                        output,
                        "{:04}      |                     {} {}",
                        off,
                        if is_local == 1 { "local" } else { "upvalue" },
                        index
                    )
                    .unwrap();
                    off += 2;
                    let _ = i; // Suppress unused warning
                }
            }
            off
        }

        // Invoke (u16 method name + u8 arg count)
        OpCode::Invoke => {
            let idx = chunk.read_u16(offset + 1).unwrap_or(0);
            let arg_count = chunk.read_byte(offset + 3).unwrap_or(0);
            let name = chunk.get_constant(idx);
            writeln!(
                output,
                "{:16} {:4} {} ({})",
                opcode.name(),
                idx,
                format_constant(name),
                arg_count
            )
            .unwrap();
            offset + 4
        }

        // PushHandler (i16 handler offset + i16 finally offset)
        OpCode::PushHandler => {
            let handler = chunk.read_i16(offset + 1).unwrap_or(0);
            let finally = chunk.read_i16(offset + 3).unwrap_or(0);
            let handler_target = (offset as isize + 5 + handler as isize) as usize;
            let finally_target = if finally != 0 {
                Some((offset as isize + 5 + finally as isize) as usize)
            } else {
                None
            };
            writeln!(
                output,
                "{:16} handler -> {}, finally -> {:?}",
                opcode.name(),
                handler_target,
                finally_target
            )
            .unwrap();
            offset + 5
        }
    }
}

fn format_constant(constant: Option<&Value>) -> String {
    match constant {
        Some(Value::String(s)) => format!("'{s}'"),
        Some(Value::Function(f)) => format!("<fn {}>", f.name),
        Some(v) => format!("{v:?}"),
        None => "<invalid>".to_string(),
    }
}

/// Trace execution of an instruction (for debugging VM)
pub fn trace_instruction(chunk: &Chunk, offset: usize, stack: &[Value]) {
    // Print stack
    print!("          ");
    for value in stack {
        print!("[ {value:?} ]");
    }
    println!();

    // Print instruction
    disassemble_instruction(chunk, offset);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disassemble_simple() {
        let mut chunk = Chunk::new();
        chunk.write_op(OpCode::Null, 1);
        chunk.write_op(OpCode::Return, 1);

        let output = disassemble_chunk(&chunk, "test");
        assert!(output.contains("NULL"));
        assert!(output.contains("RETURN"));
    }

    #[test]
    fn disassemble_constant() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(42), 1);
        chunk.write_op(OpCode::Return, 1);

        let output = disassemble_chunk(&chunk, "test");
        assert!(output.contains("CONST"));
        assert!(output.contains("42"));
    }

    #[test]
    fn disassemble_jump() {
        let mut chunk = Chunk::new();
        let jump = chunk.emit_jump(OpCode::Jump, 1);
        chunk.write_op(OpCode::Null, 2);
        chunk.patch_jump(jump);
        chunk.write_op(OpCode::Return, 3);

        let output = disassemble_chunk(&chunk, "test");
        assert!(output.contains("JUMP"));
        assert!(output.contains("->"));
    }
}
