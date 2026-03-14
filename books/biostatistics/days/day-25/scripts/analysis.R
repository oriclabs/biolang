# Day 25: Statistical Visualization Best Practices

data <- read.csv("viz_data.csv")

par(mfrow = c(2, 3))

# 1. Histogram with density
hist(data$value1, breaks = 20, probability = TRUE, col = "steelblue",
     main = "Distribution", xlab = "Value 1")
lines(density(data$value1), col = "red", lwd = 2)

# 2. Box plot by group
boxplot(value1 ~ group, data = data, col = c("lightblue", "lightyellow"),
        main = "Box Plot by Group")

# 3. Scatter with regression
plot(data$value1, data$value2, col = as.factor(data$group), pch = 16,
     main = "Scatter Plot", xlab = "Value 1", ylab = "Value 2")
legend("topleft", levels(as.factor(data$group)), col = 1:2, pch = 16)

# 4. Violin-style (stripchart + boxplot)
boxplot(value1 ~ category, data = data, col = "lightgreen", main = "By Category")
stripchart(value1 ~ category, data = data, vertical = TRUE, add = TRUE,
           method = "jitter", pch = 16, cex = 0.5)

# 5. Bar plot with error bars
means <- tapply(data$value1, data$group, mean)
sems <- tapply(data$value1, data$group, function(x) sd(x)/sqrt(length(x)))
bp <- barplot(means, col = c("steelblue", "coral"), main = "Mean +/- 95% CI",
              ylim = c(0, max(means + 2*sems) * 1.1))
arrows(bp, means - 1.96*sems, bp, means + 1.96*sems, angle = 90, code = 3, length = 0.1)

# 6. QQ plot
qqnorm(data$value1, main = "Q-Q Plot", pch = 16, cex = 0.7)
qqline(data$value1, col = "red")
