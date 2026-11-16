object Main {
  def process(items: List[Int]): Int = {
    if (a) {}
//          ^ 4
    var total = 0
    for (v <- items) {
      if (v > 0) {
        total += v
      } else {
        total -= v
      }
    }
    total
  }

  val config = Map(
    "a" -> 1,
    "b" -> 2,
  )
}
