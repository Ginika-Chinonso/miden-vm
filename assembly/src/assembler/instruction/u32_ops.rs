use super::{
    field_ops::append_pow2_op,
    push_u32_value, validate_param, AssemblyError, CodeBlock, Felt,
    Operation::{self, *},
    SpanBuilder, ZERO,
};
use crate::{MAX_U32_ROTATE_VALUE, MAX_U32_SHIFT_VALUE};
use vm_core::AdviceInjector::{Clo, Clz, Cto, Ctz};

// ENUMS
// ================================================================================================

/// This enum is intended to determine the mode of operation passed to the parsing function
#[derive(PartialEq, Eq)]
pub enum U32OpMode {
    Wrapping,
    Overflowing,
}

// CONVERSIONS AND TESTS
// ================================================================================================

/// Translates u32testw assembly instruction to VM operations.
///
/// Implemented by executing DUP U32SPLIT SWAP DROP EQZ on each element in the word
/// and combining the results using AND operation (total of 23 VM cycles)
pub fn u32testw(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    #[rustfmt::skip]
    let ops = [
         // Test the fourth element
        Dup3, U32split, Swap, Drop, Eqz,

        // Test the third element
        Dup3, U32split, Swap, Drop, Eqz, And,

         // Test the second element
        Dup2, U32split, Swap, Drop, Eqz, And,

        // Test the first element
        Dup1, U32split, Swap, Drop, Eqz, And,
    ];
    span.add_ops(ops)
}

/// Translates u32assertw assembly instruction to VM operations.
///
/// Implemented by executing `U32ASSERT2` on each pair of elements in the word.
/// Total of 6 VM cycles.
pub fn u32assertw(
    span: &mut SpanBuilder,
    err_code: Felt,
) -> Result<Option<CodeBlock>, AssemblyError> {
    #[rustfmt::skip]
    let ops = [
        // Test the first and the second elements
        U32assert2(err_code),

        // Move 3 and 4 to the top of the stack
        MovUp3, MovUp3,

        // Test them
        U32assert2(err_code),

        // Move the elements back into place
        MovUp3, MovUp3,
    ];
    span.add_ops(ops)
}

// ARITHMETIC OPERATIONS
// ================================================================================================

/// Translates u32add assembly instructions to VM operations.
///
/// The base operation is `U32ADD`, but depending on the mode, additional operations may be
/// inserted. Please refer to the docs of `handle_arithmetic_operation` for more details.
///
/// VM cycles per mode:
/// - u32wrapping_add: 2 cycles
/// - u32wrapping_add.b: 3 cycles
/// - u32overflowing_add: 1 cycles
/// - u32overflowing_add.b: 2 cycles
pub fn u32add(
    span: &mut SpanBuilder,
    op_mode: U32OpMode,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_arithmetic_operation(span, U32add, op_mode, imm)
}

/// Translates u32sub assembly instructions to VM operations.
///
/// The base operation is `U32SUB`, but depending on the mode, additional operations may be
/// inserted. Please refer to the docs of `handle_arithmetic_operation` for more details.
///
/// VM cycles per mode:
/// - u32wrapping_sub: 2 cycles
/// - u32wrapping_sub.b: 3 cycles
/// - u32overflowing_sub: 1 cycles
/// - u32overflowing_sub.b: 2 cycles
pub fn u32sub(
    span: &mut SpanBuilder,
    op_mode: U32OpMode,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_arithmetic_operation(span, U32sub, op_mode, imm)
}

/// Translates u32mul assembly instructions to VM operations.
///
/// The base operation is `U32MUL`, but depending on the mode, additional operations may be
/// inserted. Please refer to the docs of `handle_arithmetic_operation` for more details.
///
/// VM cycles per mode:
/// - u32wrapping_mul: 2 cycles
/// - u32wrapping_mul.b: 3 cycles
/// - u32overflowing_mul: 1 cycles
/// - u32overflowing_mul.b: 2 cycles
pub fn u32mul(
    span: &mut SpanBuilder,
    op_mode: U32OpMode,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_arithmetic_operation(span, U32mul, op_mode, imm)
}

/// Translates u32div assembly instructions to VM operations.
///
/// VM cycles per mode:
/// - u32div: 2 cycles
/// - u32div.b:
///    - 4 cycles if b is 1
///    - 3 cycles if b is not 1
pub fn u32div(
    span: &mut SpanBuilder,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_division(span, imm)?;
    span.add_op(Drop)
}

/// Translates u32mod assembly instructions to VM operations.
///
/// VM cycles per mode:
/// - u32mod: 3 cycle
/// - u32mod.b:
///    - 5 cycles if b is 1
///    - 4 cycles if b is not 1
pub fn u32mod(
    span: &mut SpanBuilder,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_division(span, imm)?;
    span.add_ops([Swap, Drop])
}

/// Translates u32divmod assembly instructions to VM operations.
///
/// VM cycles per mode:
/// - u32divmod: 1 cycle
/// - u32divmod.b:
///    - 3 cycles if b is 1
///    - 2 cycles if b is not 1
pub fn u32divmod(
    span: &mut SpanBuilder,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    handle_division(span, imm)
}

// ARITHMETIC OPERATIONS - HELPERS
// ================================================================================================

/// Handles U32ADD, U32SUB, and U32MUL operations in wrapping, and overflowing modes, including
/// handling of immediate parameters.
///
/// Specifically handles these specific inputs per the spec.
/// - Wrapping: does not check if the inputs are u32 values; overflow or underflow bits are
///   discarded.
/// - Overflowing: does not check if the inputs are u32 values; overflow or underflow bits are
///   pushed onto the stack.
fn handle_arithmetic_operation(
    span: &mut SpanBuilder,
    op: Operation,
    op_mode: U32OpMode,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    if let Some(imm) = imm {
        push_u32_value(span, imm);
    }

    span.push_op(op);

    // in the wrapping mode, drop high 32 bits
    if matches!(op_mode, U32OpMode::Wrapping) {
        span.add_op(Drop)
    } else {
        Ok(None)
    }
}

/// Handles common parts of u32div, u32mod, and u32divmod operations, including handling of
/// immediate parameters.
fn handle_division(
    span: &mut SpanBuilder,
    imm: Option<u32>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    if let Some(imm) = imm {
        if imm == 0 {
            return Err(AssemblyError::division_by_zero());
        }
        push_u32_value(span, imm);
    }

    span.add_op(U32div)
}

// BITWISE OPERATIONS
// ================================================================================================

/// Translates u32not assembly instruction to VM operations.
///
/// The reason this method works is because 2^32 -1 provides a bit mask of ones, which after
/// subtracting the element, flips the bits of the original value to perform a bitwise NOT.
///
/// This takes 5 VM cycles.
pub fn u32not(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    #[rustfmt::skip]
    let ops = [
        // Perform the operation
        Push(Felt::from(u32::MAX)),
        U32assert2(ZERO),
        Swap,
        U32sub,

        // Drop the underflow flag
        Drop,
    ];
    span.add_ops(ops)
}

/// Translates u32shl assembly instructions to VM operations.
///
/// The operation is implemented by putting a power of 2 on the stack, then multiplying it with
/// the value to be shifted and splitting the result.
///
/// VM cycles per mode:
/// - u32shl: 18 cycles
/// - u32shl.b: 3 cycles
pub fn u32shl(span: &mut SpanBuilder, imm: Option<u8>) -> Result<Option<CodeBlock>, AssemblyError> {
    prepare_bitwise::<MAX_U32_SHIFT_VALUE>(span, imm)?;
    if imm != Some(0) {
        span.add_ops([U32mul, Drop])
    } else {
        Ok(None)
    }
}

/// Translates u32shr assembly instructions to VM operations.
///
/// The operation is implemented by putting a power of 2 on the stack, then dividing the value to
/// be shifted by it and returning the quotient.
///
/// VM cycles per mode:
/// - u32shr: 18 cycles
/// - u32shr.b: 3 cycles
pub fn u32shr(span: &mut SpanBuilder, imm: Option<u8>) -> Result<Option<CodeBlock>, AssemblyError> {
    prepare_bitwise::<MAX_U32_SHIFT_VALUE>(span, imm)?;
    if imm != Some(0) {
        span.add_ops([U32div, Drop])
    } else {
        Ok(None)
    }
}

/// Translates u32rotl assembly instructions to VM operations.
///
/// The base operation is implemented by putting a power of 2 on the stack, then multiplying the
/// value to be shifted by it and adding the overflow limb to the shifted limb.
///
/// VM cycles per mode:
/// - u32rotl: 18 cycles
/// - u32rotl.b: 3 cycles
pub fn u32rotl(
    span: &mut SpanBuilder,
    imm: Option<u8>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    prepare_bitwise::<MAX_U32_ROTATE_VALUE>(span, imm)?;
    if imm != Some(0) {
        span.add_ops([U32mul, Add])
    } else {
        Ok(None)
    }
}

/// Translates u32rotr assembly instructions to VM operations.
///
/// The base operation is implemented by multiplying the value to be shifted by 2^(32-b), where
/// b is the shift amount, then adding the overflow limb to the shifted limb.
///
/// VM cycles per mode:
/// - u32rotr: 22 cycles
/// - u32rotr.b: 3 cycles
pub fn u32rotr(
    span: &mut SpanBuilder,
    imm: Option<u8>,
) -> Result<Option<CodeBlock>, AssemblyError> {
    match imm {
        Some(0) => {
            // if rotation is performed by 0, do nothing (Noop)
            span.push_op(Noop);
            return Ok(None);
        }
        Some(imm) => {
            validate_param(imm, 1..=MAX_U32_ROTATE_VALUE)?;
            span.push_op(Push(Felt::new(1 << (32 - imm))));
        }
        None => {
            span.push_ops([Push(Felt::new(32)), Swap, U32sub, Drop]);
            append_pow2_op(span);
        }
    }
    span.add_ops([U32mul, Add])
}

/// Translates u32popcnt assembly instructions to VM operations.
///
/// This operation takes 33 cycles.
pub fn u32popcnt(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    #[rustfmt::skip]
    let ops = [
        // i = i - ((i >> 1) & 0x55555555);
        Dup0,
        Push(Felt::new(1 << 1)), U32div, Drop,
        Push(Felt::new(0x55555555)),
        U32and,
        U32sub, Drop,
        // i = (i & 0x33333333) + ((i >> 2) & 0x33333333);
        Dup0,
        Push(Felt::new(1 << 2)), U32div, Drop,
        Push(Felt::new(0x33333333)),
        U32and,
        Swap,
        Push(Felt::new(0x33333333)),
        U32and,
        U32add, Drop,
        // i = (i + (i >> 4)) & 0x0F0F0F0F;
        Dup0,
        Push(Felt::new(1 << 4)), U32div, Drop,
        U32add, Drop,
        Push(Felt::new(0x0F0F0F0F)),
        U32and,
        // return (i * 0x01010101) >> 24;
        Push(Felt::new(0x01010101)),
        U32mul, Drop,
        Push(Felt::new(1 << 24)), U32div, Drop
    ];
    span.add_ops(ops)
}

/// Translates `u32clz` assembly instruction to VM operations. `u32clz` counts the number of
/// leading zeros of the value using non-deterministic technique (i.e. it takes help of advice
/// provider).
///
/// This operation takes 27 VM cycles.
pub fn u32clz(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    span.push_advice_injector(Clz);
    span.push_op(AdvPop); // [clz, n, ...]

    calculate_clz(span)
}

/// Translates `u32ctz` assembly instruction to VM operations. `u32ctz` counts the number of
/// trailing zeros of the value using non-deterministic technique (i.e. it takes help of advice
/// provider).
///
/// This operation takes 27 VM cycles.
pub fn u32ctz(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    span.push_advice_injector(Ctz);
    span.push_op(AdvPop); // [ctz, n, ...]

    calculate_ctz(span)
}

/// Translates `u32clo` assembly instruction to VM operations. `u32clo` counts the number of
/// leading ones of the value using non-deterministic technique (i.e. it takes help of advice
/// provider).
///
/// This operation takes 32 VM cycles.
pub fn u32clo(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    span.push_advice_injector(Clo);
    span.push_op(AdvPop); // [clo, n, ...]
    let ops = [Swap, Neg, Push(Felt::new(u32::MAX as u64)), Add, Swap];
    span.push_ops(ops);
    calculate_clz(span)
}

/// Translates `u32cto` assembly instruction to VM operations. `u32cto` counts the number of
/// trailing ones of the value using non-deterministic technique (i.e. it takes help of advice
/// provider).
///
/// This operation takes 32 VM cycles.
pub fn u32cto(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    span.push_advice_injector(Cto);
    span.push_op(AdvPop); // [cto, n, ...]
    let ops = [Swap, Neg, Push(Felt::new(u32::MAX as u64)), Add, Swap];
    span.push_ops(ops);
    calculate_ctz(span)
}

// BITWISE OPERATIONS - HELPERS
// ================================================================================================

/// Mutate the first two elements of the stack from `[b, a, ..]` into `[2^b, a, ..]`, with `b`
/// either as a provided immediate value, or as an element that already exists in the stack.
fn prepare_bitwise<const MAX_VALUE: u8>(
    span: &mut SpanBuilder,
    imm: Option<u8>,
) -> Result<(), AssemblyError> {
    match imm {
        Some(0) => {
            // if shift/rotation is performed by 0, do nothing (Noop)
            span.push_op(Noop);
        }
        Some(imm) => {
            validate_param(imm, 1..=MAX_VALUE)?;
            span.push_op(Push(Felt::new(1 << imm)));
        }
        None => {
            append_pow2_op(span);
        }
    }
    Ok(())
}

/// Appends relevant operations to the span block for the correctness check of the Clz instruction.
///
/// VM cycles: 26
fn calculate_clz(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    // [clz, n, ...]
    #[rustfmt::skip]
    let ops_group_1 = [
        Swap, Dup1, // [clz, n, clz, ...]
    ];
    span.push_ops(ops_group_1);

    append_pow2_op(span); // [pow2(clz), n, clz, ...]

    #[rustfmt::skip]
    let ops_group_2 = [
        Push(Felt::new(u32::MAX.into())), // [2^32 - 1, pow2(clz), n, clz, ...]
        Swap, U32div, Drop, // [bit_mask, n, clz, ...]
        Dup1, U32and, // [m, n, clz, ...] if calculation of clz is correct, m should be equal to n
        Eq, Assert(ZERO) // [clz, ...]
    ];

    span.add_ops(ops_group_2)
}

/// Appends relevant operations to the span block for the correctness check of the Ctz instruction.
///
/// VM cycles: 26
fn calculate_ctz(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    // [ctz, n, ...]
    #[rustfmt::skip]
    let ops_group_1 = [
        Swap, Dup1, // [ctz, n, ctz, ...]
    ];
    span.push_ops(ops_group_1);

    append_pow2_op(span); // [pow2(ctz), n, ctz, ...]

    #[rustfmt::skip]
    let ops_group_2 = [
        Push(Felt::new(u32::MAX as u64 + 1)), // [2^32, pow2(ctz), n, ctz, ...]
        Swap, Neg, Add, // [bit_mask, n, ctz]
        Dup1, U32and, // [m, n, ctz, ...] if calculation of ctz is correct, m should be equal to n
        Eq, Assert(ZERO) // [ctz, ...]
    ];

    span.add_ops(ops_group_2)
}

// COMPARISON OPERATIONS
// ================================================================================================

/// Translates u32lt assembly instructions to VM operations.
///
/// This operation takes 5 cycles.
pub fn u32lt(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    compute_lt(span);

    Ok(None)
}

/// Translates u32lte assembly instructions to VM operations.
///
/// This operation takes 7 cycles.
pub fn u32lte(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    // Compute the lt with reversed number to get a gt check
    span.push_op(Swap);
    compute_lt(span);

    // Flip the final results to get the lte results.
    span.add_op(Not)
}

/// Translates u32gt assembly instructions to VM operations.
///
/// This operation takes 6 cycles.
pub fn u32gt(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    // Reverse the numbers so we can get a gt check.
    span.push_op(Swap);

    compute_lt(span);

    Ok(None)
}

/// Translates u32gte assembly instructions to VM operations.
///
/// This operation takes 6 cycles.
pub fn u32gte(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    compute_lt(span);

    // Flip the final results to get the gte results.
    span.add_op(Not)
}

/// Translates u32min assembly instructions to VM operations.
///
/// Specifically, we subtract the top value from the second to the top value (U32SUB), check the
/// underflow flag (EQZ), and perform a conditional swap (CSWAP) to have the max number in front.
/// Then we finally drop the top element to keep the min.
///
/// This operation takes 8 cycles.
pub fn u32min(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    compute_max_and_min(span);

    // Drop the max and keep the min
    span.add_op(Drop)
}

/// Translates u32max assembly instructions to VM operations.
///
/// Specifically, we subtract the top value from the second to the top value (U32SUB), check the
/// underflow flag (EQZ), and perform a conditional swap (CSWAP) to have the max number in front.
/// Then we finally drop the 2nd element to keep the max.
///
/// This operation takes 9 cycles.
pub fn u32max(span: &mut SpanBuilder) -> Result<Option<CodeBlock>, AssemblyError> {
    compute_max_and_min(span);

    // Drop the min and keep the max
    span.add_ops([Swap, Drop])
}

// COMPARISON OPERATIONS - HELPERS
// ================================================================================================

/// Inserts the VM operations to check if the second element is less than
/// the top element. This takes 5 cycles.
fn compute_lt(span: &mut SpanBuilder) {
    span.push_ops([
        U32sub, Swap, Drop, // Perform the operations
        Eqz, Not, // Check the underflow flag
    ])
}

/// Duplicate the top two elements in the stack and determine the min and max between them.
///
/// The maximum number will be at the top of the stack and minimum will be at the 2nd index.
fn compute_max_and_min(span: &mut SpanBuilder) {
    // Copy top two elements of the stack.
    span.push_ops([Dup1, Dup1]);

    #[rustfmt::skip]
    span.push_ops([
        U32sub, Swap, Drop,

        // Check the underflow flag, if it's zero
        // then the second number is equal or larger than the first.
        Eqz, CSwap,
    ]);
}
