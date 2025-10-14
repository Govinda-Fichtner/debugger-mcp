package main

import "fmt"

// fizzbuzz returns FizzBuzz output for number n.
//
// Rules:
// - If n is divisible by 3 and 5, return "FizzBuzz"
// - If n is divisible by 3, return "Fizz"
// - If n is divisible by 5, return "Buzz"
// - Otherwise, return string representation of n
func fizzbuzz(n int) string {
	if n%15 == 0 { // Breakpoint target: line 13
		return "FizzBuzz"
	} else if n%3 == 0 {
		return "Fizz"
	} else if n%5 == 0 {
		return "Buzz"
	} else {
		return fmt.Sprintf("%d", n)
	}
}

func main() {
	// Main function that runs FizzBuzz for numbers 1-100
	var results []string
	for i := 1; i <= 100; i++ { // Breakpoint target: line 27
		result := fizzbuzz(i)
		results = append(results, result)
		fmt.Println(result)
	}
}
