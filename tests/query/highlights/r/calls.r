f <- function() {
  helper(1)
# ^ @function
  pkg::func(2)
#      ^ @function
}
