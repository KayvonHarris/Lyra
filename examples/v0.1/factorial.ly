// Lyra v0.1 executable example: factorial with a mutable loop.
fn factorial(n: Int) -> Int {
    var result = 1;
    var current = 2;
    while current <= n {
        result = result * current;
        current = current + 1;
    }
    return result;
}

fn main() -> Int {
    if factorial(5) == 120 {
        return 42;
    } else {
        return 1;
    }
}
