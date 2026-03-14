"""AI-assisted bioinformatics analysis pipeline."""

import csv
import os
import sys
from datetime import datetime
from pathlib import Path

try:
    from openai import OpenAI
except ImportError:
    print("pip install openai")
    sys.exit(1)


def detect_client():
    """Auto-detect LLM provider and return client + model."""
    if key := os.environ.get("ANTHROPIC_API_KEY"):
        model = os.environ.get("ANTHROPIC_MODEL", "claude-sonnet-4-20250514")
        client = OpenAI(api_key=key, base_url="https://api.anthropic.com/v1/")
        return client, model, "anthropic"
    if key := os.environ.get("OPENAI_API_KEY"):
        model = os.environ.get("OPENAI_MODEL", "gpt-4o")
        client = OpenAI(api_key=key)
        return client, model, "openai"
    if model := os.environ.get("OLLAMA_MODEL"):
        base = os.environ.get("OLLAMA_BASE_URL", "http://localhost:11434")
        client = OpenAI(api_key="ollama", base_url=f"{base}/v1")
        return client, model, "ollama"
    if base := os.environ.get("LLM_BASE_URL"):
        key = os.environ.get("LLM_API_KEY", "")
        model = os.environ.get("LLM_MODEL", "default")
        client = OpenAI(api_key=key, base_url=base)
        return client, model, "compatible"
    print("No LLM provider configured. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, or OLLAMA_MODEL.")
    sys.exit(1)


def chat(client, model, prompt, context=None, system=None):
    """Send a chat request with optional context."""
    if system is None:
        system = (
            "You are a bioinformatics assistant. Be concise and practical. "
            "When discussing genes or variants, be specific about evidence."
        )
    user_msg = prompt
    if context is not None:
        if isinstance(context, dict):
            ctx_str = "\n".join(f"{k}: {v}" for k, v in context.items())
        elif isinstance(context, list):
            ctx_str = "\n".join(str(item) for item in context)
        else:
            ctx_str = str(context)
        user_msg = f"{prompt}\n\n--- Context ---\n{ctx_str}"

    try:
        response = client.chat.completions.create(
            model=model,
            max_tokens=4096,
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg},
            ],
        )
        return response.choices[0].message.content
    except Exception as e:
        return f"[AI unavailable: {e}]"


def read_tsv(path):
    """Read a TSV file into a list of dicts."""
    rows = []
    with open(path, newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            for key in ("log2fc", "pvalue", "padj", "basemean"):
                if key in row:
                    row[key] = float(row[key])
            rows.append(row)
    return rows


def main():
    client, model, provider = detect_client()
    print(f"Using provider: {provider}, model: {model}")

    # Load DE genes
    de_genes = read_tsv("data/de_genes.tsv")
    sig = [g for g in de_genes if g["padj"] < 0.05]
    up = [g for g in sig if g["log2fc"] > 1.0]
    down = [g for g in sig if g["log2fc"] < -1.0]

    up_names = sorted([g["gene"] for g in up], key=lambda g: next(
        x["padj"] for x in up if x["gene"] == g
    ))
    down_names = sorted([g["gene"] for g in down], key=lambda g: next(
        x["padj"] for x in down if x["gene"] == g
    ))

    stats = {
        "total_genes": len(de_genes),
        "significant": len(sig),
        "upregulated": len(up),
        "downregulated": len(down),
        "mean_abs_fc": sum(abs(g["log2fc"]) for g in sig) / len(sig) if sig else 0,
        "top_up": ", ".join(up_names),
        "top_down": ", ".join(down_names),
    }

    interpretation = chat(
        client, model,
        "Write a results paragraph for a manuscript describing this differential "
        "expression analysis of breast cancer tumor vs normal tissue. Include: "
        "(1) overall summary, (2) notable upregulated pathways, (3) notable "
        "downregulated pathways, (4) suggested follow-up experiments.",
        stats,
    )

    # Variant annotation
    variants = read_tsv("data/variants.tsv")
    known_oncogenes = [
        "TP53", "BRCA1", "BRCA2", "KRAS", "EGFR",
        "PIK3CA", "BRAF", "MYC", "RB1", "PTEN",
    ]

    annotated = []
    for v in variants:
        if v["consequence"] == "synonymous_variant":
            continue
        is_cancer = v["gene"] in known_oncogenes
        cons = v["consequence"]
        if cons == "stop_gained" or "frameshift" in cons or "splice" in cons:
            severity = "high"
        elif cons == "missense_variant":
            severity = "moderate"
        else:
            severity = "low"

        context = {
            "gene": v["gene"],
            "position": f"{v['chrom']}:{v['pos']}",
            "change": f"{v['ref']}>{v['alt']}",
            "consequence": cons,
            "cancer_gene": is_cancer,
            "computed_severity": severity,
        }
        ai_interp = chat(
            client, model,
            "Briefly interpret this variant's clinical significance in 2-3 sentences.",
            context,
        )
        annotated.append({
            "gene": v["gene"],
            "variant": f"{v['chrom']}:{v['pos']}{v['ref']}>{v['alt']}",
            "consequence": cons,
            "severity": severity,
            "cancer_gene": is_cancer,
            "ai_interpretation": ai_interp,
        })

    # Sequence features
    seq_features = []
    current_id = None
    current_seq = ""
    with open("data/sequences.fasta") as f:
        for line in f:
            line = line.strip()
            if line.startswith(">"):
                if current_id:
                    gc = (current_seq.count("G") + current_seq.count("C")) / len(current_seq) if current_seq else 0
                    seq_features.append({
                        "id": current_id, "length": len(current_seq),
                        "gc_content": gc, "at_rich": gc < 0.4, "gc_rich": gc > 0.6,
                    })
                current_id = line[1:]
                current_seq = ""
            else:
                current_seq += line
    if current_id and current_seq:
        gc = (current_seq.count("G") + current_seq.count("C")) / len(current_seq)
        seq_features.append({
            "id": current_id, "length": len(current_seq),
            "gc_content": gc, "at_rich": gc < 0.4, "gc_rich": gc > 0.6,
        })

    seq_summary = {
        "num_sequences": len(seq_features),
        "mean_gc": sum(s["gc_content"] for s in seq_features) / len(seq_features) if seq_features else 0,
        "at_rich_count": sum(1 for s in seq_features if s["at_rich"]),
        "gc_rich_count": sum(1 for s in seq_features if s["gc_rich"]),
    }

    seq_interp = chat(
        client, model,
        "These are sequence composition statistics from genomic regions. "
        "What biological significance might the GC content distribution suggest? "
        "Consider promoter regions, coding vs non-coding, CpG islands. "
        "Be brief (3-4 sentences).",
        seq_summary,
    )

    # Write report
    Path("data/output").mkdir(parents=True, exist_ok=True)
    with open("data/output/ai_report.txt", "w") as f:
        f.write("# AI-Assisted Analysis Report\n")
        f.write(f"# Generated: {datetime.now().strftime('%Y-%m-%d %H:%M')}\n")
        f.write("# NOTE: AI interpretations require expert verification\n\n")
        f.write("## Differential Expression Summary\n")
        f.write(f"Total genes tested: {stats['total_genes']}\n")
        f.write(f"Significant (padj < 0.05): {stats['significant']}\n")
        f.write(f"Upregulated (log2FC > 1): {stats['upregulated']}\n")
        f.write(f"Downregulated (log2FC < -1): {stats['downregulated']}\n")
        f.write(f"Mean absolute fold change: {stats['mean_abs_fc']:.2f}\n\n")
        f.write("## Top Upregulated Genes\n")
        f.write(f"{stats['top_up']}\n\n")
        f.write("## Top Downregulated Genes\n")
        f.write(f"{stats['top_down']}\n\n")
        f.write("## AI Interpretation (DE Analysis)\n")
        f.write(f"{interpretation}\n\n")
        f.write("## Variant Annotations\n")
        for v in annotated:
            f.write(f"  {v['gene']} ({v['variant']}) - {v['consequence']} - severity:{v['severity']}\n")
            f.write(f"    AI: {v['ai_interpretation']}\n\n")
        f.write("## Sequence Composition Analysis\n")
        f.write(f"Sequences analyzed: {seq_summary['num_sequences']}\n")
        f.write(f"Mean GC content: {seq_summary['mean_gc']:.4f}\n")
        f.write(f"AT-rich regions: {seq_summary['at_rich_count']}\n")
        f.write(f"GC-rich regions: {seq_summary['gc_rich_count']}\n\n")
        f.write("## AI Interpretation (Sequence Composition)\n")
        f.write(f"{seq_interp}\n\n")
        f.write("## Disclaimer\n")
        f.write("All AI-generated interpretations in this report were produced by a large\n")
        f.write("language model and have NOT been verified against primary databases.\n")
        f.write("Do not use for clinical decision-making without expert review.\n")

    print(f"Report written to data/output/ai_report.txt")


if __name__ == "__main__":
    main()
