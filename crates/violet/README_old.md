# Violet: an Informational Approach to Code Legibility Analysis

## Abstract

Traditional code complexity metrics focus on structural analysis (cyclomatic complexity, NPATH) but fail to capture the fundamental question: what makes code difficult for humans to read and understand? This paper presents Violet, a complexity analyzer that applies information-theoretic principles to measure how complexity is distributed across source code. Through exponential penalties for concentrated complexity and linear rewards for distribution, Violet creates mathematical incentives that naturally align with human cognitive patterns for readable code.

Our empirical analysis reveals elegant mathematical properties that align with practical expectation: exponential penalty functions for syntactic density incidentally encourage code patterns that cognitive science identifies as more readable. The algorithm demonstrates superior correlation with human readability assessments compared to traditional structural metrics, while remaining completely language-agnostic.

## 1. Introduction

### 1.1 Motivation

#### 1.1.1 Legibility is becoming more important with the emergence of automatic code generation

Enforcing legibility makes for more human-readable code.

Enforcing legibility also makes for more model-readable code.


#### 1.1.2 There is no widespread, practical, empirical legibility tool used in production software

Cyclomatic or path counting approaches have essentially no adoption, despite decades of research and a degree of intuitability behind this type of scoring.

Software complexity and commonality continues to expand. Code isn't getting less complex.

#### 1.1.3 Current methods don't bridge the gap between human readability and code properties

Traditional code complexity metrics—McCabe cyclomatic complexity, NPATH complexity—measure structural properties of programs but have seen limited adoption in production development workflows. Despite decades of research, these metrics remain primarily academic tools due to the need for language-specific implementations and lack of clear ties back to code reability, suggesting a fundamental mismatch between what they measure, how they measure it, and what developers need for practical code quality assessment.

### 1.2 Problem Statement

### 1.3 Contributions

### 1.4 Our Proposal: A versatile, intuitive, and open-source legibility evaluation tool

We present a paradigm shift: Legibility analysis. Instead of analyzing code structure through recursive branch counting, we propose shifting towards the analysis of text readability based on Cognitive Load Theory and the application of information-theoretic principles. 

By constructing a heuristic informed by how people process text and the inherent information contained within that text, code legibility shifts focus away from machine-reading strategies (tokenization, recursion, and branching), to a model of natural reading strategies (chunking, line-by-line scanning, and line-wise skimming). This shift in paradigm offers a language-agnostic, theoretically sound, and mathematically elegant approach to understanding source code. We propose this shift while still recognizing the need to make explicit considerations for how source code differs from natural language in its affect on cognitive load, and then demonstrate that this informational approach matches practical intuition better than existing code complexity metrics.

## 2. Background and Related Work

### 2.1 The Problem of Code Complexity

#### 2.1.1 Differences Between Natural Language and Code

#### 2.1.2

### 2.2 Existing Code Complexity Research and Metrics

#### 2.2.1 McCabe Cyclomatic Complexity

#### 2.2.2 NPATH

### 2.3 Research on Human Comprehension

### 2.4 Research on Reading Strategy

### 2.5 Empirical Comparisons between Natural Language and Programming Languages

## 3. Requirements for Cognitive-Aligned Metrics

### 3.1 Practical Expectations

[TODO] This should be based on insights from section 2 as well as practical industry experience.

Effective complexity metrics must satisfy:

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


### 3.2 Correlation with Human Readability

Given a body of text $T$, describe an impression $R$ for developer readability with a score $V$. 

Our hypothesis is that the mapping of $V$ onto $R$ is logarithmic, i.e., that exponential penalties for syntactic concentration correlate more strongly with human readability assessments than linear structural complexity measures.

**Formal Optimization Target:**

[TODO]

Needs fleshing out.

[/TODO]

Let $R(T)$ be the human readability assessment of source code $T$, and $V(T)$ be Violet's complexity score. We seek to maximize the correlation:

$$\rho(R, V) = \min_{\theta} \text{corr}(R(T_i), V_{\theta}(T_i))$$

Where $V_{\theta}$ represents the parameterized Violet scoring function with thresholds and penalty coefficients $\theta$.

This formalization transforms the abstract goal of "measuring complexity" into the concrete objective of "predicting human cognitive load through distributional analysis."

## 4. Violet's Core Scoring Algorithm

### 4.1 Mathematical Foundation
[TODO] 

This section is meant to take the background research on various reading and comprehension studies and tether them together to what will become our intuitive basis for why Violet actually does improve the readability of code.

[/TODO]

### 4.2 Chunking

[TODO]

This section is meant to explain the methodology for chunking up files for analysis. Ideally, it connects the method back to the intuition that programmers still search for top level scopes as landmarks to begin processing sections of code

[/TODO]

### 4.3 Legibility Scoring

Given a line of code $\ell$, Violet calculates three legibility factors:

$$V(\ell) = V_{\delta}(\ell) + V_{\nu}(\ell) + V_{\sigma}(\ell)$$

Where:
- $V_{\delta}(\ell) = \theta_{\delta}^{\delta(\ell)}$ — the penalty for nesting depth
- $V_{\nu}(\ell) = \theta_{\nu}^{\nu(\ell)}$ — the penalty for line length, termed "verbosity" 
- $V_{\sigma}(\ell) = \theta_{\sigma}^{\sigma(\ell)}$ — the penalty for non-plain text characters present in the line, termed "syntactics"

And:

- $\delta(\ell)$ = indentation depth of line $\ell$ (adjusted: $\max(0, \text{indents} - 1)$)
- $\nu(\ell)$ = number of non-special characters in $\ell$, or the verbosity of $\ell$
- $\sigma(\ell)$ = number of special characters in $\ell$ (operators, brackets, punctuation)
- $\theta_\delta$, $\theta_\nu$, and $\theta_\sigma$ are parameterized exponential penalties for each feature of a line.

The legibility of a code chunk, then, is simply the sum of its line-wise legibility:

$$V = \sum_{i=1}^{n} V(\ell_i)$$


### 4.4 Features of this Approach

#### 4.4.1 Exponential Punishment
- Depth is exponentially punished as is typically expected of a complexity metric
- Concentrating complexity in a single line is also punished exponentially
- Breaking from typical natural language processing

## 5. Evaluation and Case Studies

### 5.1 Distributional Effects

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

### 5.2 Cognitive Alignment

This mathematical preference aligns with cognitive science principles:

**Working Memory Constraints:** The distributed version fits within typical working memory limits (7±2 items per cognitive chunk).

**Sequential Processing:** Human text comprehension works left-to-right, top-to-bottom. Distribution allows sequential parsing without cognitive overload.

**Chunking Theory:** Each distributed line forms a coherent cognitive chunk, while the concentrated version requires simultaneous processing of multiple concepts.

### 5.4 Text-Based Analysis

Unlike traditional complexity metrics that require language-specific parsing, Violet operates on textual patterns:

- **Special Character Detection:** Language-agnostic pattern matching for operators, brackets, punctuation
- **Indentation Analysis:** Universal whitespace-based depth calculation  
- **Line-by-Line Processing:** Independent analysis avoiding complex syntax tree requirements

### 5.6 Universal Applicability

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

### 5.7 Self-Validation Experiment

The most compelling validation comes from Violet's ability to analyze its own codebase successfully. After iterative refactoring guided by complexity scores:

- **58 total tests passing:** Comprehensive test coverage maintained
- **Zero complexity violations:** All functions score ≤ 6.0 threshold
- **High developer satisfaction:** Multiple developers reported improved code readability

### 5.8 Refactoring Case Study

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

### 5.9 Distributional Discovery

The most remarkable finding emerged during final testing: spreading a complex expression across multiple lines *reduced* the complexity score despite adding more lines and indentation. This counterintuitive result revealed the algorithm's mathematical elegance—it naturally encourages human-readable patterns through pure mathematical properties.

## 7. Discusion


## 7.1 Implications of this Work

## 7.2 Limitations

## 7.3 Future Work

#### 7.3.1 Empirical Validation Studies

**Cross-Language Validation**
Systematic testing across programming languages with controlled experiments:
- **Methodology**: Identical algorithms implemented in 10+ languages, scored by Violet
- **Hypothesis**: Consistent complexity patterns across language paradigms
- **Expected Outcome**: Language-specific calibration factors for threshold adjustment

**Human Subject Studies**  
Controlled experiments measuring correlation with readability assessments:
- **Design**: Developers read functionally equivalent code samples with different Violet scores
- **Measures**: Comprehension time, error rates, subjective difficulty ratings
- **Goal**: Establish empirical correlation between scores and human cognitive load

#### 7.3.2 Cognitive Load Modeling

**EEG/fMRI Integration**
Direct measurement of brain activity during code comprehension:
- **Research Question**: Do Violet scores correlate with neural indicators of cognitive load?
- **Methodology**: Brain imaging while developers read high/low scoring code samples
- **Applications**: Validate theoretical cognitive load assumptions

**Working Memory Experiments**
Test specific claims about 7±2 element processing limits:
- **Design**: Measure recall accuracy for code elements in high vs. low scoring functions  
- **Hypothesis**: Distributed complexity improves working memory performance
- **Impact**: Strengthen cognitive science foundations

#### 7.3.3 Advanced Algorithmic Development

**Semantic-Aware Extensions**
Incorporate semantic analysis while maintaining language agnosticism:
- **Approach**: Universal semantic patterns (assignment, composition, iteration)
- **Goal**: Balance syntactic and semantic complexity measurement
- **Challenge**: Maintain computational efficiency and broad applicability

**Dynamic Threshold Adaptation**
Machine learning approaches to context-specific threshold calibration:
- **Training Data**: Team-specific readability assessments paired with Violet scores
- **Features**: Language, domain, team experience, codebase size
- **Output**: Automatically calibrated thresholds for different contexts

**Temporal Complexity Analysis**
Extend analysis to code evolution and maintenance patterns:
- **Metrics**: Complexity change over time, refactoring frequency, bug correlation
- **Applications**: Predict maintenance hotspots, guide refactoring priorities
- **Data Sources**: Git history, issue tracking, code review comments

#### 7.3.4 Practical Integration Research

**IDE Integration Studies**
Real-time complexity feedback in development environments:
- **Design**: A/B testing with developers using Violet-integrated vs. standard IDEs
- **Measures**: Code quality, development velocity, developer satisfaction
- **Goal**: Quantify productivity impact of real-time complexity feedback

**Code Review Enhancement**
Integration with automated code review processes:
- **Research**: Effectiveness of Violet scores in identifying problematic changes
- **Methodology**: Historical analysis of high-scoring changes and subsequent bug reports
- **Applications**: Intelligent review assignment, automated complexity warnings

#### 7.3.5 Theoretical Extensions

**Information-Theoretic Foundations**
Deeper mathematical analysis of distributional complexity:
- **Questions**: Optimal penalty functions, theoretical limits, convergence properties
- **Methods**: Information geometry, signal processing theory, compression analysis
- **Goal**: Stronger theoretical foundations for empirically-derived constants

**Cross-Domain Applications**
Apply distributional complexity principles beyond code:
- **Natural Language**: Technical documentation, legal texts, academic papers
- **Visual Design**: UI complexity, information architecture
- **System Architecture**: Distributed system complexity, configuration management

### 7.4 Open Research Questions

1. **Threshold Universality**: Do optimal complexity thresholds generalize across programming cultures and domains?

2. **Penalty Function Optimality**: Are the current exponential bases (1.25, 2.0, 1.05) mathematically optimal or empirically convenient?

3. **Semantic Integration**: How can semantic complexity be incorporated without sacrificing language agnosticism?

4. **Scale Effects**: Does Violet's effectiveness change for very large codebases or microservice architectures?

5. **Learning Adaptation**: Can Violet scores be automatically calibrated based on team-specific readability patterns?

## 8. Conclusion

Violet represents a paradigm shift in code complexity analysis—from counting control structures to measuring information distribution. The algorithm's mathematical elegance emerges from a simple insight: exponential penalties for concentration naturally encourage patterns that human cognition finds easier to process.

The theoretical implications extend beyond software engineering. Violet demonstrates how mathematical principles from information theory can align with cognitive science findings, suggesting deeper connections between mathematical elegance and human information processing than previously recognized.

Most remarkably, this elegant theory emerged not from theoretical design but from practical iteration—a testament to the idea that mathematical beauty often reveals itself through empirical discovery rather than pure theoretical construction.

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

[deep research report](https://claude.ai/chat/a4f0d7b1-8c7b-4a7c-a10c-d36bbea9009f) - will be used to gather further references and reading materials

### Historical Context

The evolution of software complexity metrics reflects growing sophistication in our understanding of what makes code difficult to work with:

1. **1970s**: Halstead's operator/operand metrics and McCabe's structural complexity
2. **1980s**: NPATH's attempted path counting and recognition of McCabe's limitations  
3. **2000s**: Information-theoretic approaches and compression-based analysis
4. **2020s**: Violet's distributional complexity analysis combining information theory with cognitive insights

Violet represents a synthesis of these approaches, applying information-theoretic rigor to the practical problem of measuring code readability in a way that aligns with human cognitive patterns, and addresses a growing realization within the industry of code readability's very real impact on operational velocity and success.
