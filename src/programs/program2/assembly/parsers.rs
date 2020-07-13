use crate::{ math::field };
use crate::processor::opcodes2::{ UserOps as Opcode };
use super::{ AssemblyError, HintMap, OpHint };

// CONSTANTS
// ================================================================================================
const PUSH_OP_ALIGNMENT: usize = 8;
const HASH_OP_ALIGNMENT: usize = 16;

// CONTROL FLOW OPERATIONS
// ================================================================================================

/// Appends a NOOP operations to the program.
pub fn parse_noop(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 {
        return Err(AssemblyError::extra_param(op, step));
    }
    program.push(Opcode::Noop);
    return Ok(true);
}

/// Appends either ASSERT or ASSERTEQ operations to the program.
pub fn parse_assert(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 2 {
        return Err(AssemblyError::extra_param(op, step));
    }
    else if op.len() == 1 {
        program.push(Opcode::Assert);
    }
    else if op[1] == "eq" {
        program.push(Opcode::AssertEq);
    }
    else {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [eq]", op[1])));
    }
    
    return Ok(true);
}

// INPUT OPERATIONS
// ================================================================================================

/// Appends a PUSH operation to the program.
pub fn parse_push(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let value = read_value(op, step)?;
    append_push_op(program, hints, value);
    return Ok(true);
}

/// Makes sure PUSH operation alignment is correct and appends PUSH opcode to the program.
fn append_push_op(program: &mut Vec<Opcode>, hints: &mut HintMap, value: u128) {
    // pad the program with NOOPs to make sure PUSH happens on steps which are multiples of 8
    let alignment = program.len() % PUSH_OP_ALIGNMENT;
    let pad_length = (PUSH_OP_ALIGNMENT - alignment) % PUSH_OP_ALIGNMENT;
    program.resize(program.len() + pad_length, Opcode::Noop);
    
    // read the value to be pushed onto the stack
    hints.insert(program.len(), OpHint::PushValue(value));

    // add PUSH opcode to the program
    program.push(Opcode::Push);
}

/// Appends either READ or READ2 operation to the program.
pub fn parse_read(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 2 {
        return Err(AssemblyError::extra_param(op, step));
    }
    else if op.len() == 1 || op[1] == "a" {
        program.push(Opcode::Read);
    }
    else if op[1] == "ab" {
        program.push(Opcode::Read2);
    }
    else {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [a, ab]", op[1])));
    }

    return Ok(true);
}

// STACK MANIPULATION OPERATIONS
// ================================================================================================

/// Appends a sequence of operations to the program to duplicate top n values of the stack.
pub fn parse_dup(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.push(Opcode::Dup),
        2 => program.push(Opcode::Dup2),
        3 => program.extend_from_slice(&[Opcode::Dup4, Opcode::Roll4, Opcode::Drop]),
        4 => program.push(Opcode::Dup4),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 3, 4]", n)))
    };

    return Ok(true);
}

/// Appends a sequence of operations to the program to pad the stack with n zeros.
pub fn parse_pad(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Drop]),
        2 => program.push(Opcode::Pad2),
        3 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Drop]),
        4 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2]),
        5 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Pad2, Opcode::Drop]),
        6 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Pad2]),
        7 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Dup4, Opcode::Drop]),
        8 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Dup4]),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 3, 4, 5, 6, 7, 8]", n)))
    }

    return Ok(true);
}

/// Appends a sequence of operations to the program to copy n-th item to the top of the stack.
pub fn parse_pick(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.extend_from_slice(&[Opcode::Dup2, Opcode::Drop]),
        2 => program.extend_from_slice(&[
            Opcode::Dup4, Opcode::Roll4, Opcode::Drop, Opcode::Drop, Opcode::Drop
        ]),
        3 => program.extend_from_slice(&[Opcode::Dup4, Opcode::Drop, Opcode::Drop, Opcode::Drop]),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 3]", n)))
    };

    return Ok(true);
}

/// Appends a sequence of operations to the program to remove top n values from the stack.
pub fn parse_drop(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.push(Opcode::Drop),
        2 => program.extend_from_slice(&[Opcode::Drop, Opcode::Drop]),
        3 => program.extend_from_slice(&[Opcode::Dup, Opcode::Drop4]),
        4 => program.push(Opcode::Drop4),
        5 => program.extend_from_slice(&[Opcode::Drop, Opcode::Drop4]),
        6 => program.extend_from_slice(&[Opcode::Drop, Opcode::Drop, Opcode::Drop4]),
        7 => program.extend_from_slice(&[Opcode::Dup, Opcode::Drop4, Opcode::Drop4]),
        8 => program.extend_from_slice(&[Opcode::Drop4, Opcode::Drop4]),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 3, 4, 5, 6, 7, 8]", n)))
    }

    return Ok(true);
}

/// Appends a sequence of operations to the program to swap n values at the top of the stack
/// with the following n values.
pub fn parse_swap(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.push(Opcode::Swap),
        2 => program.push(Opcode::Swap2),
        4 => program.push(Opcode::Swap4),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 4]", n)))
    }

    return Ok(true);
}

/// Appends either ROLL4 or ROLL8 operation to the program.
pub fn parse_roll(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        4 => program.push(Opcode::Roll4),
        8 => program.push(Opcode::Roll8),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [4, 8]", n)))
    }

    return Ok(true);
}

// ARITHMETIC AND BOOLEAN OPERATIONS
// ================================================================================================

/// Appends ADD operation to the program.
pub fn parse_add(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Add);
    return Ok(true);
}

/// Appends NEG ADD operations to the program.
pub fn parse_sub(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.extend_from_slice(&[Opcode::Neg, Opcode::Add]);
    return Ok(true);
}

/// Appends MUL operation to the program.
pub fn parse_mul(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Mul);
    return Ok(true);
}

/// Appends INV MUL operations to the program.
pub fn parse_div(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.extend_from_slice(&[Opcode::Inv, Opcode::Mul]);
    return Ok(true);
}

/// Appends NEG operation to the program.
pub fn parse_neg(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Neg);
    return Ok(true);
}

/// Appends INV operation to the program.
pub fn parse_inv(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Inv);
    return Ok(true);
}

/// Appends NOT operation to the program.
pub fn parse_not(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Not);
    return Ok(true);
}

/// Appends AND operation to the program.
pub fn parse_and(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::And);
    return Ok(true);
}

/// Appends OR operation to the program.
pub fn parse_or(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    program.push(Opcode::Or);
    return Ok(true);
}

// COMPARISON OPERATIONS
// ================================================================================================

/// Appends EQ operation to the program.
pub fn parse_eq(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    if op.len() > 1 { return Err(AssemblyError::extra_param(op, step)); }
    hints.insert(program.len(), OpHint::EqStart);
    program.extend_from_slice(&[Opcode::Read, Opcode::Eq]);
    return Ok(true);
}

/// Appends a sequence of operations to the program to determine whether the top value on the 
/// stack is greater than the following value.
pub fn parse_gt(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    // n is the number of bits sufficient to represent each value; if either of the
    // values does not fit into n bits, the operation fill fail.
    let n = read_param(op, step)?;
    if n < 4 || n > 128 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; value must be between 4 and 128", n)))
    }

    // prepare the stack
    program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Pad2, Opcode::Dup]);
    let power_of_two = u128::pow(2, n - 1);
    append_push_op(program, hints, power_of_two);

    // add a hint indicating that value comparison is about to start
    hints.insert(program.len(), OpHint::CmpStart(n));

    // append CMP operations
    program.resize(program.len() + (n as usize), Opcode::Cmp);

    // compare binary aggregation values with the original values, and drop everything
    // but the GT value from the stack
    program.extend_from_slice(&[
        Opcode::Drop4,    Opcode::Pad2,     Opcode::Swap4, Opcode::Roll4,
        Opcode::AssertEq, Opcode::AssertEq, Opcode::Roll4, Opcode::Dup,
        Opcode::Drop4
    ]);
    return Ok(true);
}

/// Appends a sequence of operations to the program to determine whether the top value on the 
/// stack is less than the following value.
pub fn parse_lt(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    // n is the number of bits sufficient to represent each value; if either of the
    // values does not fit into n bits, the operation fill fail.
    let n = read_param(op, step)?;
    if n < 4 || n > 128 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; value must be between 4 and 128", n)))
    }

    // prepare the stack
    program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Pad2, Opcode::Dup]);
    let power_of_two = u128::pow(2, n - 1);
    append_push_op(program, hints, power_of_two);

    // add a hint indicating that value comparison is about to start
    hints.insert(program.len(), OpHint::CmpStart(n));

    // append CMP operations
    program.resize(program.len() + (n as usize), Opcode::Cmp);

    // compare binary aggregation values with the original values, and drop everything
    // but the LT value from the stack
    program.extend_from_slice(&[
        Opcode::Drop4,    Opcode::Pad2,     Opcode::Swap4, Opcode::Roll4,
        Opcode::AssertEq, Opcode::AssertEq, Opcode::Dup,   Opcode::Drop4
    ]);
    return Ok(true);
}

/// Appends a sequence of operations to the program to determine whether the top value on the 
/// stack can be represented with n bits.
pub fn parse_rc(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    // n is the number of bits against which to test the binary decomposition
    let n = read_param(op, step)?;
    if n < 4 || n > 128 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; value must be between 4 and 128", n)))
    }

    // prepare the stack
    program.push(Opcode::Pad2);
    let power_of_two = u128::pow(2, n - 1);
    append_push_op(program, hints, power_of_two);

    // add a hint indicating that range-checking is about to start
    hints.insert(program.len(), OpHint::RcStart(n));

    // append BINACC operations
    program.resize(program.len() + (n as usize), Opcode::BinAcc);

    // compare binary aggregation value with the original value
    program.extend_from_slice(&[Opcode::Drop, Opcode::Drop]);
    hints.insert(program.len(), OpHint::EqStart);
    program.extend_from_slice(&[Opcode::Read, Opcode::Eq]);
    return Ok(true);
}

/// Appends a sequence of operations to the program to determine whether the top value on the 
/// stack is odd.
pub fn parse_isodd(program: &mut Vec<Opcode>, hints: &mut HintMap, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    // n is the number of bits sufficient to represent top stack value;
    // if the values does not fit into n bits, the operation fill fail.
    let n = read_param(op, step)?;
    if n < 4 || n > 128 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; value must be between 4 and 128", n)))
    }

    // prepare the stack
    program.extend_from_slice(&[Opcode::Pad2]);
    let power_of_two = u128::pow(2, n - 1);
    append_push_op(program, hints, power_of_two);

    // add a hint indicating that range-checking is about to start
    hints.insert(program.len(), OpHint::RcStart(n));

    // append BINACC operations
    program.resize(program.len() + (n as usize), Opcode::BinAcc);

    // compare binary aggregation value with the original value and drop all
    // values used in computations except for the least significant bit of the value
    program.extend_from_slice(&[Opcode::Swap2, Opcode::AssertEq, Opcode::Drop]);
    return Ok(true);
}

// SELECTOR OPERATIONS
// ================================================================================================

/// Appends either CHOOSE or CHOOSE2 operation to the program.
pub fn parse_choose(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.push(Opcode::Choose),
        2 => program.push(Opcode::Choose2),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2]", n)))
    }
    return Ok(true);
}

// CRYPTO OPERATIONS
// ================================================================================================

/// Appends a sequence of operations to the program to hash top n values of the stack.
pub fn parse_hash(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    match n {
        1 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Pad2, Opcode::Drop]),
        2 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2]),
        3 => program.extend_from_slice(&[Opcode::Pad2, Opcode::Pad2, Opcode::Drop]),
        4 => program.push(Opcode::Pad2),
        _ => return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; allowed values are: [1, 2, 3, 4]", n)))
    }

    // pad with NOOPs to make sure hashing starts on a step which is a multiple of 16
    let alignment = program.len() % HASH_OP_ALIGNMENT;
    let pad_length = (HASH_OP_ALIGNMENT - alignment) % HASH_OP_ALIGNMENT;
    program.resize(program.len() + pad_length, Opcode::Noop);

    // append operations to execute 10 rounds of Rescue
    program.extend_from_slice(&[
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR,
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR
    ]);

    // truncate the state
    program.push(Opcode::Drop4);

    return Ok(true);
}

/// Appends a sequence of operations to the program to compute the root of Merkle
/// authentication path for a tree of depth n.
pub fn parse_mpath(program: &mut Vec<Opcode>, op: &[&str], step: usize) -> Result<bool, AssemblyError> {
    let n = read_param(op, step)?;
    if n < 2 || n > 256 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter {} is invalid; value must be between 2 and 256", n)))
    }

    // read the first node in the Merkle path and push it onto the stack;
    // also pad the stack to prepare it for hashing.
    program.extend_from_slice(&[Opcode::Read2, Opcode::Dup4, Opcode::Pad2]);

    // pad with NOOPs to make sure hashing starts on a step which is a multiple of 16
    let alignment = program.len() % HASH_OP_ALIGNMENT;
    let pad_length = (HASH_OP_ALIGNMENT - alignment) % HASH_OP_ALIGNMENT;
    program.resize(program.len() + pad_length, Opcode::Noop);

    // repeat the following cycle of operations once for each remaining node:
    // 1. compute hash(p, v)
    // 2. read next bit of position index
    // 3. compute hash(v, p)
    // 4. base on position index bit, choses either hash(p, v) or hash(v, p)
    // 5. reads the next nodes and pushes it onto the stack
    const SUB_CYCLE: [Opcode; 32] = [
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR,
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR,
        Opcode::RescR, Opcode::RescR, Opcode::Drop4, Opcode::Read2,
        Opcode::Swap2, Opcode::Swap4, Opcode::Swap2, Opcode::Pad2,
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR,
        Opcode::RescR, Opcode::RescR, Opcode::RescR, Opcode::RescR,
        Opcode::RescR, Opcode::RescR, Opcode::Drop4, Opcode::Choose2,
        Opcode::Read2, Opcode::Dup4,  Opcode::Pad2,  Opcode::Noop
    ];

    for _ in 0..(n - 2) {
        program.extend_from_slice(&SUB_CYCLE);
    }

    // at the end, use the same cycle except for the last 4 operations
    // since there is no need to read in any additional nodes
    program.extend_from_slice(&SUB_CYCLE[..28]);

    return Ok(true);
}

// HELPER FUNCTIONS
// ================================================================================================

fn read_param(op: &[&str], step: usize) -> Result<u32, AssemblyError> {
    if op.len() == 1 {
        // if no parameters were provided, assume parameter value 1
        return Ok(1);
    } else if op.len() > 2 {
        return Err(AssemblyError::extra_param(op, step));
    }

    // try to parse the parameter value
    let result = match op[1].parse::<u32>() {
        Ok(i) => i,
        Err(_) => return Err(AssemblyError::invalid_param(op, step))
    };

    // parameter value 0 is never valid
    if result == 0 {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter value must be greater than 0")));
    }

    return Ok(result);
}

fn read_value(op: &[&str], step: usize) -> Result<u128, AssemblyError> {
    // make sure exactly 1 parameter was supplied
    if op.len() == 1 {
        return Err(AssemblyError::missing_param(op, step));
    }
    else if op.len() > 2 {
        return Err(AssemblyError::extra_param(op, step));
    }

    let result = if op[1].starts_with("0x") {
        // parse hexadecimal number
        match u128::from_str_radix(&op[1][2..], 16) {
            Ok(i) => i,
            Err(_) => return Err(AssemblyError::invalid_param(op, step))
        }
    }
    else {
        // parse decimal number
        match u128::from_str_radix(&op[1], 10) {
            Ok(i) => i,
            Err(_) => return Err(AssemblyError::invalid_param(op, step))
        }
    };

    // make sure the value is a valid field element
    if result >= field::MODULUS {
        return Err(AssemblyError::invalid_param_reason(op, step,
            format!("parameter value must be smaller than {}", field::MODULUS)));
    }

    return Ok(result);
}