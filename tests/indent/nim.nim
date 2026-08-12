proc greet(name: string): string =
  result = "hello, " & name

iterator items(n: int): int =
  for i in 0 .. n:
    yield i

for i in 0 .. 10:
  echo i

while running:
  step()

case n
of 1:
  echo "one"
of 2:
  echo "two"
else:
  echo "many"
