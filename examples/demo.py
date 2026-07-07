class Calculator:
    def __init__(self, value=0):
        self.value = value

    def add(self, x):
        return Calculator(self.value + x)

    def multiply(self, x):
        return Calculator(self.value * x)

    def __repr__(self):
        return f"Calculator({self.value})"

def fibonacci(n):
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b

result = fibonacci(10)
calc = Calculator(5).add(3).multiply(2)
print(f"Fibonacci(10) = {result}")
print(f"Calculator: {calc}")
