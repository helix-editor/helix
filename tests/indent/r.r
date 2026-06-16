f <- function(x) {
  total <- 0
  for (v in x) {
    if (v > 0) {
      total <- total + v
    } else {
      total <- total - v
    }
  }
  result <- lapply(
    items,
    process
  )
  total
}
