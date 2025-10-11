package main

import "fmt"

func main() {
	fmt.Println("Multi-file Go application test")

	// Test calculator functions
	sum := Add(10, 20)
	fmt.Printf("10 + 20 = %d\n", sum)

	diff := Subtract(30, 15)
	fmt.Printf("30 - 15 = %d\n", diff)

	// Test utility functions
	doubled := Double(sum)
	fmt.Printf("Doubled: %d\n", doubled)

	// Test calculator struct
	calc := Calculator{Name: "TestCalc", Version: "1.0"}
	product := calc.Multiply(3, 4)
	fmt.Printf("%s says: 3 * 4 = %d\n", calc.Name, product)

	fmt.Println("All tests passed!")
}
