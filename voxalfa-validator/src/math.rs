pub fn gcd(a: usize, b: usize) -> usize {
    if b > 0 { gcd(b, a % b) } else { a }
}

pub fn lcm(a: usize, b: usize) -> usize {
    (a * b) / gcd(a, b)
}
