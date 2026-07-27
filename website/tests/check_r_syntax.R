args <- commandArgs(trailingOnly = TRUE)
roots <- if (length(args)) args else c(
  "books/practical-bioinformatics/days",
  "books/biostatistics/days"
)

files <- unlist(lapply(
  roots,
  list.files,
  pattern = "[.]R$",
  recursive = TRUE,
  full.names = TRUE
))

failures <- character()
for (file in files) {
  tryCatch(
    parse(file = file),
    error = function(error) {
      failures <<- c(
        failures,
        paste(file, conditionMessage(error), sep = ": ")
      )
    }
  )
}

cat(sprintf(
  "R files: %d; syntax failures: %d\n",
  length(files),
  length(failures)
))

if (length(failures)) {
  cat(paste(failures, collapse = "\n"), "\n")
  quit(status = 1)
}
