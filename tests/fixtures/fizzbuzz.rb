# FizzBuzz implementation with a deliberate bug
# Bug: Line 9 uses % 4 instead of % 5 for "Buzz"

def fizzbuzz(n)
  if n % 15 == 0
    "FizzBuzz"
  elsif n % 3 == 0
    "Fizz"
  elsif n % 4 == 0  # BUG: should be % 5
    "Buzz"
  else
    n.to_s
  end
end

def main
  (1..100).each do |i|
    puts fizzbuzz(i)
  end
end

main if __FILE__ == $0
