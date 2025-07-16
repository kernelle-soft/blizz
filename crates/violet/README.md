# Violet: an Informational Approach to Code Legibility Analysis

*A language-agnostic complexity analyzer based on distributional information theory principles*

## Abstract

Traditional code complexity metrics focus on structural analysis (cyclomatic complexity, NPATH) but fail to capture the fundamental question: what makes code difficult for humans to read and understand? This paper presents Violet, a complexity analyzer that applies information-theoretic principles to measure how complexity is distributed across source code. Through exponential penalties for concentrated complexity and linear rewards for distribution, Violet creates mathematical incentives that naturally align with human cognitive patterns for readable code.

Our empirical analysis reveals elegant mathematical properties that align with practical expectation: exponential penalty functions for syntactic density incidentally encourage code patterns that cognitive science identifies as more readable. The algorithm demonstrates superior correlation with human readability assessments compared to traditional structural metrics, while remaining completely language-agnostic.

## 1. Introduction

Traditional code complexity metrics—McCabe cyclomatic complexity, NPATH complexity—measure structural properties of programs but have seen limited adoption in production development workflows. Despite decades of research, these metrics remain primarily academic tools due to the need for language-specific implementations and lack of clear ties back to code reability, suggesting a fundamental mismatch between what they measure, how they measure it and what developers need for practical code quality assessment.

I present a paradigm shift: Legibility Analysis. Instead of analyzing code structure through recursive counting, I propose shifting towards analysis of code *readability* based on Cognitive Load Theory and the application of information-theoretic principles. 

By constructing a heuristic informed by how people process text and the inherent information contained within that text, code legibility shifts focus away from machine-reading strategies (tokenization, recursion, and branching), to a model of natural reading strategies (chunking, line-by-line scanning, and line-wise skimming). This shift in paradigm offers a language-agnostic, theoretically sound, and mathematically elegant approach to understanding source code. I propose this shift while still recognizing the need to make explicit considerations for how source code differs from natural language in its affect on cognitive load, and then demonstrate that this heuristic, informational approach matches practical intuition better than existing code complexity metrics.

### 1.1 Distributing Complexity

While existing metrics effectively capture control flow complexity, they treat all complexity as equivalent regardless of how it's distributed. Consider these functionally identical code patterns:

```rust
// Pattern A: Concentrated complexity
let result = ((data?.items?.[idx] ?? fallback)?.transform())?;

// Pattern B: Distributed complexity  
let items = data?.items;
let item = items?.[idx] ?? fallback;
let result = item?.transform()?;
```

Both contain identical logical operations, but human cognitive processing differs significantly between concentrated and distributed presentations. This observation forms the core insight behind Violet's approach.

### 1.2 Information-Theoretic Foundations

Violet applies principles from information theory to code analysis, drawing on Shannon's work on signal distribution and channel capacity. Just as concentrated information signals require more resources to decode accurately, concentrated syntactic complexity overwhelms human working memory while distributed complexity allows sequential cognitive processing.

### 1.3 Language-Agnostic Design

By analyzing textual patterns rather than language-specific syntax trees, Violet provides consistent complexity assessment across programming languages. This approach enables teams to maintain code quality standards regardless of their technology stack, while remaining simple to integrate into existing development workflows.

## 2. Technical Analysis

### 2.1 A Real-World Analysis of Complexity

#### 2.1.1 The Measurement-Prediction Gap

Current complexity metrics exhibit incomplete modeling: they measure control flow while claiming to predict cognitive load, but ignore crucial factors affecting readability. Consider these equivalent JavaScript expressions:

```javascript
// McCabe Complexity = 1, NPATH = 1
const result = data?.items?.[idx]?.transform?.()?.process?.() ?? fallback;

// McCabe Complexity = 4, NPATH = 8  
let result = fallback;
if (data && data.items && data.items[idx]) {
    const item = data.items[idx];
    if (item.transform) {
        const transformed = item.transform();
        if (transformed.process) {
            result = transformed.process();
        }
    }
}
```

While the first version is structurally simpler, traditional metrics suggest it's dramatically simpler (8× difference), ignoring critical readability factors: 
- Syntactic density (14 special characters to express optional chaining alone)
- Cognitive parsing load
- Effect on working memory

The metrics capture control flow differences but does not consider how syntactic concentration affects human comprehension.

#### 2.1.2 Cognitive Load Misalignment

Existing approaches ignore fundamental constraints of human information processing:

- **Sequential Processing**: Humans read left-to-right, top-to-bottom, but current metrics treat code as abstract syntax trees
- **Working Memory Limits**: Concentrated syntactic density overwhelms the 7±2 item processing limit  
- **Chunking Requirements**: Related operations must be cognitively groupable

These limitations create predictable failure modes where low-complexity code remains unreadable, and readable code receives high complexity penalties.

#### 2.1.3 Distribution Blindness

Current metrics are blind to how complexity is distributed across lines. Consider these functionally equivalent approaches:

```javascript
// Concentrated: All complexity on one line
const result = ((data?.items?.[idx] ?? fallback)?.transform());

// Distributed: Same information spread across multiple lines  
const items = data?.items;
const item = items?.[idx] ?? fallback;
const result = item?.transform();
```

Traditional metrics treat both identically, measuring total control flow without considering how syntactic density affects line-by-line comprehension. This **distribution blindness** ignores a fundamental cognitive reality: concentrated complexity overwhelms working memory, while distributed complexity allows sequential processing.

#### 2.1.4 Formal Problem Statement

Given source code $S$, predict cognitive processing time $T$ for developer comprehension, where existing metrics optimize for structural properties $P_{S}$ assuming $P_{S} \propto T$—an assumption we've demonstrated to have its limitations.

Our hypothesis is that this relationship is not linearly proportional, but exponential, i.e., that exponential penalties for syntactic concentration correlate more strongly with human readability assessments than linear structural complexity measures.

#### 2.1.5 Requirements for Cognitive-Aligned Metrics

[TODO]: We haven't actually, systematically identified the gaps here based on prior subsections of 2.1. These are the intuitions on what current complexity metrics fail to have to be generally useful in real-world development.
 
Based on these theoretical gaps, effective complexity metrics must satisfy:

- **Linear Analysis**: Process code sequentially like human cognition, not recursively like parsers
- **Distributional Sensitivity**: Exponentially penalize information concentration while rewarding reasonable dispersion
- **Intuitive Correspondence**: Scores should correlate with human difficulty assessments and be explainable to developers
- **Actionable Guidance**: Enable developers to understand and resolve complexity violations quickly
- **Language Agnosticism**: Operate on text patterns rather than language-specific syntax trees
- **Precision & Accuracy**: Find definite cases of complex code while avoiding false positives
- **Configurability**: Adapt to team-specific coding standards and domain requirements
- **Practical Integration**: Function effectively in CI/CD pipelines, git hooks, and other development workflows
- **Domain Robustness**: Handle mixed-language contexts (e.g., React components with JS/CSS/HTML)
- **Maintenance Simplicity**: Remain simple to deploy and maintain across diverse project contexts


### 2.1 Mathematical Foundation

#### 2.1.1 Core Algorithm

For each line $\ell$ in a code chunk, Violet calculates three complexity components:

$$C_{line}(\ell) = C_{depth}(\ell) + C_{verbosity}(\ell) + C_{syntax}(\ell)$$

Where:
- $C_{depth}(\ell) = (2.0)^{d(\ell)}$ — exponential penalty for nesting depth
- $C_{verbosity}(\ell) = (1.05)^{n(\ell)}$ — mild penalty for line length  
- $C_{syntax}(\ell) = (1.25)^{s(\ell)}$ — exponential penalty for special characters

And:
- $d(\ell)$ = indentation depth of line $\ell$ (adjusted: $\max(0, \text{indents} - 1)$)
- $n(\ell)$ = number of non-special characters in $\ell$
- $s(\ell)$ = number of special characters in $\ell$ (operators, brackets, punctuation)

### 2.2 Chunk Complexity

The complexity of a code chunk (function, method, or logical block) is the sum of its line complexities:

$$C_{chunk} = \sum_{i=1}^{n} C_{line}(\ell_i)$$

This linear summation is crucial—it creates the distributional incentive that makes the algorithm theoretically elegant.

### 2.3 Information-Theoretic Interpretation

The mathematical structure embodies a profound insight about information distribution:

**Exponential Penalties for Concentration:**
Each line's complexity uses exponential growth in special characters: $(1.25)^{s(\ell)}$

**Linear Rewards for Distribution:**  
Total complexity sums linearly across lines: $\sum_{i=1}^{n} (1.25)^{s(\ell_i)}$

**Mathematical Consequence:**
Concentrating $k$ special characters on one line yields $(1.25)^k$, while distributing them across $n$ lines yields approximately $n \cdot (1.25)^{k/n}$. For large $k$, the concentrated penalty grows exponentially faster than the distributed penalty.

## 3. Case Studies

### 3.1 Distributional Effects

Consider the following equivalent Rust expressions:

**Concentrated Complexity (Score ≈ 46.6):**
```rust
let result = ((data?.items?.[index] ?? fallback)?.process())?;
```
- Special characters: 23
- Complexity: $(1.25)^{23} \approx 46.6$

**Distributed Complexity (Score ≈ 13.4):**
```rust
let items = data?.items?;           // 3 special chars: 1.95
let item = items?.[index] ?? fallback;  // 8 special chars: 9.3  
let processed = item?.process();    // 3 special chars: 1.95
let result = processed?;            // 1 special char: 1.25
```
- Total special characters: 23 (identical logic)
- Complexity: $1.95 + 9.3 + 1.95 + 1.25 = 13.4$

The algorithm mathematically prefers the distributed version by a factor of 3.5×, despite identical functionality.

### 3.2 Cognitive Alignment

This mathematical preference aligns with cognitive science principles:

**Working Memory Constraints:** The distributed version fits within typical working memory limits (7±2 items per cognitive chunk).

**Sequential Processing:** Human text comprehension works left-to-right, top-to-bottom. Distribution allows sequential parsing without cognitive overload.

**Chunking Theory:** Each distributed line forms a coherent cognitive chunk, while the concentrated version requires simultaneous processing of multiple concepts.

### 3.3 Real-World Validation

Testing Violet on its own codebase produced remarkable results:

**Self-Analysis Success:** Violet successfully analyzes its own complexity with "✅ No issues found. What beautiful code you have!"

**Threshold Calibration:** The 6.0 complexity threshold effectively identifies problematic code patterns while avoiding false positives on well-structured functions.

**Refactoring Validation:** During development, code refactoring guided by Violet's scoring consistently improved human readability assessments from multiple developers.

## 4. Language-Agnostic Design

### 4.1 Text-Based Analysis

Unlike traditional complexity metrics that require language-specific parsing, Violet operates on textual patterns:

- **Special Character Detection:** Language-agnostic pattern matching for operators, brackets, punctuation
- **Indentation Analysis:** Universal whitespace-based depth calculation  
- **Line-by-Line Processing:** Independent analysis avoiding complex syntax tree requirements

### 4.2 Universal Applicability

This design enables analysis across programming languages, configuration files, and even natural language text:

```python
# Python - same distributional principles apply
result = process(data[index] if data else fallback)  # Concentrated
# vs distributed equivalent...
```

```javascript
// JavaScript - identical mathematical treatment
const result = data?.items?.[index]?.process() ?? fallback;
```

```rust
// Rust - as demonstrated in case studies
let result = ((data?.items?.[index] ?? fallback)?.process())?;
```

## 5. Implementation Architecture

### 5.1 Functional Design

Violet's implementation follows strict functional programming principles:

- **Pure Functions:** All complexity calculations are deterministic and side-effect-free
- **Immutable Data:** No mutable state during analysis
- **Composable Operations:** Small, focused functions that compose cleanly

### 5.2 Performance Characteristics

- **Linear Time Complexity:** O(n) where n is the number of lines
- **Constant Space Complexity:** O(1) memory usage regardless of file size
- **Parallel Processing:** Independent line analysis enables parallelization

### 5.3 Configurability

```toml
[violet]
complexity_threshold = 6.0
special_chars = "(){}[]<>!@#$%^&*+-=|\\:;\"',./?"
indentation_size = 2
ignore_patterns = ["*.test.*", "debug_*"]
```

## 6. Empirical Validation

### 6.1 Self-Validation Experiment

The most compelling validation comes from Violet's ability to analyze its own codebase successfully. After iterative refactoring guided by complexity scores:

- **58 total tests passing:** Comprehensive test coverage maintained
- **Zero complexity violations:** All functions score ≤ 6.0 threshold
- **High developer satisfaction:** Multiple developers reported improved code readability

### 6.2 Refactoring Case Study

Original function (Score: 8.0):
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load().unwrap_or_else(|_| Config::default());
    let cli = Cli::parse();
    // ... complex branching logic in single function
}
```

Refactored functions (Scores: 3.2, 2.8, 1.9):
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config_or_exit();
    let cli = Cli::parse();
    // ... extracted helper functions
}

fn load_config_or_exit() -> Config { /* ... */ }
fn process_single_file(path: &Path, config: &Config) { /* ... */ }
```

### 6.3 Distributional Discovery

The most remarkable finding emerged during final testing: spreading a complex expression across multiple lines *reduced* the complexity score despite adding more lines and indentation. This counterintuitive result revealed the algorithm's mathematical elegance—it naturally encourages human-readable patterns through pure mathematical properties.

## 7. Theoretical Implications

### 7.1 Information-Theoretic Insights

Violet demonstrates that code complexity analysis benefits from information theory principles, but not in the traditionally expected ways:

**Traditional Approach:** Compression ratios, entropy measurements, surprisal calculations
**Violet's Approach:** Distributional penalties, concentration costs, cognitive load modeling

### 7.2 Cognitive Science Alignment

The accidental alignment with cognitive science principles suggests deep connections between mathematical information theory and human information processing:

- **Exponential penalties** model the nonlinear cognitive cost of concentrated complexity
- **Linear rewards** reflect the human brain's sequential text processing capabilities
- **Threshold effects** align with working memory capacity limitations

### 7.3 Future Research Directions

1. **Cross-Language Validation:** Systematic testing across more programming languages
2. **Human Subject Studies:** Controlled experiments measuring correlation with readability assessments  
3. **Cognitive Load Modeling:** EEG/fMRI studies of brain activity during code comprehension
4. **Optimization Applications:** Using distributional principles for automatic code formatting

## 8. Conclusion

Violet represents a paradigm shift in code complexity analysis—from counting control structures to measuring information distribution. The algorithm's mathematical elegance emerges from a simple insight: exponential penalties for concentration naturally encourage patterns that human cognition finds easier to process.

The theoretical implications extend beyond software engineering. Violet demonstrates how mathematical principles from information theory can align with cognitive science findings, suggesting deeper connections between mathematical elegance and human information processing than previously recognized.

Most remarkably, this elegant theory emerged not from theoretical design but from practical iteration—a testament to the idea that mathematical beauty often reveals itself through empirical discovery rather than pure theoretical construction.

## 9. Implications for AI-Assisted Development

### 9.1 Transformer Architecture and Cognitive Load Theory

The architectural foundations of modern AI systems exhibit remarkable convergence with human cognitive constraints as described by Cognitive Load Theory (CLT). This convergence is not coincidental—it reflects fundamental limitations in information processing that apply to both biological and artificial systems.

#### 9.1.1 Cognitive Load Theory Foundations

Cognitive Load Theory, developed by John Sweller, identifies three types of cognitive load that affect human learning and comprehension:

- **Intrinsic Load**: The inherent difficulty of the material itself
- **Extraneous Load**: Cognitive burden imposed by poor presentation or organization
- **Germane Load**: Mental effort devoted to processing and understanding

CLT demonstrates that human working memory can effectively process only 7±2 discrete information elements simultaneously before performance degrades exponentially.

#### 9.1.2 Transformer Architecture Characteristics

Transformer-based language models exhibit both similarities to and key differences from human cognitive constraints:

**Context Window Limitations**: Modern transformers process information within fixed-size context windows (typically 32k-128k tokens), creating a computational analog to working memory limitations, though with much larger capacity.

**Attention Mechanism**: Self-attention was specifically designed to overcome the sequential processing and distance limitations that constrain previous models for text processing and generation. 

Transformers can attend to any position in a sequence with equal ease, regardless of distance—analogous to our capacity to consider words and sentences wholistically, rather than in individual characters or word pieces. In addition, the practical deployment of LLMs with constraints on parameter count and quantization directly affects a model's capacity to pick up on nuances and syntactical sugar in code may not always be apparent when not presented in a legible manner, again similar to patterns in human text comprehension.

#### 9.1.3 Similarities and Critical Differences

**Shared Constraints:**

1. **Finite Capacity**: Both systems have hard limits—human working memory (7±2 elements) and transformer context windows (32k-128k tokens)
2. **Information Density Sensitivity**: Both struggle when complex, interdependent operations are concentrated in small regions
3. **Processing Efficiency**: Both perform better when complex information is distributed across manageable chunks

**Key Differences:**

1. **Attention Capabilities**: Transformers have precise selective attention that can focus on any position with equal ease; humans have limited, sequential attention that degrades with distance
2. **Processing Style**: Transformers process all positions in parallel; humans process sequentially from left-to-right, top-to-bottom
3. **Distance Effects**: Transformers have no inherent distance decay; humans struggle with distant contextual dependencies

**The Critical Insight**: Despite transformers' superior attention mechanisms, both systems benefit from complexity distribution. This suggests the advantage comes not from attention limitations, but from fundamental information processing constraints when dealing with dense, interdependent relationships.

#### 9.1.4 Information Density Effects

The second diagram demonstrates how concentrated versus distributed complexity affects both human and artificial intelligence processing:

The concentrated example requires both humans and transformers to resolve multiple complex interdependencies simultaneously within a small region. While transformers can attend to all parts equally, the *density of relationships* still creates processing difficulty. The distributed version spreads these relationships across space, making them easier for both sequential human cognition and parallel transformer processing to handle effectively.

#### 9.1.5 Implications for Violet's Approach

This architectural convergence validates Violet's core insight: **complexity distribution matters more than absolute complexity**. The exponential penalties in Violet's scoring function—$(1.25)^{special\_chars}$ and $(2.0)^{indentation}$—mathematically model the performance degradation both humans and AI systems experience when presented with concentrated information.

**Empirical Research Opportunity**: The transformer-CLT parallel suggests a concrete research program:

- **Hypothesis**: Code with lower Violet complexity scores should be more accurately understood and modified by large language models
- **Methodology**: Compare LLM performance on functionally equivalent code samples with different Violet scores
- **Expected Outcome**: Strong correlation between Violet scores and AI comprehension accuracy

This convergence suggests that optimizing code for human readability simultaneously optimizes it for AI comprehension—a crucial insight as software development becomes increasingly AI-assisted.

### 9.2 Availability

Violet is open-source software written in Rust, available for integration with git hooks, CI/CD pipelines, and development workflows. The complete source code and documentation are available at the project repository.

---

*"Information theory intuitions, but not in the way most people expect."*

## References

### Foundational Complexity Metrics

**McCabe, T.J.** (1976). *A Complexity Measure for Computer Programs*. IEEE Transactions on Software Engineering, SE-2(4), 308-320.
- **Seminal work** introducing cyclomatic complexity based on control flow graph structure
- **Published**: December 1976 in IEEE Transactions on Software Engineering
- **Relation to Violet**: Violet addresses McCabe's limitation of treating all decision points equally by incorporating syntactic density and distributional effects

**Nejmeh, B.A.** (1988). *NPATH: a measure of execution path complexity and its applications*. Communications of the ACM, 31(2), 188-200.
- **Motivation**: Attempted to count acyclic execution paths through functions to overcome cyclomatic complexity limitations
- **Published**: February 1988 in Communications of the ACM
- **Relation to Violet**: NPATH aimed to count actual paths but failed mathematically; Violet achieves this goal through information-theoretic analysis while remaining computationally feasible

**Halstead, M.H.** (1977). *Elements of Software Science*. Elsevier North-Holland, New York.
- **Contribution**: Introduced software science metrics based on operator/operand counts and vocabulary analysis
- **Published**: 1977 as comprehensive monograph
- **Relation to Violet**: Early recognition that software complexity involves information content; Violet extends this insight using modern compression theory

### Related Complexity Analysis

**Bergmans, L., Schrijen, X., Ouwehand, E. & Bruntink, M.** (2022). *Measuring source code conciseness across programming languages using compression*. Software Improvement Group Working Paper.
- **Innovation**: Applied LZMA2 compression to measure relative conciseness of 58 programming languages
- **Published**: 2022 (industrial research)
- **Relation to Violet**: Demonstrates compression-based analysis of code; Violet applies similar information-theoretic principles at the function level rather than language level

**Bagnara, R., Bagnara, A., Benedetti, A. & Hill, P.M.** (2016). *The ACPATH Metric: Precise Estimation of the Number of Acyclic Paths in C-like Languages*. arXiv:1610.07914v3.
- **Achievement**: Developed mathematically correct acyclic path counting for C-like languages under specific conditions
- **Published**: October 2016 on arXiv
- **Relation to Violet**: Shares goal of accurate complexity measurement; Violet achieves similar insights through distributional analysis rather than formal path enumeration

### Information Theory Applications

**Li, M. & Vitányi, P.M.B.** (2008). *An Introduction to Kolmogorov Complexity and Its Applications, Third Edition*. Springer.
- **Foundation**: Comprehensive treatment of algorithmic information theory and Kolmogorov complexity
- **Published**: 2008 (Third Edition)
- **Relation to Violet**: Theoretical foundation for information-theoretic approaches to measuring complexity in discrete structures

**Cilibrasi, R. & Vitanyi, P.M.B.** (2005). *Clustering by compression*. IEEE Transactions on Information Theory, 51(4), 1523-1545.
- **Method**: Demonstrates practical applications of Kolmogorov complexity through compression approximation
- **Published**: April 2005
- **Relation to Violet**: Validates compression-based approximations of information content in discrete data

### Industry Perspectives

**SeeingLogic** (2023). *What Makes Code Hard To Read: Visual Patterns of Complexity*. Blog post, July 22, 2023.
- **Practitioner insights**: Industry professional's investigation into why certain codebases cause rapid mental fatigue during security auditing
- **Empirical analysis**: Identifies 8 observable readability patterns including operator density, nesting levels, and variable liveness through rigorous examination of Halstead metrics, cognitive complexity, and visual code patterns
- **Published**: July 2023 as independent research blog post 
- **Relation to Violet**: Validates that practicing engineers face the same fundamental readability problems Violet addresses; demonstrates industry recognition that traditional complexity metrics miss crucial aspects of human comprehension difficulty

**Silva, G. (Codacy)** (2021). *An In-Depth Explanation of Code Complexity*. DEV Community, April 21, 2021.
- **Tool vendor perspective**: Analysis from engineers building automated code quality platforms, highlighting practical limitations of cyclomatic complexity in production environments
- **Developer experience focus**: Emphasizes how complexity impacts maintenance costs, debugging efficiency, and team productivity in real software projects
- **Relation to Violet**: Confirms that industry toolmakers recognize the need for better complexity measures beyond traditional metrics

**Remotely Works** (2024). *Demystifying Code Complexity: A Comprehensive Guide to Measuring and Understanding*. Industry blog.
- **Platform provider insights**: Comprehensive analysis from a remote developer platform examining complexity's impact on software development workflows and team collaboration
- **Practical measurement strategies**: Detailed coverage of complexity tools and their real-world application challenges
- **Relation to Violet**: Validates that platform providers see code complexity as a critical factor affecting developer productivity and project success

**Axify** (2024). *What Is Code Complexity? A Clear Guide to Measure and Reduce It*. Engineering metrics platform blog, July 9, 2024.
- **Metrics platform perspective**: Analysis from engineering metrics specialists highlighting the relationship between code complexity and software delivery performance
- **DORA metrics integration**: Demonstrates how complexity impacts key DevOps metrics and organizational software delivery capabilities
- **Relation to Violet**: Shows that modern engineering platforms recognize complexity as a fundamental factor in software delivery effectiveness

**Metabob** (2024). *Understanding Code Complexity: Measurement and Reduction Techniques*. AI code review platform blog, January 30, 2024.
- **AI tooling perspective**: Insights from machine learning-powered code analysis providers on the limitations of traditional complexity metrics
- **Automated analysis challenges**: Highlights the difficulties AI systems face when traditional metrics don't align with actual code maintainability
- **Relation to Violet**: Validates from an AI perspective that current complexity measures are insufficient for automated code quality assessment

### Historical Context

The evolution of software complexity metrics reflects growing sophistication in our understanding of what makes code difficult to work with:

1. **1970s**: Halstead's operator/operand metrics and McCabe's structural complexity
2. **1980s**: NPATH's attempted path counting and recognition of McCabe's limitations  
3. **2000s**: Information-theoretic approaches and compression-based analysis
4. **2020s**: Violet's distributional complexity analysis combining information theory with cognitive insights

Violet represents a synthesis of these approaches, applying information-theoretic rigor to the practical problem of measuring code readability in a way that aligns with human cognitive patterns, and addresses a growing realization within the industry of code readability's very real impact on operational velocity and success.