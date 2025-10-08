// FizzBuzz implementation with deliberate bug for debugging
// Bug: Line 9 uses n % 4 instead of n % 5 for "Buzz"

function fizzbuzz(n) {
    if (n % 15 === 0) {
        return "FizzBuzz";
    } else if (n % 3 === 0) {
        return "Fizz";
    } else if (n % 4 === 0) {  // BUG: Should be n % 5
        return "Buzz";
    } else {
        return n.toString();
    }
}

// Main execution
for (let i = 1; i <= 100; i++) {
    console.log(fizzbuzz(i));
}
