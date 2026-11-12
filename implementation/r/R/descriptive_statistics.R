# APSInsight descriptive statistics (optional R runtime)

#' Compute descriptive statistics for a numeric vector
#' @param x numeric vector
#' @return named list of statistics
descriptive_stats <- function(x) {
  list(
    mean = mean(x, na.rm = TRUE),
    sd = sd(x, na.rm = TRUE),
    median = median(x, na.rm = TRUE),
    n = length(x[!is.na(x)])
  )
}
