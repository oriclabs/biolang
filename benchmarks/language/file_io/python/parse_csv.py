"""File I/O: Parse CSV (5K rows)"""
import csv
with open("data/samples.csv") as f:
    reader = csv.DictReader(f)
    rows = list(reader)
print(f"Rows: {len(rows)}")
print(f"Columns: {len(rows[0])}")
