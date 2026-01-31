//! Big integer utilities for Neve.
//! Neve 的大整数工具。

use num_bigint::BigInt;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};

/// Neve integer type (arbitrary precision).
/// Neve 整数类型（任意精度）。
pub type Int = BigInt;

/// Parse an integer string with the given radix.
/// 解析给定进制的整数字符串。
pub fn parse_int_radix(text: &str, radix: u32) -> Option<Int> {
    BigInt::parse_bytes(text.as_bytes(), radix)
}

/// Parse a base-10 integer string with optional sign.
/// 解析带可选符号的十进制整数字符串。
pub fn parse_int(text: &str) -> Option<Int> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('-') {
        parse_int_radix(rest, 10).map(|v| -v)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        parse_int_radix(rest, 10)
    } else {
        parse_int_radix(trimmed, 10)
    }
}

/// Convert an integer to i64 if possible.
/// 尝试将整数转换为 i64。
pub fn int_to_i64(value: &Int) -> Option<i64> {
    value.to_i64()
}

/// Convert an integer to usize if possible.
/// 尝试将整数转换为 usize。
pub fn int_to_usize(value: &Int) -> Option<usize> {
    value.to_usize()
}

/// Convert an integer to u32 if possible.
/// 尝试将整数转换为 u32。
pub fn int_to_u32(value: &Int) -> Option<u32> {
    value.to_u32()
}

/// Convert an integer to f64 if possible.
/// 尝试将整数转换为 f64。
pub fn int_to_f64(value: &Int) -> Option<f64> {
    value.to_f64()
}

/// Convert a float to an integer by truncating toward zero.
/// 将浮点数截断为整数（向零取整）。
pub fn int_from_f64(value: f64) -> Option<Int> {
    if value.is_finite() {
        Int::from_f64(value.trunc())
    } else {
        None
    }
}

/// Check if the integer is zero.
/// 检查整数是否为零。
pub fn int_is_zero(value: &Int) -> bool {
    value.is_zero()
}

/// Check if the integer is negative.
/// 检查整数是否为负数。
pub fn int_is_negative(value: &Int) -> bool {
    value.is_negative()
}

/// Get the absolute value of an integer.
/// 获取整数的绝对值。
pub fn int_abs(value: &Int) -> Int {
    if int_is_negative(value) {
        -value
    } else {
        value.clone()
    }
}
