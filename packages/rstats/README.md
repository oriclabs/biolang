# R-style statistics names

`rstats` is an optional compatibility facade for people who recognize R's
statistical function names. It is not an R interpreter and does not contain or
link R code. Every calculation delegates to BioLang's independently
implemented MIT-licensed statistics primitives.

```biolang
import "rstats" as r

# R's t.test default is Welch's unequal-variance test.
let comparison = r.t_test(control, treated)

# Familiar categorical and model names.
let association = r.chisq_test([[30, 24], [76, 241]])
let fitted = r.lm_fit(dose, response)
```

Supported names are deliberately limited to interfaces whose numerical
semantics are independently validated: `t_test`, `chisq_test`,
`chisq_goodness`, `fisher_test`, `wilcox_test`, `oneway_test`, `aov`,
`kruskal_test`, `tukey_test`, `lm_fit`, `cor_test`, `adjust_p`, `survfit`, and
`breslow_day`.

Options use underscore names because BioLang identifiers do not use R's dotted
argument names: `var_equal`, `conf_level`, `paired`, `exact`, and `correct`.
Alternative-hypothesis values accept familiar strings such as `two.sided`,
`less`, and `greater`. Returned records keep BioLang's explicit method and estimator labels
rather than imitating R's printed objects.

Names that would shadow a BioLang builtin use an explicit suffix or verb:
R's `lm`, `p.adjust`, and `TukeyHSD` correspond to `lm_fit`, `adjust_p`, and
`tukey_test`. This avoids ambiguous recursive resolution inside packages.
