library(GenomicRanges)

regions <- read.delim("data/regions.bed", header=FALSE,
                      col.names=c("chrom","start","end","name","score","strand"))
queries <- read.delim("data/queries.bed", header=FALSE,
                      col.names=c("chrom","start","end","name","score","strand"))

gr_regions <- GRanges(seqnames=regions$chrom, ranges=IRanges(start=regions$start+1, end=regions$end))
gr_queries <- GRanges(seqnames=queries$chrom, ranges=IRanges(start=queries$start+1, end=queries$end))

hits <- findOverlaps(gr_queries, gr_regions)

cat(sprintf("Regions: %d\n", length(gr_regions)))
cat(sprintf("Queries: %d\n", length(gr_queries)))
cat(sprintf("Total overlaps: %d\n", length(hits)))
