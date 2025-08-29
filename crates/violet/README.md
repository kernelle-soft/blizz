# Violet: a Versatile, Informational, and Objective Legibility Evaluation Tool

## Abstract



## 1. Introduction

### 1.1 The Problem of Code Readability

### 1.2 Prior Work

### 1.3 History

Both enterprise and open source software increasingly rely on automation to validate code quality and prevent regressions. Continuous integration and deployment tools such as GitHub CI, Gitlab CI, Jenkins, and Spinnaker make it possible to checkout, lint, build, validate, and publish code automatically in a web of interconnected steps. Code coverage reporting, security scanning, and automated dependency upgrade tools add to the cadre of available options for maintaining code at scale. And with the advent of large language models, there's a growing need to automatically and objectively validate that software meets a standard of readability and complexity.

Such metrics are not just aesthetic. High complexity code is known to incur a heavy maintenance cost, slowing down real world development by requiring significant time to refactor or build on top of, thereby increasing operational risk, development timelines, and overall cost. These costs pass on to end users, either through literal cost, or through increased frequency of issues and regressions.

Despite this, there is no widespread empirical tool for analyzing code readability used in software production today. Despite decades of research, cyclomatic and path counting approaches have little adoption in professional software engineering. Linting tools are commonplace and can enforce stylistic minutiae specific to a project, but implementations across languages vary in their maturity and level of granularity. Even with a mature linter, however, code can pass recommended checks and still be nearly unreadable by any meaningful interpretation.

A quality legibility analysis tool should be able to dig past surface level stylistic consistency issues to understand the underlying structures of code and score it based on readability without collapsing into full static program analysis.

## 2. Goals

### 2.1 Defining Legibility

Two pieces of code can be parsed into the same syntax tree, but appear very different to a developer actually reading them.

Traditional complexity metrics such as Cyclomatic Complexity and NPATH require language-specific parsing and semantic analysis in order to determine a Complexity score.

Legibility instead takes the opposite approach to code analysis. Rather than measuring what the code is actually doing syntactically, legibility measures how a person perceives and processes the way code is actually expressed on the page.

This approach promises several major advantages over complexity metrics. 

First, it enables universal analysis across all text-based languages. The advantages of this cannot be understated. Legibility analysis allows for meaningful like-for-like comparison of objective scores across languages, eliminates the need for repeat (and therefore spotty) implementation for each new language, while allowing for fine-tuning of a standard threshold for acceptable code from language to language, which is needed for automated validation.

Second, it focuses on human perception rather than program semantics, which ties more closely to the original value proposition of code complexity metrics as a means to ascertain code maintainability.

Third, unlike complexity and path counting metrics which provide only an aggregate score, legibility analysis gives us an avenue for more granular reporting and remediation. Legibility scores can be decomposed into fine-grained details of how that score was obtained, and where and how improvements can be made down to individual lines, which we'll discuss in later sections. [todo]

And finally, it allows us to express the readability of code in terms that have grounding in relevant mature fields: information theory and cognitive science. Legibility is focused on accurately measuring the cognitive load imposed by the the amount of information presented by individual lines.

### 2.2 Defining Constraints

## 3. Algorithm

Given a line of code $\ell$, Violet calculates three legibility factors: *comparative depth*, *verbosity*, and *syntactics*.

#### 3.1 Comparative Depth

We define the indentation depth function $\delta: \mathcal{L} \rightarrow \mathbb{N}_0 $ where $\mathcal{L} $ is the set of all possible code lines:
$$
\delta_\ell = \max(0, \iota_\ell - \tilde{\delta})
$$
Where $\iota_\ell$ is the raw indentation count:
$$
\iota_\ell = \tau_\ell + \left\lfloor \frac{\sigma_\ell}{\omega} \right\rfloor
$$
With:

- $\tau_\ell $ = number of leading tab characters in $\ell $
- $\sigma_\ell$ = number of leading space characters in $\ell $
- $\omega $ = tab width parameter (typically $\omega \in \{2, 4, 8\} $)
- $\lfloor \cdot \rfloor $ = floor function

And $\tilde{\delta}$ is the median expected indentation for the context of line $\ell$, which will be discussed in further detail in <insert future section> 

#### 3.2 Verbosity Scoring

We define our verbosity score $\nu_\ell$ as the the number of alphanumeric characters in $\ell$. For the sake of a precise definition, we let $\ell'$ be the line of code $\ell$ sans indentation.

Let $\mathcal{A} = \{a, b, c, \ldots, z, A, B, C, \ldots, Z, 0, 1, 2, \ldots, 9\} $ be the set of alphanumeric characters, and let $\mathcal{W} = \{\text{space}, \text{tab}\ , \text{underscore}\}\ $ be the set of whitespace characters. 

We define the non-special character set as:
$$
\mathcal{T} = \mathcal{A} \cup \mathcal{W}
$$
The verbosity score function $\nu: \mathcal{L} \rightarrow \mathbb{N}_0 $ can then be defined as:
$$
\nu_{\ell'} = |\{c \in \ell' : c \in \mathcal{T}\}|
$$
Or more formally:
$$
\nu_{\ell'} = \sum_{i=1}^{|\ell'|} \mathbf{1}_{\mathcal{T}}(\ell'_i)
$$
Where $\mathbf{1}_{\mathcal{T}}(c) $ is the indicator function for a single character $c$.

##### A note on underscores

Here we include the underscore as a whitespace character, as it's commonly used for variable and function identifiers to serve the purpose of a whitespace character in many naming conventions, and typically is not reserved as an operator. While the underscore can be used as a special symbol in some languages, we elect to define it in $\mathcal{W}$ because of its most common use as a substitute for whitespace. This affordance allows us to maintain the scope of analyzing patterns of high information over the code instead of tripping over the idiosyncrasies of individual edge-cases on a language by language basis. 

#### 3.3 Syntactic Scoring

Define the syntactic character set $\mathcal{S}$ as the complement of the set of text characters $\mathcal{T}$:
$$
\mathcal{S} = \mathcal{U} \setminus \mathcal{T}
$$
Where $\mathcal{U}$ is the universe of printable characters. The syntactics score function can then be defined relative to $\mathcal{T}$:
$$
\sigma_{\ell'}
= |\{c \in \ell' : c \notin \mathcal{T}\}| \\
= |\{c \in \ell' : c \in \mathcal{S}\}|
$$
Or more formally:
$$
\sigma_{\ell'}
= \sum_{i=1}^{∣\ell'∣}\mathbf{1}_S(ℓ'_i)
= \sum_{i=1}^{∣\ell'∣}(1-\mathbf{1}_N(\ell'_i))
$$

#### 3.4 Penalization

##### 3.4.1 Definition

Now that each dimension of complexity is defined, we apply parameterized exponential penalty functions to each factor.

For a given line of code $\ell$, the exponential line-wise legibility score is:

$$V(\ell) = V_\delta(\ell) + V_\nu(\ell) + V_\sigma(\ell)$$

Where each penalty function takes the form:

- $V_\delta(\ell) = \theta_\delta^{\delta_\ell}$ — penalty for comparative depth
- $V_\nu(\ell) = \theta_\nu^{\nu_{\ell'}}$ — penalty for verbosity  
- $V_\sigma(\ell) = \theta_\sigma^{\sigma_{\ell'}}$ — penalty for syntactics

The penalty base parameters $\theta_\delta$, $\theta_\nu$, and $\theta_\sigma$ are exponential bases that determine the severity of penalties for each complexity dimension. 

#### 3.4.2 Choice of Parameter Values

In practice, these values reflect the relative cognitive impact of each complexity factor: depth creates the most significant comprehension barriers, syntactic density creates moderate barriers, and verbosity creates comparatively minor barriers due to its similarity to natural language. We explore recommended values of these parameters in <future section>, however the algorithm is designed to operate on parameters in the range $1 \leq \theta \leq e$.

**Information-Theoretic Foundation of Exponential Bases**

[comment: The connections here feel really tenuous. There's elegance here, but something feels off.]

These range of these parameters has significance when viewed through the lens of information theory. The constraint $1 \leq \theta \leq e$ (where $e \approx 2.718$) represents the natural bounds of local analysis:

- **$\theta = 1$** denotes no interaction entropy between complexity elements—each unit contributes independently without cross-dependencies on other elements in the same category
- $1 < \theta < e$ describes a bounded interaction entropy where complexity elements create limited interdependencies within a local context
- $\theta = e$ denotes a saturation point. Each contributing element of the penalty would contribute natural cross-element entropy for each other element in the category, representing a theoretical maximum for information complexity at a local scope.

Values beyond Euler's constant would suggest that each instance of an element contributing to the score contributes to the global complexity of the program, extending beyond local code context. While this is objectively the case for many local elements of an arbitrary program, this would require whole-program analysis to assess accurately. By constraining parameters to $[1, e]$, Violet maintains its grounding in information theory without needing to broach the computationally intractable problem of precise whole-program complexity analysis. 

### 3.2 Distributive Complexity

### 3.3 Chunking

## 4. Validation



## 5. Case Studies



## 6. Validation & Future Work



