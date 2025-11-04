pub fn floor_division(dividend: i32, divisor: i32) -> i32 {
    assert!(divisor > 0, "Divisor must be positive.");
    if dividend % divisor == 0 || dividend.signum() == divisor.signum() {
        dividend / divisor
    } else {
        dividend / divisor - 1
    }
}

pub fn ceil_division_u128(dividend: u128, divisor: u128) -> u128 {
    assert!(divisor > 0, "Divisor must be positive.");
    let quotient = dividend / divisor;
    let prod = quotient * divisor;
    if prod == dividend {
        quotient
    } else {
        quotient + 1
    }
}

pub fn ceil_division_u32(dividend: u32, divisor: u32) -> u32 {
    assert!(divisor > 0, "Divisor must be positive.");
    let quotient = dividend / divisor;
    let prod = quotient * divisor;
    if prod == dividend {
        quotient
    } else {
        quotient + 1
    }
}
