//! Purpose
//! - Deterministic math substrate for Maer'Ken simulation.
//! - Provides f64 helpers and precision limits for world-law compliance.
//!
//! Invariants
//! - All continuous physical quantities use f64.
//! - Enforces roadmap § 1.3 residual limits.
//! - Deterministic (replay-stable across identical build/target).
//!
//! Failure Modes
//! - Non-deterministic float operations → replay divergence.
//! - NaN/Inf propagation → simulation halt.
//! - Precision limit breach → audit failure.
//!
//! Debug Notes
//! - Use libm or equivalent for complex math if platform drift is detected.
//! - Audit logs will catch cumulative float drift.

/// Floating-point residual limits for determinism (Roadmap § 1.3)
pub const TICK_RESIDUAL_LIMIT: f64 = 1e-12;
pub const CUMULATIVE_RESIDUAL_LIMIT: f64 = 1e-10;

/// PI constant for world-layer calculations
pub const PI: f64 = std::f64::consts::PI;

/// Q32.32 fixed-point representation of 1.0 (2^32)
/// Used for conversions between f64 and Q32.32 format
pub const Q32_32_ONE: i64 = 1 << 32;

/// Math precision helpers
pub trait FloatExt {
    // `self` by value is intentional: `f64` is `Copy`, so this costs
    // nothing and reads more naturally at call sites (`x.is_approx_zero(tol)`)
    // than threading a reference through for a primitive.
    #[allow(clippy::wrong_self_convention)]
    fn is_approx_zero(self, tol: f64) -> bool;
}

impl FloatExt for f64 {
    fn is_approx_zero(self, tol: f64) -> bool {
        self.abs() < tol
    }
}

/// Multiply two Q32.32 fixed-point numbers
///
/// Performs: (a * b) / 2^32
/// Uses 128-bit intermediate to avoid overflow.
///
/// # Arguments
///
/// * `a` - First Q32.32 number
/// * `b` - Second Q32.32 number
///
/// # Returns
///
/// Product in Q32.32 format
///
/// # Examples
///
/// ```
/// use mk_core::math::q32_32_mul;
/// use mk_core::math::Q32_32_ONE;
///
/// // 2.0 * 0.5 = 1.0
/// let two = Q32_32_ONE * 2;
/// let half = Q32_32_ONE / 2;
/// let result = q32_32_mul(two, half);
/// assert_eq!(result, Q32_32_ONE);
/// ```
pub fn q32_32_mul(a: i64, b: i64) -> i64 {
    // Use 128-bit to avoid overflow
    let product = (a as i128) * (b as i128);
    (product >> 32) as i64
}

/// Divide two Q32.32 fixed-point numbers
///
/// Performs: (a * 2^32) / b
/// Uses 128-bit intermediate to avoid overflow.
///
/// # Arguments
///
/// * `a` - Numerator in Q32.32 format
/// * `b` - Denominator in Q32.32 format (must not be zero)
///
/// # Returns
///
/// Quotient in Q32.32 format
///
/// # Panics
///
/// Panics if b = 0
///
/// # Examples
///
/// ```
/// use mk_core::math::q32_32_div;
/// use mk_core::math::Q32_32_ONE;
///
/// // 1.0 / 0.5 = 2.0
/// let one = Q32_32_ONE;
/// let half = Q32_32_ONE / 2;
/// let result = q32_32_div(one, half);
/// assert_eq!(result, Q32_32_ONE * 2);
/// ```
pub fn q32_32_div(a: i64, b: i64) -> i64 {
    assert_ne!(b, 0, "Division by zero in Q32.32 arithmetic");

    // Use 128-bit to avoid overflow
    let scaled = (a as i128) << 32;
    (scaled / b as i128) as i64
}

/// Add two Q32.32 fixed-point numbers
///
/// Simple integer addition since both are in same format.
///
/// # Arguments
///
/// * `a` - First Q32.32 number
/// * `b` - Second Q32.32 number
///
/// # Returns
///
/// Sum in Q32.32 format
pub fn q32_32_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Subtract two Q32.32 fixed-point numbers
///
/// Simple integer subtraction since both are in same format.
///
/// # Arguments
///
/// * `a` - Minuend in Q32.32 format
/// * `b` - Subtrahend in Q32.32 format
///
/// # Returns
///
/// Difference in Q32.32 format
pub fn q32_32_sub(a: i64, b: i64) -> i64 {
    a - b
}

/// Convert f64 to Q32.32 fixed-point (initialization ONLY)
///
/// # Warning
///
/// NEVER use for computation - initialization ONLY!
///
/// # Arguments
///
/// * `x` - f64 value to convert
///
/// # Returns
///
/// Q32.32 representation
///
/// # Examples
///
/// ```
/// use mk_core::math::{q32_32_from_f64, Q32_32_ONE};
///
/// let pi_q32 = q32_32_from_f64(3.14159);
/// let one_q32 = q32_32_from_f64(1.0);
/// assert_eq!(one_q32, Q32_32_ONE);
/// ```
pub fn q32_32_from_f64(x: f64) -> i64 {
    (x * Q32_32_ONE as f64) as i64
}

/// Convert Q32.32 fixed-point to f64 (display/logging ONLY)
///
/// # Warning
///
/// NEVER use for computation - display/logging ONLY!
///
/// # Arguments
///
/// * `x` - Q32.32 value to convert
///
/// # Returns
///
/// f64 representation
///
/// # Examples
///
/// ```
/// use mk_core::math::{q32_32_to_f64, Q32_32_ONE};
///
/// let one_f64 = q32_32_to_f64(Q32_32_ONE);
/// assert_eq!(one_f64, 1.0);
/// ```
pub fn q32_32_to_f64(x: i64) -> f64 {
    x as f64 / Q32_32_ONE as f64
}

/// Square root of Q32.32 fixed-point number
///
/// Exact: returns the floor of the true root, to Q32.32 precision.
///
/// # Arguments
///
/// * `x` - Q32.32 number (must be non-negative)
///
/// # Returns
///
/// Square root in Q32.32 format
///
/// # Panics
///
/// Panics if x is negative
pub fn q32_32_sqrt(x: i64) -> i64 {
    assert!(x >= 0, "Square root of negative number");
    // x encodes x / 2^32, so its root r must satisfy r / 2^32 = sqrt(x / 2^32),
    // i.e. r = sqrt(x * 2^32). The exact integer square root of that (floor)
    // is the correctly rounded-down Q32.32 result. x < 2^63 means
    // x * 2^32 < 2^95, whose root is below 2^48, so it fits in i64.
    isqrt_u128((x as u128) << 32) as i64
}

/// Floor of the square root of `n`, exact for every `u128`.
///
/// Newton's iteration on integers, started above the root, decreases
/// monotonically to `floor(sqrt(n))` and stops when it no longer shrinks.
fn isqrt_u128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // 2^ceil(bits/2) is always >= sqrt(n).
    let bits = 128 - n.leading_zeros();
    let mut x = 1u128 << bits.div_ceil(2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Power of Q32.32 fixed-point number (integer exponent)
///
/// Computes x^n using repeated multiplication.
///
/// # Arguments
///
/// * `x` - Q32.32 base
/// * `n` - Integer exponent (must be >= 0)
///
/// # Returns
///
/// x^n in Q32.32 format
///
/// # Panics
///
/// Panics if n is negative
pub fn q32_32_pow_int(x: i64, n: u32) -> i64 {
    if n == 0 {
        return Q32_32_ONE;
    }

    let mut result = Q32_32_ONE;
    let mut base = x;
    let mut exp = n;

    while exp > 0 {
        if exp % 2 == 1 {
            result = q32_32_mul(result, base);
        }
        base = q32_32_mul(base, base);
        exp /= 2;
    }

    result
}

/// Minimum of two Q32.32 numbers
///
/// # Arguments
///
/// * `a` - First Q32.32 number
/// * `b` - Second Q32.32 number
///
/// # Returns
///
/// Smaller of the two numbers
pub fn q32_32_min(a: i64, b: i64) -> i64 {
    if a < b {
        a
    } else {
        b
    }
}

/// Maximum of two Q32.32 numbers
///
/// # Arguments
///
/// * `a` - First Q32.32 number
/// * `b` - Second Q32.32 number
///
/// # Returns
///
/// Larger of the two numbers
pub fn q32_32_max(a: i64, b: i64) -> i64 {
    if a > b {
        a
    } else {
        b
    }
}

/// Absolute value of Q32.32 number
///
/// # Arguments
///
/// * `x` - Q32.32 number
///
/// # Returns
///
/// Absolute value in Q32.32 format
pub fn q32_32_abs(x: i64) -> i64 {
    if x < 0 {
        -x
    } else {
        x
    }
}

/// Check if Q32.32 number is zero
///
/// # Arguments
///
/// * `x` - Q32.32 number
///
/// # Returns
///
/// True if x is zero, false otherwise
pub fn q32_32_is_zero(x: i64) -> bool {
    x == 0
}

/// Check if Q32.32 number is approximately zero
///
/// Uses small epsilon for floating-point comparison.
///
/// # Arguments
///
/// * `x` - Q32.32 number
/// * `epsilon` - Q32.32 tolerance
///
/// # Returns
///
/// True if |x| < epsilon, false otherwise
pub fn q32_32_is_approx_zero(x: i64, epsilon: i64) -> bool {
    q32_32_abs(x) < epsilon
}

/// Clamp Q32.32 number to range
///
/// # Arguments
///
/// * `x` - Q32.32 number to clamp
/// * `min` - Minimum value
/// * `max` - Maximum value
///
/// # Returns
///
/// x clamped to [min, max] range
pub fn q32_32_clamp(x: i64, min: i64, max: i64) -> i64 {
    q32_32_max(min, q32_32_min(x, max))
}

/// Linear interpolation between two Q32.32 numbers
///
/// Computes: a + t * (b - a) where t is in [0, 1]
///
/// # Arguments
///
/// * `a` - Start value
/// * `b` - End value
/// * `t` - Interpolation parameter in Q32.32 format [0, Q32_32_ONE]
///
/// # Returns
///
/// Interpolated value in Q32.32 format
pub fn q32_32_lerp(a: i64, b: i64, t: i64) -> i64 {
    let diff = q32_32_sub(b, a);
    let scaled = q32_32_mul(diff, t);
    q32_32_add(a, scaled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q32_32_constants() {
        assert_eq!(Q32_32_ONE, 1i64 << 32);
        assert_eq!(Q32_32_ONE, 4294967296);
    }

    #[test]
    fn test_q32_32_from_f64() {
        assert_eq!(q32_32_from_f64(0.0), 0);
        assert_eq!(q32_32_from_f64(1.0), Q32_32_ONE);
        assert_eq!(q32_32_from_f64(2.0), Q32_32_ONE * 2);
        assert_eq!(q32_32_from_f64(0.5), Q32_32_ONE / 2);

        // Test with pi - use relaxed tolerance due to f64 precision limits
        let pi_q32 = q32_32_from_f64(std::f64::consts::PI);
        let pi_back = q32_32_to_f64(pi_q32);
        assert!((pi_back - std::f64::consts::PI).abs() < 1e-8);
    }

    #[test]
    fn test_q32_32_to_f64() {
        assert_eq!(q32_32_to_f64(0), 0.0);
        assert_eq!(q32_32_to_f64(Q32_32_ONE), 1.0);
        assert_eq!(q32_32_to_f64(Q32_32_ONE * 2), 2.0);
        assert_eq!(q32_32_to_f64(Q32_32_ONE / 2), 0.5);
    }

    #[test]
    fn test_q32_32_add() {
        assert_eq!(q32_32_add(Q32_32_ONE, Q32_32_ONE), Q32_32_ONE * 2);
        assert_eq!(q32_32_add(Q32_32_ONE * 3, Q32_32_ONE * 2), Q32_32_ONE * 5);
        assert_eq!(q32_32_add(0, Q32_32_ONE), Q32_32_ONE);
    }

    #[test]
    fn test_q32_32_sub() {
        assert_eq!(q32_32_sub(Q32_32_ONE * 5, Q32_32_ONE * 2), Q32_32_ONE * 3);
        assert_eq!(q32_32_sub(Q32_32_ONE, Q32_32_ONE), 0);
        assert_eq!(q32_32_sub(Q32_32_ONE * 2, Q32_32_ONE * 3), -Q32_32_ONE);
    }

    #[test]
    fn test_q32_32_mul() {
        // 2.0 * 3.0 = 6.0
        let two = Q32_32_ONE * 2;
        let three = Q32_32_ONE * 3;
        let result = q32_32_mul(two, three);
        assert_eq!(result, Q32_32_ONE * 6);

        // 0.5 * 0.5 = 0.25
        let half = Q32_32_ONE / 2;
        let result = q32_32_mul(half, half);
        assert_eq!(result, Q32_32_ONE / 4);

        // 1.5 * 2.0 = 3.0
        let one_and_half = Q32_32_ONE + Q32_32_ONE / 2;
        let result = q32_32_mul(one_and_half, Q32_32_ONE * 2);
        assert_eq!(result, Q32_32_ONE * 3);
    }

    #[test]
    fn test_q32_32_div() {
        // 6.0 / 2.0 = 3.0
        let six = Q32_32_ONE * 6;
        let two = Q32_32_ONE * 2;
        let result = q32_32_div(six, two);
        assert_eq!(result, Q32_32_ONE * 3);

        // 1.0 / 0.5 = 2.0
        let half = Q32_32_ONE / 2;
        let result = q32_32_div(Q32_32_ONE, half);
        assert_eq!(result, Q32_32_ONE * 2);

        // 0.25 / 0.5 = 0.5
        let quarter = Q32_32_ONE / 4;
        let result = q32_32_div(quarter, half);
        assert_eq!(result, Q32_32_ONE / 2);
    }

    #[test]
    #[should_panic(expected = "Division by zero")]
    fn q32_32_div_by_zero() {
        q32_32_div(Q32_32_ONE, 0);
    }

    #[test]
    fn test_q32_32_sqrt() {
        // sqrt(4.0) = 2.0
        let four = Q32_32_ONE * 4;
        let result = q32_32_sqrt(four);
        assert!((result as f64 / Q32_32_ONE as f64 - 2.0).abs() < 1e-6);

        // sqrt(1.0) = 1.0
        let result = q32_32_sqrt(Q32_32_ONE);
        assert!((result as f64 / Q32_32_ONE as f64 - 1.0).abs() < 1e-6);

        // sqrt(0.0) = 0.0
        let result = q32_32_sqrt(0);
        assert_eq!(result, 0);

        // sqrt(9.0) = 3.0
        let nine = Q32_32_ONE * 9;
        let result = q32_32_sqrt(nine);
        assert!((result as f64 / Q32_32_ONE as f64 - 3.0).abs() < 1e-6);
    }

    #[test]
    fn q32_32_sqrt_is_exact_across_magnitudes() {
        for x in [1e-6, 0.25, 2.0, 1e6, 1e8, 2.0e9] {
            let fixed = q32_32_from_f64(x);
            let root = q32_32_sqrt(fixed);
            // Exact floor: root^2 <= x * 2^32 < (root + 1)^2.
            let scaled = (fixed as u128) << 32;
            let r = root as u128;
            assert!(r * r <= scaled && (r + 1) * (r + 1) > scaled, "sqrt({x})");
            // And it agrees with f64 to Q32.32 resolution.
            let expected = f64::sqrt(fixed as f64 / Q32_32_ONE as f64);
            let got = root as f64 / Q32_32_ONE as f64;
            assert!((got - expected).abs() <= 2.0 / Q32_32_ONE as f64 + 1e-15 * expected);
        }
        assert_eq!(isqrt_u128(u128::MAX), u64::MAX as u128);
        for n in 0u128..2000 {
            let r = isqrt_u128(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n);
        }
        // Largest representable input does not overflow.
        let _ = q32_32_sqrt(i64::MAX);
    }

    #[test]
    #[should_panic(expected = "Square root of negative number")]
    fn q32_32_sqrt_negative() {
        q32_32_sqrt(-Q32_32_ONE);
    }

    #[test]
    fn test_q32_32_pow_int() {
        // 2^3 = 8
        let two = Q32_32_ONE * 2;
        let result = q32_32_pow_int(two, 3);
        assert_eq!(result, Q32_32_ONE * 8);

        // 3^2 = 9
        let three = Q32_32_ONE * 3;
        let result = q32_32_pow_int(three, 2);
        assert_eq!(result, Q32_32_ONE * 9);

        // 5^0 = 1
        let five = Q32_32_ONE * 5;
        let result = q32_32_pow_int(five, 0);
        assert_eq!(result, Q32_32_ONE);

        // 1^5 = 1
        let result = q32_32_pow_int(Q32_32_ONE, 5);
        assert_eq!(result, Q32_32_ONE);
    }

    #[test]
    fn test_q32_32_min_max() {
        let one = Q32_32_ONE;
        let two = Q32_32_ONE * 2;

        assert_eq!(q32_32_min(one, two), one);
        assert_eq!(q32_32_max(one, two), two);
        assert_eq!(q32_32_min(two, one), one);
        assert_eq!(q32_32_max(two, one), two);
    }

    #[test]
    fn test_q32_32_abs() {
        assert_eq!(q32_32_abs(Q32_32_ONE), Q32_32_ONE);
        assert_eq!(q32_32_abs(-Q32_32_ONE), Q32_32_ONE);
        assert_eq!(q32_32_abs(Q32_32_ONE * 5), Q32_32_ONE * 5);
        assert_eq!(q32_32_abs(-Q32_32_ONE * 5), Q32_32_ONE * 5);
        assert_eq!(q32_32_abs(0), 0);
    }

    #[test]
    fn test_q32_32_is_zero() {
        assert!(q32_32_is_zero(0));
        assert!(!q32_32_is_zero(Q32_32_ONE));
        assert!(!q32_32_is_zero(-Q32_32_ONE));
    }

    #[test]
    fn test_q32_32_is_approx_zero() {
        let epsilon = Q32_32_ONE / 1000; // 0.001

        assert!(q32_32_is_approx_zero(0, epsilon));
        assert!(q32_32_is_approx_zero(epsilon / 2, epsilon));
        assert!(!q32_32_is_approx_zero(epsilon * 2, epsilon));
        assert!(q32_32_is_approx_zero(-epsilon / 2, epsilon));
        assert!(!q32_32_is_approx_zero(-epsilon * 2, epsilon));
    }

    #[test]
    fn test_q32_32_clamp() {
        let one = Q32_32_ONE;
        let two = Q32_32_ONE * 2;
        let three = Q32_32_ONE * 3;

        assert_eq!(q32_32_clamp(one, two, three), two); // below range
        assert_eq!(q32_32_clamp(two, one, three), two); // in range
        assert_eq!(q32_32_clamp(three, one, two), two); // above range
    }

    #[test]
    fn test_q32_32_lerp() {
        let one = Q32_32_ONE;
        let three = Q32_32_ONE * 3;

        // t = 0.0 -> return a
        let result = q32_32_lerp(one, three, 0);
        assert_eq!(result, one);

        // t = 1.0 -> return b
        let result = q32_32_lerp(one, three, Q32_32_ONE);
        assert_eq!(result, three);

        // t = 0.5 -> midpoint
        let half = Q32_32_ONE / 2;
        let result = q32_32_lerp(one, three, half);
        assert_eq!(result, Q32_32_ONE * 2);

        // t = 0.25 -> quarter point
        let quarter = Q32_32_ONE / 4;
        let result = q32_32_lerp(one, three, quarter);
        let expected = one + (three - one) / 4;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_q32_32_complex_calculation() {
        // Test: (2.5 * 3.2) / 1.6 + sqrt(4.0)
        let two_point_five = q32_32_from_f64(2.5);
        let three_point_two = q32_32_from_f64(3.2);
        let one_point_six = q32_32_from_f64(1.6);
        let four = Q32_32_ONE * 4;

        let product = q32_32_mul(two_point_five, three_point_two);
        let quotient = q32_32_div(product, one_point_six);
        let sqrt_four = q32_32_sqrt(four);
        let result = q32_32_add(quotient, sqrt_four);

        // Expected: (2.5 * 3.2) / 1.6 + 2.0 = 8.0 / 1.6 + 2.0 = 5.0 + 2.0 = 7.0
        assert!((result as f64 / Q32_32_ONE as f64 - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_q32_32_precision() {
        // Test that we maintain precision for small numbers
        let small = q32_32_from_f64(0.001);
        let result = q32_32_mul(small, Q32_32_ONE * 1000);

        // Should be approximately 1.0
        assert!((result as f64 / Q32_32_ONE as f64 - 1.0).abs() < 1e-6);
    }
}
