use crate::math::{ FiniteField };
use super::utils::{ agg_op_constraint, is_binary, are_equal, enforce_no_change };

// CONSTRAINT EVALUATORS
// ================================================================================================

pub fn enforce_choose(evaluations: &mut [u128], current: &[u128], next: &[u128], op_flag: u128) -> u128 {
    
    let condition1 = current[2];
    let condition2 = u128::sub(u128::ONE, condition1);
    let op_result = u128::add(u128::mul(condition1, current[0]), u128::mul(condition2, current[1]));
    evaluations[0] = agg_op_constraint(evaluations[0], op_flag, are_equal(next[0], op_result));

    let n = next.len() - 2;
    enforce_no_change(&mut evaluations[1..n], &current[3..], &next[1..n], op_flag);
    
    return u128::mul(op_flag, is_binary(condition1));
}

pub fn enforce_choose2(evaluations: &mut [u128], current: &[u128], next: &[u128], op_flag: u128) -> u128 {

    let condition1 = current[4];
    let condition2 = u128::sub(u128::ONE, condition1);
    let op_result1 = u128::add(u128::mul(condition1, current[0]), u128::mul(condition2, current[2]));
    let op_result2 = u128::add(u128::mul(condition1, current[1]), u128::mul(condition2, current[3]));
    evaluations[0] = agg_op_constraint(evaluations[0], op_flag, are_equal(next[0], op_result1));
    evaluations[1] = agg_op_constraint(evaluations[1], op_flag, are_equal(next[1], op_result2));

    let n = next.len() - 4;
    enforce_no_change(&mut evaluations[2..n], &current[6..], &next[2..n], op_flag);

    return u128::mul(op_flag, is_binary(condition1));
}