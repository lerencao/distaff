use std::ops::Range;
use crate::math::{ field };
use crate::utils::accumulator::{ add_constants, apply_sbox, apply_mds, apply_inv_sbox };

use super::super::opcodes2::{ FlowOps, UserOps };

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const BASE_CYCLE_LENGTH: usize = 16;
const PUSH_OP_ALIGNMENT: usize = 8;

const SPONGE_WIDTH: usize = 4;
const NUM_CF_OP_BITS: usize = 3;
const NUM_LD_OP_BITS: usize = 5;
const NUM_HD_OP_BITS: usize = 2;

const OP_ACC_RANGE      : Range<usize> = Range { start:  0, end:  4 };
const CF_OP_BITS_RANGE  : Range<usize> = Range { start:  4, end:  7 };
const LD_OP_BITS_RANGE  : Range<usize> = Range { start:  7, end: 12 };
const HD_OP_BITS_RANGE  : Range<usize> = Range { start: 12, end: 14 };

const MAX_CONTEXT_DEPTH: usize = 16;
const MAX_LOOP_DEPTH: usize = 8;

// TYPES AND INTERFACES
// ================================================================================================
pub struct Decoder {

    step        : usize,

    op_acc      : [Vec<u128>; SPONGE_WIDTH],
    sponge      : [u128; SPONGE_WIDTH],

    cf_op_bits  : [Vec<u128>; NUM_CF_OP_BITS],
    ld_op_bits  : [Vec<u128>; NUM_LD_OP_BITS],
    hd_op_bits  : [Vec<u128>; NUM_HD_OP_BITS],

    ctx_stack   : Vec<Vec<u128>>,
    ctx_depth   : usize,

    loop_stack  : Vec<Vec<u128>>,
    loop_depth  : usize,
}

// DECODER IMPLEMENTATION
// ================================================================================================
impl Decoder {

    /// Creates a new instance of instruction decoder.
    pub fn new(init_trace_length: usize) -> Decoder {

        // initialize instruction accumulator
        let op_acc = [
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length],
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length],
        ];
        let sponge = [field::ZERO; SPONGE_WIDTH];

        // initialize op_bits registers
        let cf_op_bits = [
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length],
            vec![field::ZERO; init_trace_length]
        ];
        let ld_op_bits = [
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length],
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length],
            vec![field::ZERO; init_trace_length]
        ];
        let hd_op_bits = [
            vec![field::ZERO; init_trace_length], vec![field::ZERO; init_trace_length]
        ];

        // initialize the stacks
        let ctx_stack = vec![vec![field::ZERO; init_trace_length]];
        let ctx_depth = ctx_stack.len();

        let loop_stack = Vec::new();
        let loop_depth = loop_stack.len();

        // create and return decoder
        return Decoder {
            step: 0, op_acc, sponge, cf_op_bits, ld_op_bits, hd_op_bits,
            ctx_stack, ctx_depth, loop_stack, loop_depth,
        };
    }

    /// Returns trace length of register traces in the decoder.
    pub fn trace_length(&self) -> usize {
        return self.op_acc[0].len();
    }

    /// Returns value of the current step pointer.
    pub fn current_step(&self) -> usize {
        return self.step;
    }

    /// Returns the max value of the context stack reached during program execution.
    pub fn max_ctx_stack_depth(&self) -> usize {
        return self.ctx_stack.len();
    }

    /// Returns the max value of the loop stack reached during program execution.
    pub fn max_loop_stack_depth(&self) -> usize {
        return self.loop_stack.len();
    }

    /// Returns the state of the stack at the specified `step`.
    pub fn get_state(&self, step: usize) -> Vec<u128> {
        let mut state = Vec::new();

        for register in self.op_acc.iter()     { state.push(register[step]); }
        for register in self.cf_op_bits.iter() { state.push(register[step]); }
        for register in self.ld_op_bits.iter() { state.push(register[step]); }
        for register in self.hd_op_bits.iter() { state.push(register[step]); }
        for register in self.ctx_stack.iter()  { state.push(register[step]); }
        for register in self.loop_stack.iter() { state.push(register[step]); }

        return state;
    }

    pub fn print_state(&self, step: usize) {
        let state = self.get_state(step);
        let ctx_stack_start = HD_OP_BITS_RANGE.end;
        let ctx_stack_end = ctx_stack_start + self.max_ctx_stack_depth();

        println!("{}:\t{:>32X?} {:?} {:?} {:?} {:X?} {:X?}", step,
            &state[OP_ACC_RANGE], &state[CF_OP_BITS_RANGE],
            &state[LD_OP_BITS_RANGE], &state[HD_OP_BITS_RANGE],
            &state[ctx_stack_start..ctx_stack_end], &state[ctx_stack_end..],
        );
    }

    // OPERATION DECODERS
    // --------------------------------------------------------------------------------------------

    /// Initiates a new program block (Group or Switch).
    pub fn start_block(&mut self) {
        assert!(self.step % BASE_CYCLE_LENGTH == BASE_CYCLE_LENGTH - 1,
            "cannot start context block at step {}: operation alignment is not valid", self.step);

        self.advance_step();
        self.save_context();
        self.copy_loop_stack();
        self.set_op_bits(FlowOps::Begin, UserOps::Noop);
        self.set_sponge([0, 0, 0, 0]);
    }

    /// Terminates a program block (Group, Switch, or Loop).
    pub fn end_block(&mut self, sibling_hash: u128, true_branch: bool) {
        assert!(self.step % BASE_CYCLE_LENGTH == 0,
            "cannot exit context block at step {}: operation alignment is not valid", self.step);

        self.advance_step();
        let context_hash = self.pop_context();
        self.copy_loop_stack();

        let block_hash = self.sponge[0];
        if true_branch {
            // we are closing true branch of execution
            self.set_op_bits(FlowOps::Tend, UserOps::Noop);
            self.set_sponge([context_hash, block_hash, sibling_hash, 0]);
        }
        else {
            // we are closing false branch of execution
            self.set_op_bits(FlowOps::Fend, UserOps::Noop);
            self.set_sponge([context_hash, sibling_hash, block_hash, 0]);
        }
    }

    /// Initiates a new Loop block
    pub fn start_loop(&mut self, loop_image: u128) {
        assert!(self.step % BASE_CYCLE_LENGTH == BASE_CYCLE_LENGTH - 1,
            "cannot start a loop at step {}: operation alignment is not valid", self.step);

        self.advance_step();
        self.save_context();
        self.save_loop_image(loop_image);
        self.set_op_bits(FlowOps::Loop, UserOps::Noop);
        self.set_sponge([0, 0, 0, 0]);
    }

    /// Prepares the decoder for the next iteration of a loop.
    pub fn wrap_loop(&mut self) {
        assert!(self.step % BASE_CYCLE_LENGTH == BASE_CYCLE_LENGTH - 1,
            "cannot wrap a loop at step {}: operation alignment is not valid", self.step);

        self.advance_step();
        self.copy_context_stack();
        assert!(self.sponge[0] == self.peek_loop_image(),
            "cannot wrap a loop at step {}: hash of the last iteration doesn't match loop image", self.step);
        self.set_op_bits(FlowOps::Wrap, UserOps::Noop);
        self.set_sponge([0, 0, 0, 0]);
    }

    /// Prepares the decoder for exiting a loop.
    pub fn break_loop(&mut self) {
        assert!(self.step % BASE_CYCLE_LENGTH == BASE_CYCLE_LENGTH - 1,
            "cannot break a loop at step {}: operation alignment is not valid", self.step);

        self.advance_step();
        self.copy_context_stack();
        assert!(self.sponge[0] == self.pop_loop_image(),
            "cannot break a loop at step {}: hash of the last iteration doesn't match loop image", self.step);
        self.set_op_bits(FlowOps::Break, UserOps::Noop);
        self.set_sponge(self.sponge);
    }

    /// Updates the decoder with the value of the specified operation.
    pub fn decode_op(&mut self, op_code: UserOps, op_value: u128) {
        
        // op_value can be provided only for a PUSH operation and only
        // at steps which are multiples of 8
        if op_value != field::ZERO {
            match op_code {
                UserOps::Push => assert!(self.step % PUSH_OP_ALIGNMENT == 0,
                        "invalid PUSH operation alignment at step {}", self.step),
                _ => panic!("invalid {:?} operation at step {}: op_value is non-zero", op_code, self.step),
            }
        }

        self.advance_step();
        self.copy_context_stack();
        self.copy_loop_stack();
        self.set_op_bits(FlowOps::Hacc, op_code);
        self.apply_hacc_round(op_code, op_value);        
    }

    /// Populate all register traces with values for steps between the current step
    /// and the end of the trace.
    pub fn finalize_trace(&mut self) {
        // set all bit registers to 1 to indicate NOOP operation
        for register in self.cf_op_bits.iter_mut() { fill_register(register, self.step, field::ONE); }
        for register in self.ld_op_bits.iter_mut() { fill_register(register, self.step, field::ONE); }
        for register in self.hd_op_bits.iter_mut() { fill_register(register, self.step, field::ONE); }

        // for op_acc and stack registers, just copy the value of the last state of the register
        for register in self.op_acc.iter_mut()     { fill_register(register, self.step + 1, register[self.step]); }
        for register in self.ctx_stack.iter_mut()  { fill_register(register, self.step + 1, register[self.step]); }
        for register in self.loop_stack.iter_mut() { fill_register(register, self.step + 1, register[self.step]); }
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Moves step pointer to the next step and ensures that register traces have sufficient size.
    fn advance_step(&mut self) {
        // increment step by 1
        self.step += 1;

        // make sure there is enough memory allocated for register traces
        if self.step >= self.trace_length() {
            let new_length = self.trace_length() * 2;

            for register in self.op_acc.iter_mut()     { register.resize(new_length, field::ZERO); }
            for register in self.cf_op_bits.iter_mut() { register.resize(new_length, field::ZERO); }
            for register in self.ld_op_bits.iter_mut() { register.resize(new_length, field::ZERO); }
            for register in self.hd_op_bits.iter_mut() { register.resize(new_length, field::ZERO); }
            for register in self.ctx_stack.iter_mut()  { register.resize(new_length, field::ZERO); }
            for register in self.loop_stack.iter_mut() { register.resize(new_length, field::ZERO); }
        }
    }
    
    /// Populates all bits registers based on the opcodes for control flow and user operations.
    fn set_op_bits(&mut self, flow_op: FlowOps, user_op: UserOps) {

        // op_bits are always populated for the previous step
        let step = self.step - 1;

        let flow_op = flow_op as u8;
        for i in 0..NUM_CF_OP_BITS {
            self.cf_op_bits[i][step] = ((flow_op >> i) & 1) as u128;
        }

        let user_op = user_op as u8;
        for i in 0..NUM_LD_OP_BITS {
            self.ld_op_bits[i][step] = ((user_op >> i) & 1) as u128;
        }

        for i in 0..NUM_HD_OP_BITS {
            self.hd_op_bits[i][step] = ((user_op >> (i + NUM_LD_OP_BITS)) & 1) as u128;
        }
    }

    // CONTEXT STACK HELPERS
    // --------------------------------------------------------------------------------------------

    /// Pushes hash of the current program block onto the context stack.
    fn save_context(&mut self) {
        // increment context depth and make sure it doesn't overflow the stack
        self.ctx_depth += 1;
        assert!(self.ctx_depth <= MAX_CONTEXT_DEPTH, "context stack overflow at step {}", self.step);

        // if the depth exceeds current number of registers allocated for the context stack,
        // add a new register trace to the stack
        if self.ctx_depth > self.ctx_stack.len() {
            self.ctx_stack.push(vec![field::ZERO; self.trace_length()]);
        }

        // shift all stack values by one item to the right
        for i in 1..self.ctx_stack.len() {
            self.ctx_stack[i][self.step] = self.ctx_stack[i - 1][self.step - 1];
        }

        // set the top of the stack to the hash of the current program block
        // which is located in the first register of the sponge
        self.ctx_stack[0][self.step] = self.sponge[0]
    }

    /// Removes the top value from the context stack and returns it.
    fn pop_context(&mut self) -> u128 {
        // make sure the stack is not empty
        assert!(self.ctx_depth > 0, "context stack underflow at step {}", self.step);

        // shift all stack values by one item to the left
        for i in 1..self.ctx_stack.len() {
            self.ctx_stack[i - 1][self.step] = self.ctx_stack[i][self.step - 1];
        }

        // update the stack depth and return the value that was at the top of the stack
        // before it was shifted to the left
        self.ctx_depth -= 1;
        return self.ctx_stack[0][self.step - 1];
    }

    /// Copies contents of the context stack from the previous to the current step.
    fn copy_context_stack(&mut self) {
        for i in 0..self.ctx_stack.len() {
            self.ctx_stack[i][self.step] = self.ctx_stack[i][self.step - 1];
        }
    }

    // LOOP STACK HELPERS
    // --------------------------------------------------------------------------------------------

    /// Pushes `loop_image` onto the loop stack.
    fn save_loop_image(&mut self, loop_image: u128) {
        // increment loop depth and make sure it doesn't overflow the stack
        self.loop_depth += 1;
        assert!(self.loop_depth <= MAX_LOOP_DEPTH, "loop stack overflow at step {}", self.step);

        // if the depth exceeds current number of registers allocated for the loop stack,
        // add a new register trace to the stack
        if self.loop_depth > self.loop_stack.len() {
            self.loop_stack.push(vec![field::ZERO; self.trace_length()]);
        }

        // shift all stack values by one to the right
        for i in 1..self.loop_stack.len() {
            self.loop_stack[i][self.step] = self.loop_stack[i - 1][self.step - 1];
        }

        // set the top of the stack to loop_image
        self.loop_stack[0][self.step] = loop_image;
    }

    /// Copies contents of the loop stack from the previous to the current step and returns
    /// the top value of the stack.
    fn peek_loop_image(&mut self) -> u128 {
        // make sure the stack is not empty
        assert!(self.loop_depth > 0, "loop stack underflow at step {}", self.step);

        // copy all values of the stack from the last step to the current step
        for i in 0..self.loop_stack.len() {
            self.loop_stack[i][self.step] = self.loop_stack[i][self.step - 1];
        }

        // return top value of the stack
        return self.loop_stack[0][self.step];
    }

    // Removes the top value from the loop stack and returns it.
    fn pop_loop_image(&mut self) -> u128 {
        // make sure the stack is not empty
        assert!(self.loop_depth > 0, "loop stack underflow at step {}", self.step);

        // shift all stack values by one item to the left
        for i in 1..self.loop_stack.len() {
            self.loop_stack[i - 1][self.step] = self.loop_stack[i][self.step - 1];
        }

        // update the stack depth and return the value that was at the top of the stack
        // before it was shifted to the left
        self.loop_depth -= 1;
        return self.loop_stack[0][self.step - 1];
    }

    /// Copies contents of the loop stack from the previous to the current step.
    fn copy_loop_stack(&mut self) {
        for i in 0..self.loop_stack.len() {
            self.loop_stack[i][self.step] = self.loop_stack[i][self.step - 1];
        }
    }

    // HASH ACCUMULATOR HELPERS
    // --------------------------------------------------------------------------------------------

    /// Sets the states of the sponge to the provided values and updates `op_acc` registers 
    /// at the current step.
    fn set_sponge(&mut self, state: [u128; SPONGE_WIDTH]) {
        self.sponge = state;
        self.op_acc[0][self.step] = state[0];
        self.op_acc[1][self.step] = state[1];
        self.op_acc[2][self.step] = state[2];
        self.op_acc[3][self.step] = state[3];
    }

    /// Applies a modified version of Rescue round to the sponge state and copies the result
    /// into `op_acc` registers.
    fn apply_hacc_round(&mut self, op_code: UserOps, op_value: u128) {

        let ark_idx = (self.step - 1) % BASE_CYCLE_LENGTH;

        // apply first half of Rescue round
        add_constants(&mut self.sponge, ark_idx, 0);
        apply_sbox(&mut self.sponge);
        apply_mds(&mut self.sponge);
    
        // inject value into the state
        self.sponge[0] = field::add(self.sponge[0], op_code as u128);
        self.sponge[1] = field::add(self.sponge[1], op_value);
    
        // apply second half of Rescue round
        add_constants(&mut self.sponge, ark_idx, SPONGE_WIDTH);
        apply_inv_sbox(&mut self.sponge);
        apply_mds(&mut self.sponge);

        // copy the new sponge state into the op_acc registers
        for i in 0..SPONGE_WIDTH {
            self.op_acc[i][self.step] = self.sponge[i];
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn fill_register(register: &mut Vec<u128>, from: usize, value: u128) {
    let to = register.len();
    register.resize(from, field::ZERO);
    register.resize(to, value);
}