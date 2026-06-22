object Main {
  def process(items: List[Int]): Int = {
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
