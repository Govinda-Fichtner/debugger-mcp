#!/usr/bin/env python3
"""
FizzBuzz implementation for testing debugger_mcp.

This program is used as a test fixture to validate debugging functionality.
"""

def fizzbuzz(n):
    """
    Returns FizzBuzz output for number n.

    Rules:
    - If n is divisible by 3 and 5, return "FizzBuzz"
    - If n is divisible by 3, return "Fizz"
    - If n is divisible by 5, return "Buzz"
    - Otherwise, return str(n)
    """
    if n % 15 == 0:  # Breakpoint target: line 18
        return "FizzBuzz"
    elif n % 3 == 0:
        return "Fizz"
    elif n % 5 == 0:
        return "Buzz"
    else:
        return str(n)


def main():
    """Main function that runs FizzBuzz for numbers 1-100."""
    results = []
    for i in range(1, 101):  # Breakpoint target: line 32
        result = fizzbuzz(i)
        results.append(result)
        print(result)

    return results


if __name__ == "__main__":
    main()
