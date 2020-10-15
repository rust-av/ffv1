pub fn min32(a: i32, b: i32) -> i32 {
    if a > b {
        b
    } else {
        a
    }
}

pub fn max32(a: i32, b: i32) -> i32 {
    if a < b {
        b
    } else {
        a
    }
}

pub fn abs32(n: i32) -> i32 {
    if n >= 0 {
        n
    } else {
        -n
    }
}
