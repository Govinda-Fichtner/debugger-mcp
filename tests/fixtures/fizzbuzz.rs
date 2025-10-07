// FizzBuzz implementation with a deliberate bug for testing
// Bug: Line 9 checks n % 4 instead of n % 5

fn fizzbuzz(n: i32) -> String {
    if n % 15 == 0 {
        "FizzBuzz".to_string()
    } else if n % 3 == 0 {
        "Fizz".to_string()
    } else if n % 4 == 0 {  // BUG: Should be n % 5
        "Buzz".to_string()
    } else {
        n.to_string()
    }
}

fn main() {
    for i in 1..=100 {
        println!("{}: {}", i, fizzbuzz(i));
    }
}
