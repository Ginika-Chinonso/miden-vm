use super::{ExecutionError, Felt, StarkField, TraceFragment};

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

/// Number of columns needed to record an execution trace of the bitwise helper.
const TRACE_WIDTH: usize = 11;

/// Initial capacity of each column.
const INIT_TRACE_CAPACITY: usize = 128;

// BITWISE HELPER
// ================================================================================================

/// Bitwise operation helper for the VM.
///
/// This component is responsible for computing AND, OR, and XOR bitwise operations on 32-bit
/// values, as well as for building an execution trace of these operations.
///
/// ## Execution trace
/// Execution trace for each operation consists of 8 rows and 11 columns. At the hight level,
/// we break input values into 4-bit limbs, apply the bitwise operation to these limbs at every
/// row starting with the most significant limb, and accumulate the result in the result column.
///
/// The layout of the table is illustrated below.
///
///    a     b      a0     a1     a2     a3     b0     b1     b2     b3     c
/// ├─────┴─────┴───────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴─────┤
///
/// In the above, the meaning of the columns is as follows:
/// - Columns `a` and `b` contain accumulated 4-bit limbs of input values. Specifically, at the
///   first row, the values of columns `a` and `b` are set to the most significant 4-bit limb
///   of each input value. With all subsequent rows, the next most significant limb is appended
///   to each column for the corresponding value. Thus, by the 8th row, columns `a` and `b`
///   contain full input values for the bitwise operation.
/// - Columns `a0` through `a3` and `b0` through `b3` contain bits of the least significant 4-bit
///   limb of the values in `a` and `b` columns respectively.
/// - Column `c` contains the accumulated result of applying the bitwise operation to 4-bit limbs.
///   At the first row, column `c` contains the result of bitwise operation applied to the most
///   significant 4-bit limbs of the input values. With every subsequent row, the next most
///   significant 4-bit limb of the result is appended to it. Thus, but the 8th row, column `c`
///   contains the full result of the bitwise operation.
pub struct Bitwise {
    trace: [Vec<Felt>; TRACE_WIDTH],
}

impl Bitwise {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [Bitwise] initialized with an empty trace.
    pub fn new() -> Self {
        let trace = (0..TRACE_WIDTH)
            .map(|_| Vec::with_capacity(INIT_TRACE_CAPACITY))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        Self { trace }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns length of execution trace required to describe bitwise operations executed on the
    /// VM.
    pub fn trace_len(&self) -> usize {
        self.trace[0].len()
    }

    // TRACE MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Computes a bitwise AND of `a` and `b` and returns the result. We assume that `a` and `b`
    /// are 32-bit values. If that's not the case, the result of the computation is undefined.
    ///
    /// This also adds 8 rows to the internal execution trace table required for computing the
    /// operation.
    pub fn u32and(&mut self, a: Felt, b: Felt) -> Result<Felt, ExecutionError> {
        let a = a.as_int();
        let b = b.as_int();
        let mut result = 0u64;

        // append 8 rows to the trace, each row computing bitwise AND in 4 bit limbs starting with
        // the most significant limb.
        for bit_offset in (0..32).step_by(4).rev() {
            // shift a and b so that the next 4-bit limb is in the least significant position
            let a = a >> bit_offset;
            let b = b >> bit_offset;

            // add a new row to the trace table and populate it with binary decomposition of the 4
            // least significant bits of a and b.
            self.add_trace_row(a, b);

            // compute bitwise AND of the 4 least significant bits of a and b
            let result_4_bit = (a & b) & 0xF;

            // append the 4 bit result to the result accumulator, and save the current result into
            // the 11th column of the trace.
            result = (result << 4) | result_4_bit;
            self.trace[10].push(Felt::new(result));
        }

        Ok(Felt::new(result))
    }

    /// Computes a bitwise OR of `a` and `b` and returns the result. We assume that `a` and `b`
    /// are 32-bit values. If that's not the case, the result of the computation is undefined.
    ///
    /// This also adds 8 rows to the internal execution trace table required for computing the
    /// operation.
    pub fn u32or(&mut self, a: Felt, b: Felt) -> Result<Felt, ExecutionError> {
        let a = a.as_int();
        let b = b.as_int();
        let mut result = 0u64;

        // append 8 rows to the trace, each row computing bitwise OR in 4 bit limbs starting with
        // the most significant limb.
        for bit_offset in (0..32).step_by(4).rev() {
            // shift a and b so that the next 4-bit limb is in the least significant position
            let a = a >> bit_offset;
            let b = b >> bit_offset;

            // add a new row to the trace table and populate it with binary decomposition of the 4
            // least significant bits of a and b.
            self.add_trace_row(a, b);

            // compute bitwise OR of the 4 least significant bits of a and b
            let result_4_bit = (a | b) & 0xF;

            // append the 4 bit result to the result accumulator, and save the current result into
            // the 11th column of the trace.
            result = (result << 4) | result_4_bit;
            self.trace[10].push(Felt::new(result));
        }

        Ok(Felt::new(result))
    }

    /// Computes a bitwise XOR of `a` and `b` and returns the result. We assume that `a` and `b`
    /// are 32-bit values. If that's not the case, the result of the computation is undefined.
    ///
    /// This also adds 8 rows to the internal execution trace table required for computing the
    /// operation.
    pub fn u32xor(&mut self, a: Felt, b: Felt) -> Result<Felt, ExecutionError> {
        let a = a.as_int();
        let b = b.as_int();
        let mut result = 0u64;

        // append 8 rows to the trace, each row computing bitwise XOR in 4 bit limbs starting with
        // the most significant limb.
        for bit_offset in (0..32).step_by(4).rev() {
            // shift a and b so that the next 4-bit limb is in the least significant position
            let a = a >> bit_offset;
            let b = b >> bit_offset;

            // add a new row to the trace table and populate it with binary decomposition of the 4
            // least significant bits of a and b.
            self.add_trace_row(a, b);

            // compute bitwise XOR of the 4 least significant bits of a and b
            let result_4_bit = (a ^ b) & 0xF;

            // append the 4 bit result to the result accumulator, and save the current result into
            // the 11th column of the trace.
            result = (result << 4) | result_4_bit;
            self.trace[10].push(Felt::new(result));
        }

        Ok(Felt::new(result))
    }

    // EXECUTION TRACE GENERATION
    // --------------------------------------------------------------------------------------------

    /// Fills the provide trace fragment with trace data from this bitwise helper instance.
    #[allow(dead_code)]
    pub fn fill_trace(self, trace: &mut TraceFragment) {
        // make sure fragment dimensions are consistent with the dimensions of this trace
        debug_assert_eq!(self.trace_len(), trace.len(), "inconsistent trace lengths");
        debug_assert_eq!(TRACE_WIDTH, trace.width(), "inconsistent trace widths");

        // copy trace into the fragment column-by-column
        // TODO: this can be parallelized to copy columns in multiple threads
        for (out_column, column) in trace.columns().zip(self.trace) {
            out_column.copy_from_slice(&column);
        }
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Appends a new row to the trace table and populates the first 10 column of trace as follows:
    /// - Column 0 is set to the current value of `a`.
    /// - Column 1 is set to the current value of `b`.
    /// - Columns 2 to 5 are set to the 4 least-significant bits of `a`.
    /// - Columns 6 to 9 are set to the 4 least-significant bits of `b`.
    fn add_trace_row(&mut self, a: u64, b: u64) {
        self.trace[0].push(Felt::new(a));
        self.trace[1].push(Felt::new(b));

        self.trace[2].push(Felt::new(a & 1));
        self.trace[3].push(Felt::new((a >> 1) & 1));
        self.trace[4].push(Felt::new((a >> 2) & 1));
        self.trace[5].push(Felt::new((a >> 3) & 1));

        self.trace[6].push(Felt::new(b & 1));
        self.trace[7].push(Felt::new((b >> 1) & 1));
        self.trace[8].push(Felt::new((b >> 2) & 1));
        self.trace[9].push(Felt::new((b >> 3) & 1));
    }
}
