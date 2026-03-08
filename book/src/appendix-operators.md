# Appendix B: Operator Reference

BioLang operators are listed below from **highest precedence** (binds
tightest) to **lowest**. Within the same precedence level, operators are
left-associative unless noted otherwise.

---

## Precedence Table

### 1. Access and Call (highest)

| Symbol | Name | Description | Example |
|---|---|---|---|
| `.` | Field access | Access a record field or method | `variant.chrom` |
| `()` | Call | Invoke a function or builtin | `gc_content(seq)` |
| `[]` | Index | Index into a list, string, or sequence | `reads[0]` |

### 2. Exponentiation

| Symbol | Name | Description | Example |
|---|---|---|---|
| `**` | Power | Raise to a power (right-associative) | `2 ** 10` gives 1024 |

### 3. Unary

| Symbol | Name | Description | Example |
|---|---|---|---|
| `-` | Negate | Arithmetic negation | `-log2(fc)` |
| `!` | Logical NOT | Boolean negation | `!is_snp(v)` |
| `not` | Logical NOT (word) | Same as `!`, reads more naturally in predicates | `not is_indel(v)` |
| `~` | Bitwise NOT | Bitwise complement | `~flags` |

### 4. Multiplicative

| Symbol | Name | Description | Example |
|---|---|---|---|
| `*` | Multiply | Arithmetic multiplication | `coverage * depth` |
| `/` | Divide | Arithmetic division | `gc_count / len(seq)` |
| `%` | Modulo | Remainder after division; useful for reading frame math | `pos % 3` |

### 5. Additive

| Symbol | Name | Description | Example |
|---|---|---|---|
| `+` | Add | Arithmetic addition; also concatenates strings | `start + kb(1)` |
| `-` | Subtract | Arithmetic subtraction | `end - start` |

### 6. Type Cast

| Symbol | Name | Description | Example |
|---|---|---|---|
| `as` | Cast | Convert between compatible types | `qual as Int` |

### 7. Ranges

| Symbol | Name | Description | Example |
|---|---|---|---|
| `..` | Range (exclusive) | Half-open range; useful for genomic coordinates | `0..len(seq)` |
| `..=` | Range (inclusive) | Closed range | `1..=22` for autosomal chromosomes |

### 8. Bit Shifts

| Symbol | Name | Description | Example |
|---|---|---|---|
| `<<` | Left shift | Shift bits left | `1 << 4` |
| `>>` | Right shift | Shift bits right; useful for SAM flag decoding | `flags >> 8` |

### 9. Bitwise AND

| Symbol | Name | Description | Example |
|---|---|---|---|
| `&` | Bitwise AND | Test individual flag bits in BAM flags | `flags & 0x4` to check unmapped |

### 10. Bitwise XOR

| Symbol | Name | Description | Example |
|---|---|---|---|
| `^` | Bitwise XOR | Exclusive or | `a ^ b` |

### 11. Comparison

| Symbol | Name | Description | Example |
|---|---|---|---|
| `==` | Equal | Structural equality | `variant_type(v) == "snp"` |
| `!=` | Not equal | Structural inequality | `v.chrom != "chrM"` |
| `<` | Less than | Numeric or lexicographic comparison | `v.qual < 30` |
| `>` | Greater than | | `gc_content(seq) > 0.6` |
| `<=` | Less or equal | | `len(seq) <= kb(1)` |
| `>=` | Greater or equal | | `depth >= 10` |
| `~` | Regex match | True if the left string matches the right pattern | `header ~ "^>chr[0-9]+"` |

### 12. Logical AND

| Symbol | Name | Description | Example |
|---|---|---|---|
| `&&` | AND | Short-circuiting logical and | `is_snp(v) && v.qual > 30` |
| `and` | AND (word) | Same as `&&`, reads naturally in filters | `is_snp(v) and v.qual > 30` |

### 13. Logical OR

| Symbol | Name | Description | Example |
|---|---|---|---|
| `\|\|` | OR | Short-circuiting logical or | `is_het(v) \|\| is_hom_alt(v)` |
| `or` | OR (word) | Same as `\|\|` | `is_het(v) or is_hom_alt(v)` |

### 14. Null Coalesce

| Symbol | Name | Description | Example |
|---|---|---|---|
| `??` | Null coalesce | Use the right-hand value when the left is nil | `v.info["AF"] ?? 0.0` |

### 15. Pipe

| Symbol | Name | Description | Example |
|---|---|---|---|
| `\|>` | Pipe | Pass the left-hand value as the first argument to the right-hand function | `reads \|> filter(\|r\| mean_phred(r.quality) > 30)` |
| `\|>>` | Tap pipe | Like pipe, but passes the value through unchanged; used for side effects | `reads \|>> print \|> len()` |

The pipe operators are the idiomatic way to build multi-step analysis
pipelines in BioLang:

```
read_fastq("sample.fq")
  |> filter(|r| mean_phred(r.quality) > 25)
  |>> |reads| print("After QC: " + str(len(reads)) + " reads")
  |> map(|r| {id: r.id, gc: gc_content(r.seq)})
  |> write_csv("gc_report.csv")
```

### 16. Assignment (lowest)

| Symbol | Name | Description | Example |
|---|---|---|---|
| `=` | Assign | Bind a value to a name | `let gc = gc_content(seq)` |
| `+=` | Add-assign | Increment in place | `count += 1` |
| `-=` | Subtract-assign | Decrement in place | `remaining -= 1` |
| `*=` | Multiply-assign | Multiply in place | `score *= weight` |
| `/=` | Divide-assign | Divide in place | `total /= n` |
| `?=` | Nil-assign | Assign only if the target is currently nil | `cache ?= compute()` |

### 17. Structural (not expression operators)

These symbols appear in specific syntactic positions and do not participate
in general expression precedence.

| Symbol | Name | Description | Example |
|---|---|---|---|
| `=>` | Fat arrow | Separates patterns from bodies in `match` and `given` arms | `match v { "snp" => handle_snp() }` |
| `->` | Arrow | Return type annotation in function signatures | `fn gc(seq: Dna) -> Float` |

---

## Newlines and Statement Termination

BioLang uses **newlines as statement terminators** -- there are no
semicolons. A newline ends the current expression unless it is suppressed.

### Newline Suppression Rules

A newline does **not** terminate a statement when it appears:

1. **After a binary operator.** The expression continues on the next line.

   ```
   let total = a +
       b +
       c
   ```

2. **After an opening delimiter** (`(`, `[`, `{`). The expression continues
   until the matching closing delimiter.

   ```
   let record = {
       chrom: "chr1",
       start: 1000,
       end: 2000
   }
   ```

3. **After a pipe operator** (`|>` or `|>>`). The pipeline continues on the
   next line.

   ```
   reads
     |> filter(|r| mean_phred(r.quality) > 30)
     |> map(|r| r.seq)
   ```

4. **After a comma** inside argument lists, list literals, and record
   literals.

   ```
   let xs = [
       1,
       2,
       3
   ]
   ```

5. **After keywords that expect a continuation**: `then`, `else`, `do`,
   `and`, `or`.

These rules mean that multi-line pipelines and data structures work
naturally without any explicit continuation characters.

### Blank Lines

Blank lines (lines containing only whitespace) are ignored between
statements. Use them freely to organize code into logical sections.

---

## Comments

| Syntax | Name | Description |
|---|---|---|
| `#` | Line comment | Everything from `#` to end of line is ignored |
| `##` | Doc comment | Attached to the following declaration; extractable by documentation tools |

```
# This is a regular comment

## Compute GC content for a DNA sequence.
## Returns a float between 0.0 and 1.0.
fn gc(seq: Dna) -> Float
  gc_content(seq)
end
```

Doc comments (`##`) attach to the immediately following `fn`, `let`, or
`type` declaration and are preserved in the AST for tooling and
auto-generated documentation.
