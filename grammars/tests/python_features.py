async def fetch():
    await asyncio.sleep(1)
    return 42

@decorator
def greet(name):
    with open("f.txt") as f:
        try:
            x = 1
        except ValueError as e:
            raise e from None
    return f"hello {name}"

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

for a, b in [(1, 2)]:
    pass
