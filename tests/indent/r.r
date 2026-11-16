f <- function(x) {
#                 ^ 2 eof
  if (a) {}
#         ^ 2
  total <- 0
  for (v in x) {
#               ^ 4 eof
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
