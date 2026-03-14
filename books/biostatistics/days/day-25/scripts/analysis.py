# Day 25: Statistical Visualization Best Practices
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns

data = pd.read_csv("viz_data.csv")

fig, axes = plt.subplots(2, 3, figsize=(14, 9))

# 1. Histogram with density
axes[0, 0].hist(data.value1, bins=20, density=True, alpha=0.7, color="steelblue")
axes[0, 0].set_title("Distribution (Histogram + KDE)")
axes[0, 0].set_xlabel("Value 1")

# 2. Box plot by group
sns.boxplot(data=data, x="group", y="value1", ax=axes[0, 1])
axes[0, 1].set_title("Box Plot by Group")

# 3. Scatter with regression
for g, color in [("A", "blue"), ("B", "red")]:
    subset = data[data.group == g]
    axes[0, 2].scatter(subset.value1, subset.value2, alpha=0.5, label=g, c=color)
axes[0, 2].set_title("Scatter Plot")
axes[0, 2].legend()

# 4. Violin plot
sns.violinplot(data=data, x="category", y="value1", ax=axes[1, 0])
axes[1, 0].set_title("Violin Plot")

# 5. Bar plot with error bars
means = data.groupby("group")["value1"].agg(["mean", "sem"])
axes[1, 1].bar(means.index, means["mean"], yerr=means["sem"]*1.96, capsize=5)
axes[1, 1].set_title("Bar Plot (mean +/- 95% CI)")

# 6. QQ plot
from scipy import stats
stats.probplot(data.value1, dist="norm", plot=axes[1, 2])
axes[1, 2].set_title("Q-Q Plot")

plt.tight_layout()
plt.savefig("visualization_gallery.png", dpi=150)
print("Saved visualization_gallery.png")
