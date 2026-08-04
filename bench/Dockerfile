# Benchmark environment: BioLang, Python and R in one image.
#
# One image rather than three, deliberately. Comparing a number from one
# container against a number from another — or worse, against the host — makes
# the container a source of variance rather than a control for it. Everything
# here is measured by the same kernel, the same CPU governor and the same libc.
#
# What this fixes: reproducibility and environment drift. An earlier attempt at
# an R comparison died because the system R library under Program Files was not
# writable, which is exactly the class of problem an image removes.
#
# What this does NOT fix: fairness. The two worst bugs this harness has had were
# design bugs — Python handed a pre-loaded string while BioLang was charged for
# parsing a megabase, and a "FASTA parse" workload that was really timing
# `import Bio`. A container would have reproduced both faithfully and lent them
# the authority of a pinned environment. Fairness lives in the harness.
#
#   docker build -f bench/Dockerfile -t biolang-bench .
#   docker run --rm -v "$PWD/bench:/out" biolang-bench
#
# Pin the CPU for less variance:
#   docker run --rm --cpuset-cpus=0-3 -v "$PWD/bench:/out" biolang-bench

# Bioconductor image: R plus Biostrings already built, which otherwise takes
# twenty minutes to compile from source.
FROM bioconductor/bioconductor_docker:RELEASE_3_20

ARG DEBIAN_FRONTEND=noninteractive

# Rust, to build BioLang from this tree rather than trusting a copied binary
# that may have been built against a different libc.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Python comparators. Versions are pinned so a rerun months later measures the
# same libraries; an unpinned benchmark compares against a moving target.
RUN python3 -m pip install --no-cache-dir --break-system-packages \
      biopython==1.85 \
      edlib==1.3.9.post1 \
      hmmlearn==0.3.3 \
      numpy==2.2.6 \
      pydivsufsort==0.0.15

# Biostrings is *not* in the base image — it ships BiocManager and the
# Bioconductor repositories, not the packages themselves. Assuming otherwise is
# what broke the first build of this file.
RUN Rscript -e 'BiocManager::install("Biostrings", ask = FALSE, update = FALSE)' && \
    Rscript -e 'suppressPackageStartupMessages(library(Biostrings)); cat("Biostrings", as.character(packageVersion("Biostrings")), "\n")'

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p bl-cli && \
    cargo --version && ./target/release/bl --version

COPY bench ./bench

# Results land in /out so a bind mount captures them; without the mount they
# stay in the container and the run is merely a smoke test.
CMD ["sh", "-c", "python3 bench/harness.py --json /out/results-container.json"]
