# Getting the data

**You do not need to download anything to work through this companion.**

Every script generates its own data with `set_seed`, so it runs offline, gives
the same numbers on every machine, and costs the repository nothing. Where a
real dataset matters — the *C. elegans* mitochondrial base counts — the handful
of numbers are written into the script directly.

That is a deliberate choice. Simulated data has a property real data does not:
you know the right answer, because you chose it. When a script draws from
`Poisson(0.5)` and then asks whether a count of 7 is surprising, the truth is
not in question, so any disagreement is a bug in the analysis rather than a
mystery about biology. That makes it far better for learning the method.

## If you want the book's original data

The authors publish the full bundle. It is not mirrored here — it is theirs to
host, and it would add tens of megabytes to this repository for files most
readers never open.

```bash
curl -O https://www.huber.embl.de/msmb/data.tar.gz
tar xzf data.tar.gz
```

Point the scripts at it with:

```bash
export MSMB_DATA=/path/to/data
```

The files are R serialisation formats (`.RData`, `.rds`), which BioLang does not
read. Converting one needs R:

```r
load("data/e100.RData")
write.csv(data.frame(count = e100), "e100.csv", row.names = FALSE)
```

Chapter 1's `e100` is 100 Poisson(0.5) draws with position 42 replaced by 7 —
which `04-epitope-detection.bl` reproduces directly, so the conversion is
optional even there.

## Why the data directory is gitignored

`books/msmb/data/` is ignored. Download into it freely; nothing you put there
will be committed.
