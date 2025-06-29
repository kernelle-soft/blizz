##

Can it really be that simple?

```
function chunk_complexity(chunk):
  let scores = []
  for each line of chunk.lines:
    const indents = get_indents(line, INDENT_STR)
    const special_chars = get_num_specials(line)
    const non_special_chars = line.strip().length - special_chars
    scores.append(non_special_chars**1.25 + special_chars**1.5 + indents**2)

  // Sum is a simple way of factoring in every line's contributions to complexity fairly
  // Exponentiation disproportionately punishes higher sums
  return exp(scores.sum())
```

```
function file_complexity(file):
  const chunks = get_chunks(file) // a chunk is any top-level scope dilineated by a new-line.
  const scores = chunks.for_each(chunk_complexity)
  return exp(scores.sum() * chunks.length)
```

- Punishes deep nesting
- Punishes ternaries, repeated null coalescing or other assertions
- Punishes explicit use of \<generic\> syntax
- Punishes casting
- Punishes overly long names
- Punishes long lines in general
- Can be used to create a softmax vector for use in with editor hints (imagine coloring lines by relative complexity within a function)
- Doesn't really punish lines that are just closing scope
- Doesn't require any special knowledge of language features
- Explainable. It's more about how humans read text than it is about the exact details of the function.
- Pairs well with other simple metrics like function length, max function depth, max number of params
- Punishes long files
- Punishes long functions

Just need thresholoding