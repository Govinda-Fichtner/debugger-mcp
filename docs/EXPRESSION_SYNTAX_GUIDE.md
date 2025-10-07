# Language-Specific Expression Syntax Guide

**Last Updated**: 2025-10-07

## Overview

When using `debugger_evaluate` to inspect variables and evaluate expressions, syntax varies by programming language. This guide provides practical examples for Python, Ruby, and Node.js debugging.

## Quick Reference

| Language | Variable | Property/Attribute | Array/List Index | Method Call | String Interpolation |
|----------|----------|-------------------|------------------|-------------|---------------------|
| **Python** | `n` | `obj.attr` | `arr[0]` | `len(arr)` | `f"{n}"` |
| **Ruby** | `n` | `obj.attr` | `arr[0]` | `arr.length` | `"#{n}"` |
| **Node.js** | `n` | `obj.prop` | `arr[0]` | `arr.length` | `` `${n}` `` |

## Python (debugpy)

### Variables

```python
# Simple variable
debugger_evaluate({ expression: "n" })
# Result: "15"

# Local variable
debugger_evaluate({ expression: "result" })
# Result: "'Buzz'"

# Global variable
debugger_evaluate({ expression: "global_var" })
```

### Attributes and Methods

```python
# Object attribute
debugger_evaluate({ expression: "obj.name" })
# Result: "'Alice'"

# Method call
debugger_evaluate({ expression: "len(my_list)" })
# Result: "10"

# Chained attributes
debugger_evaluate({ expression: "user.address.city" })
# Result: "'New York'"
```

### Lists and Dictionaries

```python
# List access
debugger_evaluate({ expression: "items[0]" })
# Result: "'first'"

# List slicing
debugger_evaluate({ expression: "items[1:3]" })
# Result: "['second', 'third']"

# Dictionary access
debugger_evaluate({ expression: "config['timeout']" })
# Result: "30"

# Dictionary get with default
debugger_evaluate({ expression: "config.get('missing', 'default')" })
# Result: "'default'"
```

### Arithmetic and Comparison

```python
# Arithmetic
debugger_evaluate({ expression: "n + 5" })
# Result: "20"

debugger_evaluate({ expression: "n * 2" })
# Result: "30"

# Modulo (useful for FizzBuzz!)
debugger_evaluate({ expression: "n % 3" })
# Result: "0"

debugger_evaluate({ expression: "n % 5" })
# Result: "0"

# Comparison
debugger_evaluate({ expression: "n > 10" })
# Result: "True"

debugger_evaluate({ expression: "n == 15" })
# Result: "True"
```

### Boolean Logic

```python
# Logical AND
debugger_evaluate({ expression: "n % 3 == 0 and n % 5 == 0" })
# Result: "True"

# Logical OR
debugger_evaluate({ expression: "n % 3 == 0 or n % 5 == 0" })
# Result: "True"

# Logical NOT
debugger_evaluate({ expression: "not (n % 2 == 0)" })
# Result: "True"
```

### String Operations

```python
# String concatenation
debugger_evaluate({ expression: "'Value: ' + str(n)" })
# Result: "'Value: 15'"

# f-string (Python 3.6+)
debugger_evaluate({ expression: "f'n is {n}'" })
# Result: "'n is 15'"

# String methods
debugger_evaluate({ expression: "result.upper()" })
# Result: "'FIZZBUZZ'"

debugger_evaluate({ expression: "result.startswith('Fizz')" })
# Result: "True"
```

### Type Checking and Conversion

```python
# Type checking
debugger_evaluate({ expression: "type(n)" })
# Result: "<class 'int'>"

debugger_evaluate({ expression: "isinstance(n, int)" })
# Result: "True"

# Type conversion
debugger_evaluate({ expression: "str(n)" })
# Result: "'15'"

debugger_evaluate({ expression: "int('42')" })
# Result: "42"
```

### Comprehensions

```python
# List comprehension
debugger_evaluate({ expression: "[x * 2 for x in range(5)]" })
# Result: "[0, 2, 4, 6, 8]"

# Dict comprehension
debugger_evaluate({ expression: "{x: x**2 for x in range(3)}" })
# Result: "{0: 0, 1: 1, 2: 4}"
```

### Common Patterns

```python
# Check if None
debugger_evaluate({ expression: "result is None" })
# Result: "False"

# Check if empty
debugger_evaluate({ expression: "len(my_list) == 0" })
# Result: "False"

# Ternary operator
debugger_evaluate({ expression: "'even' if n % 2 == 0 else 'odd'" })
# Result: "'odd'"
```

## Ruby (rdbg)

### Variables

```ruby
# Simple variable
debugger_evaluate({ expression: "n" })
# Result: "15"

# Instance variable
debugger_evaluate({ expression: "@count" })
# Result: "42"

# Class variable
debugger_evaluate({ expression: "@@total" })
# Result: "100"
```

### Attributes and Methods

```ruby
# Object attribute (via method)
debugger_evaluate({ expression: "user.name" })
# Result: "\"Alice\""

# Method call
debugger_evaluate({ expression: "arr.length" })
# Result: "10"

# Method with arguments
debugger_evaluate({ expression: "str.include?('test')" })
# Result: "true"

# Chained methods
debugger_evaluate({ expression: "arr.map(&:to_i).sum" })
# Result: "45"
```

### Arrays and Hashes

```ruby
# Array access
debugger_evaluate({ expression: "items[0]" })
# Result: "\"first\""

# Array slicing
debugger_evaluate({ expression: "items[1..3]" })
# Result: "[\"second\", \"third\", \"fourth\"]"

# Array methods
debugger_evaluate({ expression: "items.first" })
# Result: "\"first\""

debugger_evaluate({ expression: "items.last" })
# Result: "\"last\""

# Hash access
debugger_evaluate({ expression: "config[:timeout]" })
# Result: "30"

# Hash with string keys
debugger_evaluate({ expression: "config['timeout']" })
# Result: "30"

# Hash fetch with default
debugger_evaluate({ expression: "config.fetch(:missing, 'default')" })
# Result: "\"default\""
```

### Arithmetic and Comparison

```ruby
# Arithmetic
debugger_evaluate({ expression: "n + 5" })
# Result: "20"

debugger_evaluate({ expression: "n * 2" })
# Result: "30"

# Modulo
debugger_evaluate({ expression: "n % 3" })
# Result: "0"

debugger_evaluate({ expression: "n % 5" })
# Result: "0"

# Comparison
debugger_evaluate({ expression: "n > 10" })
# Result: "true"

debugger_evaluate({ expression: "n == 15" })
# Result: "true"

# Spaceship operator
debugger_evaluate({ expression: "n <=> 10" })
# Result: "1"  (n is greater than 10)
```

### Boolean Logic

```ruby
# Logical AND
debugger_evaluate({ expression: "n % 3 == 0 && n % 5 == 0" })
# Result: "true"

# Logical OR
debugger_evaluate({ expression: "n % 3 == 0 || n % 5 == 0" })
# Result: "true"

# Logical NOT
debugger_evaluate({ expression: "!(n % 2 == 0)" })
# Result: "true"
```

### String Operations

```ruby
# String concatenation
debugger_evaluate({ expression: "'Value: ' + n.to_s" })
# Result: "\"Value: 15\""

# String interpolation
debugger_evaluate({ expression: "\"n is #{n}\"" })
# Result: "\"n is 15\""

# String methods
debugger_evaluate({ expression: "result.upcase" })
# Result: "\"FIZZBUZZ\""

debugger_evaluate({ expression: "result.start_with?('Fizz')" })
# Result: "true"
```

### Type Checking and Conversion

```ruby
# Type checking
debugger_evaluate({ expression: "n.class" })
# Result: "Integer"

debugger_evaluate({ expression: "n.is_a?(Integer)" })
# Result: "true"

# Type conversion
debugger_evaluate({ expression: "n.to_s" })
# Result: "\"15\""

debugger_evaluate({ expression: "'42'.to_i" })
# Result: "42"
```

### Blocks and Enumerables

```ruby
# Map
debugger_evaluate({ expression: "[1, 2, 3].map { |x| x * 2 }" })
# Result: "[2, 4, 6]"

# Select (filter)
debugger_evaluate({ expression: "[1, 2, 3, 4].select { |x| x.even? }" })
# Result: "[2, 4]"

# Reduce
debugger_evaluate({ expression: "[1, 2, 3].reduce(:+)" })
# Result: "6"
```

### Common Patterns

```ruby
# Check if nil
debugger_evaluate({ expression: "result.nil?" })
# Result: "false"

# Check if empty
debugger_evaluate({ expression: "arr.empty?" })
# Result: "false"

# Ternary operator
debugger_evaluate({ expression: "n % 2 == 0 ? 'even' : 'odd'" })
# Result: "\"odd\""

# Safe navigation operator
debugger_evaluate({ expression: "user&.address&.city" })
# Result: "\"New York\"" or "nil"
```

## Node.js (vscode-js-debug)

### Variables

```javascript
// Simple variable
debugger_evaluate({ expression: "n" })
// Result: "15"

// Let/const variables
debugger_evaluate({ expression: "result" })
// Result: "'Buzz'"
```

### Properties and Methods

```javascript
// Object property
debugger_evaluate({ expression: "obj.name" })
// Result: "'Alice'"

// Method call
debugger_evaluate({ expression: "arr.length" })
// Result: "10"

// Function call
debugger_evaluate({ expression: "Math.max(1, 2, 3)" })
// Result: "3"

// Chained properties
debugger_evaluate({ expression: "user.address.city" })
// Result: "'New York'"
```

### Arrays and Objects

```javascript
// Array access
debugger_evaluate({ expression: "items[0]" })
// Result: "'first'"

// Array destructuring in expression
debugger_evaluate({ expression: "items.slice(1, 3)" })
// Result: "['second', 'third']"

// Array methods
debugger_evaluate({ expression: "items.length" })
// Result: "5"

debugger_evaluate({ expression: "items[items.length - 1]" })
// Result: "'last'"

// Object property access
debugger_evaluate({ expression: "config.timeout" })
// Result: "30"

// Bracket notation
debugger_evaluate({ expression: "config['timeout']" })
// Result: "30"

// Optional chaining
debugger_evaluate({ expression: "user?.address?.city" })
// Result: "'New York'" or "undefined"
```

### Arithmetic and Comparison

```javascript
// Arithmetic
debugger_evaluate({ expression: "n + 5" })
// Result: "20"

debugger_evaluate({ expression: "n * 2" })
// Result: "30"

// Modulo (note: %-4 issue in FizzBuzz bug!)
debugger_evaluate({ expression: "n % 3" })
// Result: "0"

debugger_evaluate({ expression: "n % 4" })  // Bug in fizzbuzz.js!
// Result: "3"

debugger_evaluate({ expression: "n % 5" })
// Result: "0"

// Comparison
debugger_evaluate({ expression: "n > 10" })
// Result: "true"

debugger_evaluate({ expression: "n === 15" })
// Result: "true"

// Strict vs loose equality
debugger_evaluate({ expression: "n == '15'" })
// Result: "true" (loose)

debugger_evaluate({ expression: "n === '15'" })
// Result: "false" (strict)
```

### Boolean Logic

```javascript
// Logical AND
debugger_evaluate({ expression: "n % 3 === 0 && n % 5 === 0" })
// Result: "true"

// Logical OR
debugger_evaluate({ expression: "n % 3 === 0 || n % 5 === 0" })
// Result: "true"

// Logical NOT
debugger_evaluate({ expression: "!(n % 2 === 0)" })
// Result: "true"

// Nullish coalescing
debugger_evaluate({ expression: "value ?? 'default'" })
// Result: "'default'" if value is null/undefined
```

### String Operations

```javascript
// String concatenation
debugger_evaluate({ expression: "'Value: ' + n" })
// Result: "'Value: 15'"

// Template literal
debugger_evaluate({ expression: "`n is ${n}`" })
// Result: "'n is 15'"

// String methods
debugger_evaluate({ expression: "result.toUpperCase()" })
// Result: "'FIZZBUZZ'"

debugger_evaluate({ expression: "result.startsWith('Fizz')" })
// Result: "true"

// Includes
debugger_evaluate({ expression: "result.includes('Buzz')" })
// Result: "true"
```

### Type Checking and Conversion

```javascript
// typeof
debugger_evaluate({ expression: "typeof n" })
// Result: "'number'"

debugger_evaluate({ expression: "typeof result" })
// Result: "'string'"

// instanceof
debugger_evaluate({ expression: "arr instanceof Array" })
// Result: "true"

// Type conversion
debugger_evaluate({ expression: "String(n)" })
// Result: "'15'"

debugger_evaluate({ expression: "Number('42')" })
// Result: "42"

debugger_evaluate({ expression: "parseInt('42', 10)" })
// Result: "42"
```

### Array Methods

```javascript
// Map
debugger_evaluate({ expression: "[1, 2, 3].map(x => x * 2)" })
// Result: "[2, 4, 6]"

// Filter
debugger_evaluate({ expression: "[1, 2, 3, 4].filter(x => x % 2 === 0)" })
// Result: "[2, 4]"

// Reduce
debugger_evaluate({ expression: "[1, 2, 3].reduce((a, b) => a + b, 0)" })
// Result: "6"

// Find
debugger_evaluate({ expression: "[1, 2, 3].find(x => x > 2)" })
// Result: "3"

// Some
debugger_evaluate({ expression: "[1, 2, 3].some(x => x > 2)" })
// Result: "true"

// Every
debugger_evaluate({ expression: "[1, 2, 3].every(x => x > 0)" })
// Result: "true"
```

### Common Patterns

```javascript
// Check if undefined/null
debugger_evaluate({ expression: "result === undefined" })
// Result: "false"

debugger_evaluate({ expression: "result == null" })
// Result: "false"

// Check if empty array
debugger_evaluate({ expression: "arr.length === 0" })
// Result: "false"

// Ternary operator
debugger_evaluate({ expression: "n % 2 === 0 ? 'even' : 'odd'" })
// Result: "'odd'"

// Truthy/falsy
debugger_evaluate({ expression: "!!n" })
// Result: "true"
```

### ES6+ Features

```javascript
// Spread operator
debugger_evaluate({ expression: "[...arr, 4, 5]" })
// Result: "[1, 2, 3, 4, 5]"

// Destructuring assignment
debugger_evaluate({ expression: "const [first, ...rest] = arr; first" })
// Result: "1"

// Object spread
debugger_evaluate({ expression: "{...obj, extra: true}" })
// Result: "{name: 'Alice', extra: true}"

// Arrow function immediate invocation
debugger_evaluate({ expression: "((x) => x * 2)(5)" })
// Result: "10"
```

## Advanced Evaluation Techniques

### Multi-line Expressions

Some debuggers support multi-line expressions:

```javascript
// Node.js (vscode-js-debug)
debugger_evaluate({
  expression: `
    const temp = n * 2;
    temp + 5
  `
})
// Result: "35"
```

### Side Effects (Use with Caution!)

Evaluations can have side effects, but this is generally discouraged:

```javascript
// ⚠️ Modifies variable during debugging
debugger_evaluate({ expression: "n = 20" })

// ⚠️ Calls function with side effects
debugger_evaluate({ expression: "console.log('Debug message')" })
```

**Best Practice**: Use read-only expressions for inspection.

### Watch Expressions

For repeated evaluation, consider using watch expressions (if supported):

```javascript
// Set up watch expression
debugger_set_watch({ expression: "n % 3" })
debugger_set_watch({ expression: "n % 5" })

// Now these evaluate automatically at each stop
```

## Debugging Expressions

### Common Errors

**1. ReferenceError: Variable not in scope**

```javascript
debugger_evaluate({ expression: "undefinedVar" })
// Error: ReferenceError: undefinedVar is not defined
```

**Solution**: Check variable is in current frame scope.

**2. TypeError: Cannot read property**

```javascript
debugger_evaluate({ expression: "obj.nonexistent.property" })
// Error: TypeError: Cannot read property 'property' of undefined
```

**Solution**: Use optional chaining or check existence first.

**3. SyntaxError: Invalid expression**

```javascript
debugger_evaluate({ expression: "if (n > 0) { ... }" })
// Error: SyntaxError: Unexpected token 'if'
```

**Solution**: Use ternary operator or simpler expressions.

### Frame Context

Expressions are evaluated in the context of the current stack frame:

```javascript
// At line 10, frame 0 (current function)
debugger_evaluate({ expression: "localVar", frame_id: 0 })
// Can access local variables

// At frame 1 (caller function)
debugger_evaluate({ expression: "parentVar", frame_id: 1 })
// Can access caller's variables
```

**Best Practice**: If `frame_id` is omitted, defaults to top frame (most recent).

## Language-Specific Gotchas

### Python

- **Integer division**: Use `//` for floor division, `/` for float division
- **Boolean values**: `True`/`False` (capitalized)
- **None vs null**: Use `is None`, not `== None`
- **String quotes**: Single `'` or double `"` both work

### Ruby

- **Symbols vs strings**: `:symbol` vs `"string"`
- **Boolean values**: `true`/`false` (lowercase)
- **nil vs null**: Use `.nil?`, not `== nil`
- **Question mark methods**: `empty?`, `nil?`, `even?` (Ruby convention)
- **Bang methods**: `upcase!` modifies in-place (side effects!)

### Node.js

- **Strict equality**: Prefer `===` over `==`
- **Boolean values**: `true`/`false` (lowercase)
- **undefined vs null**: Two different concepts
- **Template literals**: Use backticks `` ` ``, not quotes
- **Arrow functions**: `=>` syntax may not work in all expressions
- **Async/await**: Cannot use `await` in evaluate expressions

## Best Practices

1. **Start simple**: Test variable existence before complex expressions
2. **Use read-only expressions**: Avoid modifying state during debugging
3. **Test in REPL first**: Try expressions in language REPL before using in debugger
4. **Check frame context**: Ensure variable is in scope
5. **Handle null/undefined**: Use safe navigation operators
6. **Prefer built-in functions**: They're more reliable than custom code
7. **Keep expressions short**: Complex logic is hard to debug in expressions

## FizzBuzz-Specific Examples

### Diagnosing the Bug

```javascript
// At line 9, when n = 5
debugger_evaluate({ expression: "n" })
// Result: "5"

debugger_evaluate({ expression: "n % 3" })
// Result: "2" (not divisible by 3)

debugger_evaluate({ expression: "n % 4" })  // BUG: Should be % 5!
// Result: "1" (not divisible by 4)

debugger_evaluate({ expression: "n % 5" })
// Result: "0" (divisible by 5 - THIS is what we want!)
```

**Discovery**: Line 5 checks `n % 4` instead of `n % 5`, causing incorrect behavior.

### Verifying the Fix

```javascript
// After fixing line 5 to use n % 5
debugger_evaluate({ expression: "n % 5 === 0" })
// Result: "true" (for n = 5, 10, 15, etc.)

debugger_evaluate({ expression: "n % 4 === 0" })
// Result: "false" (for n = 5)
```

## Summary

Expression evaluation is a powerful debugging tool. Key takeaways:

✅ **Syntax varies by language** - Use appropriate operators/methods
✅ **Frame context matters** - Variables must be in scope
✅ **Start simple** - Test variable existence first
✅ **Read-only preferred** - Avoid side effects
✅ **Use language idioms** - Follow each language's conventions
✅ **Handle edge cases** - Check for null/undefined/nil

## References

- [Python Expressions](https://docs.python.org/3/reference/expressions.html)
- [Ruby Syntax](https://ruby-doc.org/core/doc/syntax_rdoc.html)
- [JavaScript Expressions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators)
- [DAP Evaluate Request](https://microsoft.github.io/debug-adapter-protocol/specification#Requests_Evaluate)

---

**Author**: Claude Code
**Last Updated**: 2025-10-07
**Version**: 1.0.0
