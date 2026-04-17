package main

import "fmt"

func main() {
	// Variable declarations
	x := 10
	y := 20
	isActive := true

	// If statement
	if x < y {
		fmt.Println("x is less than y")
	} else {
		fmt.Println("x is greater than or equal to y")
	}

	// For loop
	for i := 0; i < 5; i++ {
		fmt.Println(i)
	}

	// Function call
	result := add(x, y)
	fmt.Println("Sum:", result)

	// Boolean expression
	if isActive && result > 0 {
		fmt.Println("Active and positive")
	}

	// Range loop
	numbers := []int{1, 2, 3, 4, 5}
	for _, num := range numbers {
		if num%2 == 0 {
			fmt.Println("Even:", num)
		} else {
			fmt.Println("Odd:", num)
		}
	}
}

func add(a, b int) int {
	return a + b
}

func multiply(a, b int) int {
	result := a * b
	return result
}
