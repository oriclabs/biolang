# Appendix A: Installation and Setup

This appendix walks you through installing everything you need for this book: BioLang itself, plus the optional Python and R environments for running comparison scripts.

## Installing BioLang

### macOS and Linux

Open a terminal and run the installer:

```bash
curl -sSf https://biolang.org/install.sh | sh
```

This downloads the latest release binary and installs it to `~/.biolang/bin/`. The installer adds this directory to your `PATH` automatically. You may need to restart your terminal or run `source ~/.bashrc` (or `source ~/.zshrc` on macOS) for the change to take effect.

To verify the installation:

```bash
bl --version
```

You should see output like:

```
biolang 0.1.0
```

### Windows

Open PowerShell and run:

```powershell
irm https://biolang.org/install.ps1 | iex
```

This installs `bl.exe` to `%USERPROFILE%\.biolang\bin\` and adds it to your user `PATH`. You may need to restart your terminal.

Alternatively, if you have [Scoop](https://scoop.sh/) installed:

```powershell
scoop install biolang
```

### Building from Source

If you prefer to build from source, you need Rust 1.75 or later:

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/bioras/biolang.git
cd biolang
cargo build --release

# The binary is at target/release/bl
```

## The `bl` CLI

BioLang provides a single command-line tool called `bl` with several subcommands:

### `bl repl` — Interactive Mode

Launches the Read-Eval-Print Loop where you can type BioLang expressions and see results immediately:

```bash
bl repl
```

Or simply:

```bash
bl
```

Running `bl` with no arguments starts the REPL by default. This is the best way to experiment with new concepts.

REPL commands (type these at the `bl>` prompt):

| Command | Description |
|---------|-------------|
| `:help` | Show available REPL commands |
| `:env` | Display all variables in the current environment |
| `:reset` | Clear the environment and start fresh |
| `:load file.bl` | Load and execute a script file |
| `:save file.bl` | Save the current session to a file |
| `:time expr` | Measure execution time of an expression |
| `:type expr` | Show the type of an expression |
| `:profile expr` | Profile an expression's execution |
| `:plugins` | List available plugins |
| `:history` | Show command history |
| `:plot` | Show the last generated plot |

### `bl run` — Execute a Script

Runs a `.bl` script file:

```bash
bl run my_script.bl
```

You can pass arguments to the script:

```bash
bl run analysis.bl input.fastq output.csv
```

### `bl init` — Create a New Project

Scaffolds a new BioLang project directory:

```bash
bl init my-project
```

This creates:

```
my-project/
  main.bl        # Entry point
  data/          # Data directory
  results/       # Output directory
```

### `bl lsp` — Language Server

Starts the Language Server Protocol server for editor integration:

```bash
bl lsp
```

You typically do not run this directly — your editor starts it automatically.

### `bl plugins` — Plugin Management

Lists or manages BioLang plugins:

```bash
bl plugins          # List installed plugins
bl plugins install  # Install a plugin
```

## Setting Up Python (Optional)

Python comparison scripts require Python 3.8 or later. Most exercises use BioPython.

### Check Your Python Installation

```bash
python3 --version   # macOS/Linux
python --version    # Windows
```

### Create a Virtual Environment

We recommend using a virtual environment so the book's dependencies do not interfere with your system Python:

```bash
# Create the environment
python3 -m venv bio-env

# Activate it
source bio-env/bin/activate      # macOS/Linux
bio-env\Scripts\activate          # Windows PowerShell
```

### Install Required Packages

```bash
pip install biopython pandas numpy scipy matplotlib seaborn requests
```

These packages cover all the Python comparison scripts in the book:

| Package | Used For |
|---------|----------|
| `biopython` | Sequence I/O, NCBI access, BLAST |
| `pandas` | Table operations, CSV handling |
| `numpy` | Numerical computing |
| `scipy` | Statistical tests |
| `matplotlib` | Plotting |
| `seaborn` | Statistical visualization |
| `requests` | API access |

### Verify Python Setup

```bash
python3 -c "from Bio import SeqIO; print('BioPython OK')"
python3 -c "import pandas; print('Pandas OK')"
```

## Setting Up R (Optional)

R comparison scripts require R 4.0 or later with Bioconductor packages.

### Install R

- **macOS**: Download from [https://cran.r-project.org/](https://cran.r-project.org/) or use `brew install r`
- **Linux**: Use your package manager (`sudo apt install r-base` on Ubuntu/Debian)
- **Windows**: Download from [https://cran.r-project.org/](https://cran.r-project.org/)

### Install Required Packages

Open an R console (`R` or `Rscript`) and run:

```r
# CRAN packages
install.packages(c("tidyverse", "ggplot2", "data.table", "jsonlite", "httr"))

# Bioconductor packages
if (!requireNamespace("BiocManager", quietly = TRUE))
    install.packages("BiocManager")

BiocManager::install(c("Biostrings", "GenomicRanges", "DESeq2",
                        "VariantAnnotation", "Rsamtools"))
```

### Verify R Setup

```bash
Rscript -e 'library(Biostrings); cat("Biostrings OK\n")'
Rscript -e 'library(tidyverse); cat("tidyverse OK\n")'
```

## Editor Setup

You can write BioLang in any text editor, but we recommend Visual Studio Code for the best experience.

### VS Code

1. Install [VS Code](https://code.visualstudio.com/)
2. Open the Extensions panel (`Ctrl+Shift+X` / `Cmd+Shift+X`)
3. Search for "BioLang" and install the BioLang extension
4. The extension provides:
   - Syntax highlighting for `.bl` files
   - Code completion via the language server
   - Hover documentation for builtins
   - Error diagnostics as you type
   - REPL integration

### Other Editors

Any editor that supports the Language Server Protocol (LSP) can use `bl lsp` for BioLang support. For editors without LSP support, you will still get a good experience — BioLang syntax is clean enough to read without highlighting.

## Environment Variables

Some features in this book require API keys. These are optional — you can complete most exercises without them — but they unlock higher rate limits and additional data sources.

| Variable | Purpose | Required? |
|----------|---------|-----------|
| `NCBI_API_KEY` | NCBI E-utilities — increases rate limit from 3 to 10 requests/second | Optional (recommended for Day 9, 24) |
| `ANTHROPIC_API_KEY` | Claude AI integration for Day 26 (AI-Assisted Analysis) | Optional (Day 26 only) |
| `OPENAI_API_KEY` | Alternative LLM provider for Day 26 | Optional (Day 26 only) |

### Setting Environment Variables

**macOS/Linux** — add to your `~/.bashrc` or `~/.zshrc`:

```bash
export NCBI_API_KEY="your-key-here"
export ANTHROPIC_API_KEY="your-key-here"
```

Then run `source ~/.bashrc` to apply.

**Windows** — set in PowerShell or System Settings:

```powershell
[Environment]::SetEnvironmentVariable("NCBI_API_KEY", "your-key-here", "User")
```

### Getting an NCBI API Key

1. Create a free NCBI account at [https://www.ncbi.nlm.nih.gov/account/](https://www.ncbi.nlm.nih.gov/account/)
2. Go to Settings > API Key Management
3. Click "Create an API Key"
4. Copy the key and set it as `NCBI_API_KEY`

## Getting the Companion Files

The companion files contain all exercise solutions, sample data generators, and comparison scripts.

### Option 1: Git Clone

```bash
git clone https://github.com/bioras/practical-bioinformatics.git
cd practical-bioinformatics
```

### Option 2: Download ZIP

Download from the book's website and extract to a directory of your choice.

### Directory Structure

After cloning, the companion directory looks like this:

```
practical-bioinformatics/
  days/
    day-01/
      init.bl
      scripts/
      expected/
      compare.md
    day-02/
      ...
    day-30/
      ...
  data/           # Shared sample data
  book/           # This book's source
```

### Running a Day's Setup

Each day has an `init.bl` script that prepares sample data:

```bash
cd days/day-06
bl run init.bl
```

This creates any necessary test files in the day's directory. Always run `init.bl` before starting a day's exercises.

## Verifying Everything Works

Run this quick check to confirm your environment is ready:

```bash
# BioLang
bl -e 'println("BioLang: OK")'

# Check REPL
echo ':help' | bl repl

# Python (optional)
python3 -c "from Bio import SeqIO; print('Python: OK')"

# R (optional)
Rscript -e 'cat("R: OK\n")'
```

If BioLang prints "BioLang: OK", you are ready to start Day 1.

## Troubleshooting

### "bl: command not found"

The `bl` binary is not on your `PATH`. Add it:

```bash
# macOS/Linux
export PATH="$HOME/.biolang/bin:$PATH"

# Add to your shell profile to make it permanent
echo 'export PATH="$HOME/.biolang/bin:$PATH"' >> ~/.bashrc
```

On Windows, check that `%USERPROFILE%\.biolang\bin` is in your system `PATH`.

### Permission Denied (macOS)

macOS may block the binary because it was downloaded from the internet:

```bash
xattr -d com.apple.quarantine ~/.biolang/bin/bl
```

### Python Package Install Fails

If `pip install biopython` fails, try:

```bash
pip install --upgrade pip
pip install biopython
```

On Linux, you may need development headers:

```bash
sudo apt install python3-dev   # Debian/Ubuntu
sudo dnf install python3-devel # Fedora
```

### R Bioconductor Install Fails

Bioconductor packages can take a long time to compile. If installation times out or fails:

```r
# Try installing one at a time
BiocManager::install("Biostrings")
BiocManager::install("GenomicRanges")
```

On Linux, you may need system libraries:

```bash
sudo apt install libcurl4-openssl-dev libxml2-dev libssl-dev  # Debian/Ubuntu
```

### Firewall or Proxy Issues

If you are behind a corporate firewall, you may need to configure proxy settings:

```bash
export HTTP_PROXY="http://proxy.example.com:8080"
export HTTPS_PROXY="http://proxy.example.com:8080"
```

### Getting Help

If you are stuck:

1. Check the [BioLang documentation](https://biolang.org/docs/)
2. Search the [GitHub Issues](https://github.com/bioras/biolang/issues)
3. Ask in the BioLang community forum
