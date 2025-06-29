# Strategies

This document defines and outlines the default scoring strategies of Violet's complexity scoring algorithm.

## Table of Contents
- Constant Strategy
- Operation Strategy
- Branch Strategy
- Iteration Strategy
- Match Strategy

## Primitive Strategy

### Applicable To
- Language primitives, such as numbers and strings

### For Coders

```
function complexity_of(primitive):
  return 1
```

### For Theorists

$
  Violet(P) = 1
$

Where:
- $P$: the primitive value

### Why

Numbers, boolean values, and other primitives expressed in source code always have some semantic meaning. Because of this, they're considered the atomic basis of Violet's complexity metric.

## Alias Strategy

### Applicable To
- constants
- variable declarations

### Pseudocode

```
function complexity_of(alias):
  return 1
```

### Formalization

$
Violet(a) = 1
$

Where:
- $a$: the alias


### Why:

Aliases tuck complexity away. The complexity they're an abstraction for is already being counted somewhere else. However, there's cognitive cost associated with comprehending the meaning of the abstraction, and so the abstraction itself should be counted as a separate factor.

Consider the following example:

```
let var = "Hello, Violet!"
```

This would have a complexity of 2, 1 for the string primitive, and 1 for the variable assignment. Naively, one might consider counting the variable declaration separately a double counting. However, consider the next example:

```
function add_1():
  let x = 2
  return x + 1

function add_2():
  return 2 + 1
```

The second function is more legible than the first, and so a lower complexity would be fitting, but the only difference between the two functions is the introduction of the alias. Therefore, the alias is a factor of complexity unto itself.


## Operation Strategy

### Applicable To
- Common mathematical operations
- Boolean Expressions
- Vectorized operations
- Function calls

### Pseudocode
```
function complexity_of(operation):
  let total = 0

  for each operand in operation.operands:
    total += complexity_of(operand)
  
  return total + operation.operands.length
```

### Formalization

$
Violet(O) =  \big(\sum\limits_{o_i \in O}Violet(o_i)\big) + |O|
$

where:

- $O$: The list of operands in the operation

### Why

Consider these four functions:

```
function calc_1(x1, ..., xn):
  return x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + ... + xn
```

```
function calc_2(x1, ..., xn):
  return ((((x1 + x2) + x3) + x4) + ...) + xn)
```

```
function calc_3(x1, ..., xn):
  let sum = 0
  if x1:
    sum += x1
    if x2:
      sum += x2
      if x3:
        sum += x3
        if ...
```

```
function calc_4(x1,..., xn):
  let sum = 0
  sum += x1
  sum += x2
  sum += x3
  ...
  return sum
```

The individual addition operations of `calc_1` are part of an implicit nested expression tree (shown explicitly in `calc_2`).

This nesting has a similar cognitive load on the reader as if the same additions were performed through nested branching (shown in `calc_3`). 

The more deeply nested the expression, the less likely the reader is able or willing to take on the load of parsing and understanding the meaning of it. NPATH asserts that branching operations have a scaling effect on cognitive complexity, with more deeply nested code creating more perceived complexity than the same operations being performed in the same level of nesting. We agree, and maintain that this is true for nested expressions and not simply branching: Comparing `calc_1`, `calc_2`, and `calc_3` with the non-nested `calc_4`, the cognitive pressure the first three have on the reader is similar to one another and distinctly higher than that of `calc_4`

Adding 2 to the score for each of these binary operations allows Violet to punish deeply nested operands appropriately by scaling the score with the depth of the expression tree.

Now, consider how this interacts with function calls:

```
function uses_calc_1():
  return calc_1(1, 2, 3) + calc_1(4, 5, 6) + calc_1(7, 8, 9)
```

```
function uses_calc_2():
  return ( calc_2(1, 2, 3) + calc_2(4, 5, 6) ) + calc_2(7, 8, 9)
```

```
function uses_calc_4():
  let sum = 0
  sum += calc_2(1, 2, 3)
  sum += calc_2(4, 5, 6)
  sum += calc_2(7, 8, 9)
  return sum
```

`use_calc_1` and `use_calc_2` both score a total of 4 + 9 = 13, compared with `use_calc_4`'s score of 15

## Branch Strategy 

### Applicable To
- Singlet Branches (IF with no ELSE)
- Doublet Branches (IF/ELSE)
- Open Chain Branches (IF/ELSE-IF...ELSE-IF, but no ELSE)
- Closed Chain Branches (IF/ELSE-IF...ELSE-IF/ELSE)

### For Theorists:

$
Violet(B) = \sum\limits_{b_i \in B}Violet(b_i) + |B|
$

#### Where:
- $B$: the chain of branches
- $b_i$ is an individual branch in the chain



### For Coders:
```
function complexity_of(branches):
  let total = 0

  for each branch in branches:
    total += complexity_of(branch)

  total += branches.length

  return total
```

## Iteration Strategy

### Applicable To

### Pseudocode

### Formalization

### Why

## Match Strategy

### Applicable To

- Switch statements and cases

### Pseudocode

```
function complexity_of(cases):
  let total = 1

  for each case of cases:
    total += compexity_of(case)

  return total
```


### Formalization

### Why