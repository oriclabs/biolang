# Day 28: Capstone — Clinical Trial Analysis
import pandas as pd
import numpy as np
from scipy import stats
from lifelines import KaplanMeierFitter, CoxPHFitter
from lifelines.statistics import logrank_test

data = pd.read_csv("clinical_trial.csv")
trt = data[data.arm == "Treatment"]
ctrl = data[data.arm == "Control"]

# 1. Demographics
print("=== Demographics ===")
for arm_name, group in [("Treatment", trt), ("Control", ctrl)]:
    print(f"\n{arm_name} (n={len(group)}):")
    print(f"  Age: {group.age.mean():.1f} +/- {group.age.std():.1f}")
    print(f"  Male: {(group.sex == 'M').mean()*100:.0f}%")
    print(f"  ECOG 0/1/2: {(group.ecog==0).sum()}/{(group.ecog==1).sum()}/{(group.ecog==2).sum()}")

# 2. Response rates
print("\n=== RECIST Response ===")
for arm_name, group in [("Treatment", trt), ("Control", ctrl)]:
    orr = ((group.recist == "CR") | (group.recist == "PR")).mean()
    print(f"  {arm_name} ORR: {orr*100:.1f}%")
_, p_orr = stats.chi2_contingency(pd.crosstab(data.arm, data.recist.isin(["CR","PR"])))[:2]
print(f"  Chi-square p: {p_orr:.4f}")

# 3. PFS
print("\n=== Progression-Free Survival ===")
lr_pfs = logrank_test(trt.pfs_months, ctrl.pfs_months, trt.pfs_event, ctrl.pfs_event)
print(f"Log-rank p: {lr_pfs.p_value:.4f}")
kmf = KaplanMeierFitter()
for label, group in [("Treatment", trt), ("Control", ctrl)]:
    kmf.fit(group.pfs_months, group.pfs_event, label=label)
    print(f"  {label} median PFS: {kmf.median_survival_time_:.1f} months")

# 4. OS
print("\n=== Overall Survival ===")
lr_os = logrank_test(trt.os_months, ctrl.os_months, trt.os_event, ctrl.os_event)
print(f"Log-rank p: {lr_os.p_value:.4f}")

# 5. Cox PH
print("\n=== Cox PH (PFS) ===")
cph = CoxPHFitter()
model_data = data[["pfs_months","pfs_event","arm","age","sex","ecog"]].copy()
model_data["arm"] = (model_data.arm == "Treatment").astype(int)
model_data["sex"] = (model_data.sex == "M").astype(int)
cph.fit(model_data, "pfs_months", "pfs_event")
cph.print_summary()

# 6. Safety
print("\n=== Safety ===")
ae_trt = trt.grade3_ae.mean()
ae_ctrl = ctrl.grade3_ae.mean()
print(f"Grade 3+ AE: Treatment={ae_trt*100:.1f}%, Control={ae_ctrl*100:.1f}%")
