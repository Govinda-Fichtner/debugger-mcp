// Example demonstrating library usage
use cargo_example::{add, multiply};

fn main() {
    let sum = add(5, 3);
    let product = multiply(5, 3);

    println!("5 + 3 = {}", sum);
    println!("5 * 3 = {}", product);
}
