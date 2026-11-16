proc greet(name: string): string =
#                                 ^ 2 eof
  result = "hello, " & name

iterator items(n: int): int =
#                            ^ 2 eof
  for i in 0 .. n:
    yield i

for i in 0 .. 10:
  echo i

while running:
  step()

case n
of 1:
  echo "one"
#           ^ 2
of 2:
  echo "two"
#           ^ 2
else:
  echo "many"
