# Day 17: Survival Analysis
import pandas as pd
from lifelines import KaplanMeierFitter, CoxPHFitter
from lifelines.statistics import logrank_test

data = pd.read_csv("breast_cancer_survival.csv")

# Kaplan-Meier: TP53 mutated vs wild-type
kmf = KaplanMeierFitter()
print("=== Kaplan-Meier by TP53 Status ===")
for label, group in data.groupby("tp53_mutated"):
    kmf.fit(group["time_months"], group["event"], label=f"TP53={'Mut' if label else 'WT'}")
    median_surv = kmf.median_survival_time_
    print(f"  TP53={'Mut' if label else 'WT'}: median={median_surv:.1f} months")

# Log-rank test
tp53_mut = data[data.tp53_mutated == 1]
tp53_wt = data[data.tp53_mutated == 0]
lr = logrank_test(tp53_mut.time_months, tp53_wt.time_months,
                  tp53_mut.event, tp53_wt.event)
print(f"Log-rank p-value: {lr.p_value:.4f}")

# Cox proportional hazards
print("\n=== Cox PH Model ===")
cph = CoxPHFitter()
cph.fit(data[["time_months", "event", "tp53_mutated", "age", "stage"]],
        duration_col="time_months", event_col="event")
cph.print_summary()
