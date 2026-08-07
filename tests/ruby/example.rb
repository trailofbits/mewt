def factorial(n)
  return 1 if n <= 1

  result = 1
  while n > 1
    result *= n
    n -= 1
  end

  result
end

def clamp(value, low, high)
  return low unless value >= low
  return high if value > high

  value
end

def countdown(n)
  until n <= 0
    puts n
    n -= 1
  end
end

def describe(value)
  label = value > 0 ? "positive" : "non-positive"

  case value
  when 0
    "zero"
  else
    label
  end
end

def safe_parse(text)
  Integer(text)
rescue ArgumentError
  nil
end

puts factorial(5)
puts clamp(10, 0, 5)
countdown(3)
puts describe(2)
puts safe_parse("not a number").inspect
