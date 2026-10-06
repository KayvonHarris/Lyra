// Lyra v0.1 executable example: iterative Fibonacci and lexical scopes.
fn fibonacci(n: Int) -> Int {
    var previous = 0;
    var current = 1;
    var index = 0;
    while index < n {
        let next = previous + current;
        previous = current;
        current = next;
        index = index + 1;
    }
    return previous;
}

fn main() -> Int {
    let answer = fibonacci(10);
    if answer == 55 {
        let answer = fibonacci(7);
        if answer == 13 {
            return 42;
        } else {
            return 2;
        }
    } else {
        return 1;
    }
}
