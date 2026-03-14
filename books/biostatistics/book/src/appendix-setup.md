# Appendix A: Installation and Setup

This appendix walks you through installing BioLang and the optional Python and R environments used for multi-language comparisons throughout the book.

## Installing BioLang

### Linux and macOS

Open a terminal and run:

```bash
curl -sSf https://biolang.org/install.sh | sh
```

This installs the `bl` binary to `~/.biolang/bin/` and adds it to your PATH. Restart your terminal or run:

```bash
source ~/.bashrc   # or ~/.zshrc on macOS
```

Verify the installation:

```bash
bl --version
```

You should see output like `biolang 0.x.y`. Any version 0.4.0 or later includes all the statistical builtins used in this book.

### Windows

Open PowerShell and run:

```powershell
irm https://biolang.org/install.ps1 | iex
```

This installs `bl.exe` to `%USERPROFILE%\.biolang\bin\` and updates your PATH. Close and reopen PowerShell, then verify:

```powershell
bl --version
```

### Manual Installation

If the installer does not work for your system, download the appropriate binary from the [releases page](https://github.com/bioras/biolang/releases):

| Platform | File |
|---|---|
| Linux x86_64 | `bl-linux-x86_64.tar.gz` |
| Linux aarch64 | `bl-linux-aarch64.tar.gz` |
| macOS x86_64 | `bl-macos-x86_64.tar.gz` |
| macOS Apple Silicon | `bl-macos-aarch64.tar.gz` |
| Windows x86_64 | `bl-windows-x86_64.zip` |

Extract the binary and place it somewhere on your PATH.

### Verifying Statistical Builtins

To confirm that statistical functions are available, launch the REPL and try a quick test:

```
bl> ttest([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
{statistic: -3.674, p_value: 0.0214, df: 4}

bl> mean([10, 20, 30, 40, 50])
30.0

bl> sd([10, 20, 30, 40, 50])
15.8114
```

If these commands produce output, your installation is complete.

## Python Setup (Optional)

Python comparisons are included for every day but are not required. If you want to run them, you need Python 3.8 or later.

### Installing Python

Most systems come with Python pre-installed. Check your version:

```bash
python3 --version
```

If you need to install Python, visit [python.org](https://www.python.org/downloads/) or use your system package manager:

```bash
# Ubuntu / Debian
sudo apt install python3 python3-pip python3-venv

# macOS (Homebrew)
brew install python3

# Windows — download from python.org
```

### Installing Python Dependencies

We recommend using a virtual environment to keep the book's dependencies isolated:

```bash
python3 -m venv biostat-env
source biostat-env/bin/activate   # Linux/macOS
# biostat-env\Scripts\activate    # Windows
```

Install all required packages:

```bash
pip install scipy numpy pandas matplotlib statsmodels lifelines scikit-learn seaborn
```

| Package | Version | Used For |
|---|---|---|
| `scipy` | >= 1.10 | Statistical tests, distributions |
| `numpy` | >= 1.24 | Numerical arrays |
| `pandas` | >= 2.0 | Data frames, I/O |
| `matplotlib` | >= 3.7 | Plotting |
| `statsmodels` | >= 0.14 | Regression, ANOVA, time series |
| `lifelines` | >= 0.27 | Survival analysis |
| `scikit-learn` | >= 1.3 | PCA, clustering, preprocessing |
| `seaborn` | >= 0.13 | Statistical visualization |

Verify the installation:

```bash
python3 -c "import scipy; import statsmodels; import lifelines; print('Python setup OK')"
```

### Troubleshooting Python

**`pip: command not found`** — Use `pip3` instead of `pip`, or install pip: `python3 -m ensurepip`.

**`ModuleNotFoundError`** — Make sure your virtual environment is activated. The prompt should show `(biostat-env)`.

**Version conflicts** — If you have existing Python packages that conflict, create a fresh virtual environment dedicated to this book.

## R Setup (Optional)

R comparisons are included for every day but are not required. If you want to run them, you need R 4.0 or later.

### Installing R

Download R from [CRAN](https://cran.r-project.org/):

```bash
# Ubuntu / Debian
sudo apt install r-base r-base-dev

# macOS (Homebrew)
brew install r

# Windows — download from cran.r-project.org
```

Verify:

```bash
R --version
```

### Installing R Packages

Launch R and install the required packages:

```r
install.packages(c(
  "stats",        # Base statistics (usually pre-installed)
  "survival",     # Survival analysis
  "ggplot2",      # Visualization
  "dplyr",        # Data manipulation
  "pwr",          # Power analysis
  "lme4",         # Mixed models
  "boot",         # Bootstrap methods
  "car",          # ANOVA type II/III
  "broom",        # Tidy model output
  "pheatmap",     # Heatmaps
  "ggrepel",      # Label placement for plots
  "multcomp"      # Multiple comparisons
))
```

Verify:

```r
library(survival)
library(ggplot2)
library(pwr)
cat("R setup OK\n")
```

### Troubleshooting R

**`package 'xxx' is not available`** — Update your CRAN mirror: `chooseCRANmirror()`. Select a mirror close to your location.

**Compilation errors on Linux** — Install development libraries: `sudo apt install libcurl4-openssl-dev libxml2-dev libssl-dev`.

**Permission denied** — Install packages to a user library: `install.packages("pkg", lib = Sys.getenv("R_LIBS_USER"))`.

## Running Companion Scripts

Each day's companion directory contains three analysis scripts. Here is how to run each one:

### BioLang

```bash
cd days/day-07
bl run init.bl           # Setup (generates data, downloads files)
bl run scripts/analysis.bl    # Run the analysis
```

### Python

```bash
cd days/day-07
source ~/biostat-env/bin/activate   # Activate virtual environment
python3 scripts/analysis.py
```

### R

```bash
cd days/day-07
Rscript scripts/analysis.R
```

### Checking Expected Output

Each day includes an `expected/` directory with reference output. You can diff your results:

```bash
bl run scripts/analysis.bl > my_output.txt
diff my_output.txt expected/output.txt
```

> **Key insight:** Statistical results may differ slightly between languages due to floating-point arithmetic and algorithmic differences. Results that agree to 2-3 decimal places are considered matching. The companion `compare.md` file notes any expected discrepancies.

## Editor Setup

BioLang has a Language Server Protocol (LSP) implementation that provides syntax highlighting, autocompletion, and inline diagnostics in supported editors.

### VS Code

Install the BioLang extension from the VS Code marketplace. It includes the LSP client and syntax highlighting for `.bl` files.

### Vim / Neovim

Add the BioLang LSP to your `lspconfig`:

```lua
require('lspconfig').biolang.setup{}
```

### Other Editors

Any editor that supports LSP can use the BioLang language server. Start it with:

```bash
bl lsp
```

## Directory Structure for the Book

We recommend organizing your working directory like this:

```
biostatistics/
  days/               # Companion files (from git clone)
  my-work/            # Your own scripts and notes
    day-01/
    day-02/
    ...
  data/               # Shared datasets across days
```

This keeps the companion files clean while giving you a place to experiment.

## System Requirements

| Requirement | Minimum | Recommended |
|---|---|---|
| RAM | 4 GB | 8 GB |
| Disk | 2 GB free | 5 GB free |
| OS | Windows 10, macOS 11, Ubuntu 20.04 | Latest stable |
| BioLang | 0.4.0 | Latest |
| Python | 3.8 (optional) | 3.11+ |
| R | 4.0 (optional) | 4.3+ |

All exercises in this book run comfortably on a standard laptop. No GPU, cluster access, or cloud computing is required.
