//! Exact big integers, the substrate under the rationals.
//!
//! A sign and a magnitude of base one billion limbs, little endian,
//! normalised so the magnitude carries no leading zero limbs and
//! zero stands with an empty magnitude and a zero sign. The ranking
//! arithmetic of the Python runs on integers of unbounded size, with
//! harmonic sums standing past one hundred digits, so no machine
//! integer can carry it.

use std::cmp::Ordering;

const BASE: u64 = 1_000_000_000;

/// An exact integer of unbounded size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigInt {
    sign: i8,
    mag: Vec<u32>,
}

fn mag_normalize(mut mag: Vec<u32>) -> Vec<u32> {
    while let Some(&0) = mag.last() {
        mag.pop();
    }
    mag
}

fn mag_cmp(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn mag_add(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry: u64 = 0;
    for i in 0..a.len().max(b.len()) {
        let mut sum = carry;
        if i < a.len() {
            sum += u64::from(a[i]);
        }
        if i < b.len() {
            sum += u64::from(b[i]);
        }
        out.push((sum % BASE) as u32);
        carry = sum / BASE;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    mag_normalize(out)
}

/// Subtract b from a, for a not smaller than b.
fn mag_sub(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow: i64 = 0;
    for i in 0..a.len() {
        let mut diff = i64::from(a[i]) - borrow;
        if i < b.len() {
            diff -= i64::from(b[i]);
        }
        if diff < 0 {
            diff += BASE as i64;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(diff as u32);
    }
    mag_normalize(out)
}

fn mag_mul(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        if x == 0 {
            continue;
        }
        let mut carry: u64 = 0;
        for (j, &y) in b.iter().enumerate() {
            let total = out[i + j] + u64::from(x) * u64::from(y) + carry;
            out[i + j] = total % BASE;
            carry = total / BASE;
        }
        let mut k = i + b.len();
        while carry > 0 {
            let total = out[k] + carry;
            out[k] = total % BASE;
            carry = total / BASE;
            k += 1;
        }
    }
    mag_normalize(out.into_iter().map(|limb| limb as u32).collect())
}

fn mag_mul_small(a: &[u32], m: u32) -> Vec<u32> {
    if m == 0 || a.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(a.len() + 1);
    let mut carry: u64 = 0;
    for &x in a {
        let total = u64::from(x) * u64::from(m) + carry;
        out.push((total % BASE) as u32);
        carry = total / BASE;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    mag_normalize(out)
}

fn mag_divmod_small(a: &[u32], d: u32) -> (Vec<u32>, u32) {
    let mut out = vec![0u32; a.len()];
    let mut rem: u64 = 0;
    for i in (0..a.len()).rev() {
        let cur = rem * BASE + u64::from(a[i]);
        out[i] = (cur / u64::from(d)) as u32;
        rem = cur % u64::from(d);
    }
    (mag_normalize(out), rem as u32)
}

/// Divide magnitudes, returning the quotient and the remainder.
/// The divisor must not be zero. A multi limb divisor goes through
/// long division in the shape of Knuth's Algorithm D. Both operands
/// are normalised first by a one limb factor that pushes the
/// divisor's top limb to at least half the base, which makes each
/// quotient limb estimate exact to within one, and the rare over
/// estimate is repaired by the add back step.
fn mag_divmod(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if mag_cmp(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    if b.len() == 1 {
        let (q, r) = mag_divmod_small(a, b[0]);
        let rem = if r == 0 { Vec::new() } else { vec![r] };
        return (q, rem);
    }
    let n = b.len();
    let m = a.len() - n;
    // Normalise. The factor is at most half the base, so the
    // products gain at most one limb.
    let shift = (BASE / (u64::from(b[n - 1]) + 1)) as u32;
    let v = mag_mul_small(b, shift);
    debug_assert_eq!(v.len(), n);
    let mut u = mag_mul_small(a, shift);
    while u.len() < a.len() + 1 {
        u.push(0);
    }
    let mut q = vec![0u32; m + 1];
    for j in (0..=m).rev() {
        // Estimate the quotient limb from the top two limbs of the
        // running remainder over the divisor's top limb, then
        // correct it against the second limbs. The estimate never
        // undershoots and overshoots by at most two before this
        // correction and at most one after it.
        let top = u64::from(u[j + n]) * BASE + u64::from(u[j + n - 1]);
        let mut qhat = top / u64::from(v[n - 1]);
        let mut rhat = top % u64::from(v[n - 1]);
        loop {
            if qhat >= BASE {
                // A limb never stands at the base, so the estimate
                // is certainly too large and comes down.
                qhat -= 1;
                rhat += u64::from(v[n - 1]);
                continue;
            }
            if qhat * u64::from(v[n - 2]) > BASE * rhat + u64::from(u[j + n - 2]) {
                qhat -= 1;
                rhat += u64::from(v[n - 1]);
                if rhat < BASE {
                    continue;
                }
            }
            break;
        }
        // Multiply and subtract, u[j..=j+n] less qhat times v. The
        // borrow carries the high half of each product limb and the
        // borrow out of each subtraction together.
        let mut borrow: i64 = 0;
        for i in 0..n {
            let product = qhat * u64::from(v[i]);
            let mut t = i64::from(u[j + i]) - borrow - (product % BASE) as i64;
            borrow = (product / BASE) as i64;
            // The borrow in carries the high half of the previous
            // product as well as its own borrow out, so the
            // difference can stand almost two bases under zero and
            // the base is added back as many times as it takes.
            while t < 0 {
                t += BASE as i64;
                borrow += 1;
            }
            u[j + i] = t as u32;
        }
        let mut top = i64::from(u[j + n]) - borrow;
        if top < 0 {
            // The estimate stood one too large. Take it back and
            // add the divisor back into the window, the carry out
            // of the add back cancelling the borrowed limb.
            top += BASE as i64;
            qhat -= 1;
            let mut carry: u64 = 0;
            for i in 0..n {
                let sum = u64::from(u[j + i]) + u64::from(v[i]) + carry;
                if sum >= BASE {
                    u[j + i] = (sum - BASE) as u32;
                    carry = 1;
                } else {
                    u[j + i] = sum as u32;
                    carry = 0;
                }
            }
            top += carry as i64;
            if top >= BASE as i64 {
                top -= BASE as i64;
            }
        }
        u[j + n] = top as u32;
        q[j] = qhat as u32;
    }
    // The remainder stands scaled by the same factor, so divide it
    // back out. The division is exact.
    let (rem, rest) = mag_divmod_small(&u[..n], shift);
    debug_assert!(rest == 0 || shift == 1);
    (mag_normalize(q), rem)
}

fn mag_gcd(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut x = a.to_vec();
    let mut y = b.to_vec();
    while !y.is_empty() {
        let (_, r) = mag_divmod(&x, &y);
        x = y;
        y = r;
    }
    x
}

impl BigInt {
    pub fn zero() -> BigInt {
        BigInt {
            sign: 0,
            mag: Vec::new(),
        }
    }

    pub fn from_i64(value: i64) -> BigInt {
        if value == 0 {
            return BigInt::zero();
        }
        let sign = if value < 0 { -1 } else { 1 };
        let mut rest = value.unsigned_abs();
        let mut mag = Vec::new();
        while rest > 0 {
            mag.push((rest % BASE) as u32);
            rest /= BASE;
        }
        BigInt { sign, mag }
    }

    pub fn from_u64(value: u64) -> BigInt {
        if value == 0 {
            return BigInt::zero();
        }
        let mut rest = value;
        let mut mag = Vec::new();
        while rest > 0 {
            mag.push((rest % BASE) as u32);
            rest /= BASE;
        }
        BigInt { sign: 1, mag }
    }

    /// Read a decimal spelling, an optional minus sign and a run of
    /// ASCII digits, keeping every digit exactly.
    pub fn from_decimal(text: &str) -> Option<BigInt> {
        let (sign, digits) = match text.strip_prefix('-') {
            Some(rest) => (-1i8, rest),
            None => (1i8, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let bytes = digits.as_bytes();
        let mut mag = Vec::new();
        let mut end = bytes.len();
        while end > 0 {
            let start = end.saturating_sub(9);
            let chunk = std::str::from_utf8(&bytes[start..end]).ok()?;
            mag.push(chunk.parse::<u32>().ok()?);
            end = start;
        }
        let mag = mag_normalize(mag);
        if mag.is_empty() {
            return Some(BigInt::zero());
        }
        Some(BigInt { sign, mag })
    }

    pub fn is_zero(&self) -> bool {
        self.sign == 0
    }

    pub fn is_positive(&self) -> bool {
        self.sign > 0
    }

    pub fn is_negative(&self) -> bool {
        self.sign < 0
    }

    pub fn neg(&self) -> BigInt {
        BigInt {
            sign: -self.sign,
            mag: self.mag.clone(),
        }
    }

    pub fn abs(&self) -> BigInt {
        BigInt {
            sign: if self.sign == 0 { 0 } else { 1 },
            mag: self.mag.clone(),
        }
    }

    pub fn add(&self, other: &BigInt) -> BigInt {
        if self.sign == 0 {
            return other.clone();
        }
        if other.sign == 0 {
            return self.clone();
        }
        if self.sign == other.sign {
            BigInt {
                sign: self.sign,
                mag: mag_add(&self.mag, &other.mag),
            }
        } else {
            match mag_cmp(&self.mag, &other.mag) {
                Ordering::Equal => BigInt::zero(),
                Ordering::Greater => BigInt {
                    sign: self.sign,
                    mag: mag_sub(&self.mag, &other.mag),
                },
                Ordering::Less => BigInt {
                    sign: other.sign,
                    mag: mag_sub(&other.mag, &self.mag),
                },
            }
        }
    }

    pub fn sub(&self, other: &BigInt) -> BigInt {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &BigInt) -> BigInt {
        if self.sign == 0 || other.sign == 0 {
            return BigInt::zero();
        }
        BigInt {
            sign: self.sign * other.sign,
            mag: mag_mul(&self.mag, &other.mag),
        }
    }

    /// Multiply by a power of ten, for a nonnegative shift.
    pub fn mul_pow10(&self, shift: usize) -> BigInt {
        if self.sign == 0 || shift == 0 {
            return self.clone();
        }
        let limb_shift = shift / 9;
        let rem = (shift % 9) as u32;
        let mut mag = vec![0u32; limb_shift];
        if rem == 0 {
            mag.extend_from_slice(&self.mag);
        } else {
            let factor = 10u32.pow(rem);
            let scaled = mag_mul_small(&self.mag, factor);
            mag.extend_from_slice(&scaled);
        }
        BigInt {
            sign: self.sign,
            mag: mag_normalize(mag),
        }
    }

    /// Divide magnitudes for nonnegative operands, returning the
    /// quotient and the remainder. Both stand nonnegative.
    pub fn divmod_abs(&self, other: &BigInt) -> (BigInt, BigInt) {
        assert!(self.sign >= 0 && other.sign > 0);
        let (q, r) = mag_divmod(&self.mag, &other.mag);
        (BigInt::from_mag(q), BigInt::from_mag(r))
    }

    fn from_mag(mag: Vec<u32>) -> BigInt {
        let mag = mag_normalize(mag);
        if mag.is_empty() {
            BigInt::zero()
        } else {
            BigInt { sign: 1, mag }
        }
    }

    /// The greatest common divisor of the magnitudes.
    pub fn gcd_abs(&self, other: &BigInt) -> BigInt {
        if self.mag.is_empty() {
            return other.abs();
        }
        if other.mag.is_empty() {
            return self.abs();
        }
        BigInt::from_mag(mag_gcd(&self.mag, &other.mag))
    }

    /// The count of decimal digits in the magnitude.
    pub fn decimal_digits(&self) -> usize {
        match self.mag.last() {
            None => 1,
            Some(&top) => (self.mag.len() - 1) * 9 + top.to_string().len(),
        }
    }

    /// The value as an i64 when it fits.
    pub fn to_i64(&self) -> Option<i64> {
        let text = self.to_decimal_string();
        text.parse::<i64>().ok()
    }

    pub fn to_decimal_string(&self) -> String {
        if self.sign == 0 {
            return "0".to_string();
        }
        let mut out = String::new();
        if self.sign < 0 {
            out.push('-');
        }
        let top = self.mag.len() - 1;
        out.push_str(&self.mag[top].to_string());
        for i in (0..top).rev() {
            out.push_str(&format!("{:09}", self.mag[i]));
        }
        out
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &BigInt) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &BigInt) -> Ordering {
        if self.sign != other.sign {
            return self.sign.cmp(&other.sign);
        }
        match self.sign {
            1 => mag_cmp(&self.mag, &other.mag),
            -1 => mag_cmp(&other.mag, &self.mag),
            _ => Ordering::Equal,
        }
    }
}

impl std::fmt::Display for BigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_decimal_string())
    }
}
