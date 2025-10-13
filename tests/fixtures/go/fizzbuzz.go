package main

import "fmt"

// fizzbuzz returns FizzBuzz string for given number
func fizzbuzz(n int) string {
	if n%15 == 0 {
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
	fmt.Println("FizzBuzz from 1 to 100:")
	for i := 1; i <= 100; i++ {
		result := fizzbuzz(i)
		fmt.Println(result)
	}
	fmt.Println("Done!")
}
