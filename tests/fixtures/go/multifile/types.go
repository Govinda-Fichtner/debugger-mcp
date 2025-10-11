package main

// Calculator represents a simple calculator
type Calculator struct {
	Name    string
	Version string
}

// Multiply multiplies two numbers
func (c *Calculator) Multiply(a, b int) int {
	return a * b
}

// Divide divides two numbers
func (c *Calculator) Divide(a, b int) int {
	if b == 0 {
		return 0
	}
	return a / b
}
