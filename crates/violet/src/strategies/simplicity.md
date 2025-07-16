# Information-Theoretic Simplicity Scoring

## The Algorithm That Accidentally Discovered Cognitive Elegance

Violet's complexity algorithm embodies a profound insight about information distribution that aligns mathematical principles with human cognitive patterns.

### Core Algorithm

```rust
fn calculate_line_complexity(line: &str, indents: f64) -> (f64, f64, f64) {
    let special_chars = count_special_characters(line);
    let non_special_chars = (line.trim().len() as f64) - special_chars;

    let verbosity_component = 1.05_f64.powf(non_special_chars);
    let syntactic_component = 1.25_f64.powf(special_chars);
    let depth_component = 2.0_f64.powf(indents);

    (depth_component, verbosity_component, syntactic_component)
}
```

### Information-Theoretic Insights

**Exponential Penalties for Concentration:**
- Each line's complexity is calculated independently, then summed
- Special characters use exponential growth: `(1.25)^special_chars`
- Result: Exponential penalty for concentrating complexity, linear reward for distribution

**Mathematical Beauty in Practice:**
- Cramming 23 special characters in one line: `(1.25)^23 ≈ 46.6`
- Spreading across 4 lines: `1.25 + 9.3 + 1.25 + 1.56 ≈ 13.4`
- The algorithm mathematically prefers distributed complexity over concentrated complexity

### Cognitive Science Alignment

This approach accidentally aligns with cognitive science principles about code readability:

- **Chunking Theory**: Human working memory processes information in chunks
- **Cognitive Load**: Concentrated complexity overwhelms cognitive processing
- **Reading Patterns**: Distributed information is easier for humans to parse sequentially

### Practical Excellence

- **Language-agnostic**: Based on text patterns, not syntax trees
- **Self-validating**: Violet successfully analyzes its own codebase
- **Threshold-calibrated**: 6.0 threshold hits the sweet spot for practical development
- **Never-nester friendly**: Encourages functional programming patterns naturally
- **Explainable**: Every score can be traced to specific textual patterns

### The Elegant Discovery

The most remarkable aspect? This mathematical elegance emerged from practical iteration, not theoretical design. The algorithm encourages human-readable code not through rules or prescriptions, but through fundamental mathematical properties that align with how our brains process complex information.

*"Information theory intuitions, but not in the way most people expect."*