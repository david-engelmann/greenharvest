//! Exact rationals and their rendering.
//!
//! Every share and weighted catches count and value in the ranking
//! is an exact rational, formed and reduced the way the Python
//! `Fraction` forms and reduces them. The rendering follows the
//! Python rendering step for step, a division in a decimal context
//! of forty significant digits with halves rounding to even and
//! then a rounding to six places after the point with halves
//! rounding up, printed in fixed notation.

use std::cmp::Ordering;

use crate::bigint::BigInt;

/// An exact rational, numerator over a positive denominator,
/// reduced to lowest terms. Zero stands as zero over one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    num: BigInt,
    den: BigInt,
}

impl Rational {
    /// Form a rational from a numerator and a nonzero denominator,
    /// reducing to lowest terms and carrying the sign in the
    /// numerator, the denominator standing positive.
    pub fn new(num: BigInt, den: BigInt) -> Rational {
        assert!(!den.is_zero(), "a rational denominator is not zero");
        if num.is_zero() {
            return Rational {
                num: BigInt::zero(),
                den: BigInt::from_i64(1),
            };
        }
        let negative = num.is_negative() != den.is_negative();
        let gcd = num.gcd_abs(&den);
        let (num, _) = num.abs().divmod_abs(&gcd);
        let (den, _) = den.abs().divmod_abs(&gcd);
        let num = if negative { num.neg() } else { num };
        Rational { num, den }
    }

    pub fn from_int(value: BigInt) -> Rational {
        Rational {
            num: value,
            den: BigInt::from_i64(1),
        }
    }

    /// One divided by a positive count, the share of a defect.
    pub fn unit(count: u64) -> Rational {
        Rational {
            num: BigInt::from_i64(1),
            den: BigInt::from_u64(count),
        }
    }

    pub fn add(&self, other: &Rational) -> Rational {
        let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
        let den = self.den.mul(&other.den);
        Rational::new(num, den)
    }

    /// Divide by a positive integer, the value rule of the ranking.
    pub fn div_int(&self, divisor: &BigInt) -> Rational {
        Rational::new(self.num.clone(), self.den.mul(divisor))
    }

    pub fn numerator(&self) -> &BigInt {
        &self.num
    }

    pub fn denominator(&self) -> &BigInt {
        &self.den
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Rational) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Rational) -> Ordering {
        // Denominators stand positive, so the cross products order
        // the rationals.
        let left = self.num.mul(&other.den);
        let right = other.num.mul(&self.den);
        left.cmp(&right)
    }
}

/// Render an exact rational the way the Python renders one, at six
/// decimal places in fixed notation.
pub fn render_value(value: &Rational) -> String {
    let num = value.numerator();
    let den = value.denominator();
    if num.is_zero() {
        return "0.000000".to_string();
    }
    // Step one, the division at forty significant digits with
    // halves rounding to even. The quotient stands as an integer of
    // forty digits and a power of ten it is scaled by, the value
    // reading as the integer times ten to the minus scale.
    let digits_num = num.decimal_digits() as i64;
    let digits_den = den.decimal_digits() as i64;
    let mut scale: i64 = 39 - (digits_num - digits_den);
    let (mut quotient, mut remainder, mut divisor) = scaled_divmod(num, den, scale);
    while quotient.decimal_digits() > 40 {
        scale -= 1;
        let (q, r, d) = scaled_divmod(num, den, scale);
        quotient = q;
        remainder = r;
        divisor = d;
    }
    while quotient.decimal_digits() < 40 {
        scale += 1;
        let (q, r, d) = scaled_divmod(num, den, scale);
        quotient = q;
        remainder = r;
        divisor = d;
    }
    // Round the fortieth digit on the exact remainder.
    let doubled = remainder.mul(&BigInt::from_i64(2));
    match doubled.cmp(&divisor) {
        Ordering::Greater => {
            quotient = quotient.add(&BigInt::from_i64(1));
        }
        Ordering::Equal => {
            if quotient_is_odd(&quotient) {
                quotient = quotient.add(&BigInt::from_i64(1));
            }
        }
        Ordering::Less => {}
    }
    if quotient.decimal_digits() > 40 {
        // The rounding carried into a forty first digit, which is
        // a one followed by zeros, so the fortieth digit reading
        // stands one power of ten lower.
        quotient = BigInt::from_i64(1).mul_pow10(39);
        scale -= 1;
    }
    // Step two, the rounding to six places after the point with
    // halves rounding up, counted in units of a millionth.
    let units = if scale <= 6 {
        quotient.mul_pow10((6 - scale) as usize)
    } else {
        let drop = (scale - 6) as usize;
        let step = BigInt::from_i64(1).mul_pow10(drop);
        let (kept, dropped) = quotient.divmod_abs(&step);
        let doubled = dropped.mul(&BigInt::from_i64(2));
        if doubled.cmp(&step) != Ordering::Less {
            kept.add(&BigInt::from_i64(1))
        } else {
            kept
        }
    };
    let text = units.to_decimal_string();
    if text.len() <= 6 {
        format!("0.{}{}", "0".repeat(6 - text.len()), text)
    } else {
        let split = text.len() - 6;
        format!("{}.{}", &text[..split], &text[split..])
    }
}

/// Divide the numerator by the denominator with the numerator
/// scaled by ten to the given power, which may be negative, in
/// which case the denominator carries the scale instead. Returns
/// the quotient, the remainder, and the divisor it divided by.
fn scaled_divmod(num: &BigInt, den: &BigInt, scale: i64) -> (BigInt, BigInt, BigInt) {
    if scale >= 0 {
        let scaled = num.mul_pow10(scale as usize);
        let (q, r) = scaled.divmod_abs(den);
        (q, r, den.clone())
    } else {
        let scaled_den = den.mul_pow10((-scale) as usize);
        let (q, r) = num.divmod_abs(&scaled_den);
        (q, r, scaled_den)
    }
}

fn quotient_is_odd(quotient: &BigInt) -> bool {
    let (_, rem) = quotient.divmod_abs(&BigInt::from_i64(2));
    !rem.is_zero()
}
