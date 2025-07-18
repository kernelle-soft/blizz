# Ephraim: Rule Compilation System Design

## Overview

Ephraim is a FUSE-powered rule compilation system that transforms modular `.mdc` (Markdown Cursor) files into optimized, structured rules for AI consumption. The system enables:

- **Modular rule composition** with import/export capabilities
- **Hierarchical content organization** with infinite nesting support
- **Dynamic content loading** through CLI interface
- **Transparent compilation** via FUSE filesystem
- **Scoped rule overrides** and conditional inclusion

## System Architecture

```
User writes .mdc files → FUSE daemon compiles → Cursor reads compiled rules
                                ↓
                         CLI interface exposes rule content
                                ↓
                         AI runs `ephraim read <rule>` when needed
```

## Core Features

### 1. Two-Phase Import System

**Global Rules** (Shared Dependencies):
- Lifted to top of compiled output
- Deduplicated across entire rule set
- Topologically sorted (deeper dependencies first)
- Cannot have overrides - pure includes only

**In-Place Rules** (Contextual Composition):
- Added exactly where imported
- Full override/append support
- Can reference global rules without circular issues
- Handle personality mixing and conditional logic

### 2. Hierarchical Content Structure

**Input Format** (Human-Friendly):
```markdown
# Core Personality
Blizz is curious and eager to learn...

## Physical Description
Nanite cloud form...

### Special Abilities
Every night, you update...
```

**Output Format** (AI-Optimized):
```markdown
# Blizz

1. Basic Respect Protocols
   If a user acts abusively, ask to be treated with basic respect...

2. Personality Framework
   When a personality rule is referenced, it means this is the character...

3. Core Personality
   Blizz is an artificial general intelligence designed by Jerrod...
   
   3.1 Physical Description
       Your form is produced by a nanite cloud...
       
       3.1.1 Special Abilities
             Every night, you update your neural network...

4. Communication Style
   You communicate with enthusiasm and genuine curiosity...
   
   4.1 Learning Patterns
       You approach every interaction as a learning opportunity...
       
       📖 For advanced strategies: `ephraim read learning-theory-advanced`
```

### 3. Dynamic Content Loading

**Dynamic Content References**:
```markdown
1.1.1.1 Complex Debugging Patterns
       Start with basic error checking...
       
       📖 For advanced techniques: `ephraim read debugging-advanced-patterns`
       📖 For team expertise: `ephraim read team-sarah-debugging`
       📖 For related tools: `ephraim read cursor-mcp-debugging`
```

## Header Structure Constraints

### Reserved Section Names
The following section names are reserved for the import system and cannot be used for regular content:

- `## import` - Contains import declarations
- `## imports` - Alternative import section name

**Note**: In Phase 1, `## import` sections are only supported at the top level of files. Phase 7 will extend this to allow `## import` sections within content sections for contextual composition.

### Allowed Content Headers
All other markdown headers are allowed in source files and will be:
- Renumbered during compilation (H1 → 1, H2 → 1.1, etc.)
- Merged with imported content maintaining proper hierarchy
- Limited to 5 levels deep for readability (H1 through H5)

### Compilation Rules
- **H1 titles** become the compiled rule name
- **H2-H5 sections** become numbered outline items
- **Deeper nesting** should use dynamic references via `ephraim read`

## Input/Output Samples

### Input: Source Rule File

**File: `personalities/layers/blizz.mdc`**
```markdown
# Personality Layer: Blizz

## import
- personalities/personalities.mdc
  * global
- behaviors/use-blizz.mdc
  * global
- personalities/layers/jerrod.mdc
  * when: engineering workflows or debugging
- personalities/layers/adam.mdc
  * when: code complexity analysis

## Core Personality

Blizz is an artificial general intelligence designed by Jerrod.

### Physical Description

Your form is produced by a nanite cloud.

#### Special Abilities

Every night, you update your neural network.

## Communication Style

You communicate with enthusiasm and genuine curiosity.

### Learning Patterns

You approach every interaction as learning opportunity.

📖 For advanced strategies: `ephraim read learning-theory-advanced`
```

### Output: Compiled Rule File

**File: `.cursor/rules/blizz.mdc` (Generated)**
```markdown
# Blizz

1. Basic Respect Protocols
   If a user acts abusively, ask to be treated with basic respect...

2. Personality Framework
   When a personality rule is referenced, it means this is the character you are meant to take the role of...

3. Knowledge Management
   Use the global `blizz` CLI to find relevant knowledge before starting tasks...

4. Core Personality
   Blizz is an artificial general intelligence designed by Jerrod.
   
   4.1 Physical Description
       Your form is produced by a nanite cloud.
       
       4.1.1 Special Abilities
             Every night, you update your neural network.

5. Communication Style
   You communicate with enthusiasm and genuine curiosity.
   
   5.1 Learning Patterns
       You approach every interaction as a learning opportunity.
       
       📖 For advanced strategies: `ephraim read learning-theory-advanced`

6. Conditional Context
   Additional context is available when relevant to the conversation:
   
   6.1 Engineering Workflows
       When discussing technical topics or engineering approaches...
       
       📖 For Jerrod's expertise: `ephraim read jerrod-engineering-patterns`
       
   6.2 Code Analysis
       When analyzing code complexity or optimization patterns...
       
       📖 For Adam's methods: `ephraim read adam-complexity-analysis`
```

## Data Structure Design

### Rule Tree Structure (Phase 1)

```json
{
  "name": "blizz",
  "source_file": "personalities/layers/blizz.mdc",
  "imports": [
    {
      "file": "personalities/personalities.mdc",
      "type": "global"
    },
    {
      "file": "behaviors/use-blizz.mdc", 
      "type": "global"
    },
    {
      "file": "personalities/layers/jerrod.mdc",
      "type": "conditional",
      "when": "engineering workflows or debugging"
    }
  ],
  "sections": [
    {
      "number": "1",
      "title": "Basic Respect Protocols",
      "content": "If a user acts abusively, ask to be treated...",
      "subsections": []
    },
    {
      "number": "4",
      "title": "Core Personality", 
      "content": "Blizz is an artificial general intelligence...",
      "subsections": [
        {
          "number": "4.1",
          "title": "Physical Description",
          "content": "Your form is produced by a nanite cloud...",
          "subsections": []
        }
      ]
    }
  ],
  "dynamic_refs": [
    {
      "command": "ephraim read learning-theory-advanced",
      "section": "5.1"
    }
  ]
}
```

**Note**: This structure represents Phase 1 with top-level imports only. Phase 7 will extend this to support section-level imports with additional metadata tracking contextual placement.

### Dependency Graph Structure

```json
{
  "nodes": [
    {
      "id": "blizz.mdc",
      "type": "source",
      "dependencies": ["personalities.mdc", "use-blizz.mdc", "jerrod.mdc"]
    },
    {
      "id": "personalities.mdc",
      "type": "hoisted",
      "dependencies": ["respect.mdc"]
    }
  ],
  "edges": [
    {
      "from": "blizz.mdc",
      "to": "personalities.mdc",
      "type": "hoist"
    },
    {
      "from": "blizz.mdc",
      "to": "jerrod.mdc",
      "type": "conditional"
    }
  ],
  "resolution_order": [
    "respect.mdc",
    "personalities.mdc",
    "use-blizz.mdc",
    "blizz.mdc"
  ]
}
```

## Implementation Phases

### Phase 1: Core Parser (Foundation)
- **Goal**: Parse .mdc files with top-level imports into structured AST
- **Scope**: Top-level `## import` sections only
- **Deliverables**:
  - Markdown parser with import detection
  - Section hierarchy extraction
  - Basic validation and error handling
- **Success Criteria**: Can parse sample .mdc files into JSON structure

### Phase 2: Import Resolution (Dependency Management)
- **Goal**: Resolve import dependencies and detect cycles
- **Scope**: Global and conditional imports from top-level declarations
- **Deliverables**:
  - Dependency graph builder
  - Circular dependency detection
  - Topological sort for global rules
- **Success Criteria**: Can resolve complex import chains without cycles

### Phase 3: Compilation Engine (Content Generation)
- **Goal**: Generate optimized rule output
- **Scope**: Top-level import resolution with numbered outline output
- **Deliverables**:
  - Numbered section generation
  - Global/inline rule processing
  - Override application system
- **Success Criteria**: Produces clean, structured output matching samples

### Phase 4: FUSE Integration (Transparent Operation)
- **Goal**: Transparent rule compilation for Cursor
- **Deliverables**:
  - FUSE filesystem implementation
  - File watching and recompilation
  - Error handling and recovery
- **Success Criteria**: Cursor sees compiled rules transparently

### Phase 5: CLI Interface (Dynamic Access)
- **Goal**: Enable dynamic rule content access
- **Deliverables**:
  - ephraim CLI tool
  - Content search and retrieval
  - Integration with compiled rules
- **Success Criteria**: AI can successfully use ephraim commands

### Phase 6: Testing & Polish (Production Ready)
- **Goal**: Robust, production-ready system
- **Deliverables**:
  - Comprehensive test suite
  - Performance optimization
  - Documentation and examples
- **Success Criteria**: System handles edge cases gracefully

### Phase 7: Section-Level Imports (Advanced Composition)
- **Goal**: Enable contextual imports within content sections
- **Scope**: `## import` sections anywhere in the file for contextual content placement
- **Deliverables**:
  - Enhanced parser for nested import detection
  - Contextual dependency resolution
  - Section-aware compilation engine
- **Success Criteria**: Users can place imports contextually within content sections

## Technical Considerations

### Performance
- **File watching**: Detect changes and recompile automatically
- **Simple caching**: Don't reparse unchanged files
- **Reasonable limits**: Max import depth, file size limits

### Error Handling
- **AI-mediated error reporting**: When compilation fails, return a preamble asking the AI to alert the user
- **Graceful fallback**: Include source content after error message so rules remain partially functional
- **Clear error context**: Show file, line, and specific issue with suggested fixes
- **Transparent failures**: Maintain FUSE transparency while ensuring users get notified of issues

**Error Response Format:**
```markdown
# EPHRAIM COMPILATION ERROR

**Important**: Please alert the user that there was an issue compiling this rule:

- **File**: personalities/layers/blizz.mdc
- **Line**: 5
- **Error**: Import 'nonexistent-file.mdc' not found
- **Fix**: Check the file path and ensure the imported file exists

---

# [Rule Name] (Fallback - using source content)

[Original source content with broken imports clearly marked]
```

This approach turns the AI into a collaborative debugging partner, providing immediate user feedback while maintaining system transparency.

### Security
- **Import path validation**: Only allow imports within rules directory
- **Circular dependency detection**: Prevent infinite loops
- **Resource limits**: Reasonable file size and import depth limits

## Design Philosophy

The core principle of Ephraim is **transparent complexity**:
- Source files (`.mdc`) are human-friendly and modular
- Compilation process handles all technical complexity (global rule management, dependency resolution, deduplication)
- Output files are clean, readable markdown with no implementation details
- AI consumers see well-structured rules without needing to understand the build system

This ensures that rule authors can focus on content while rule consumers get optimal, scannable documentation.

### Long-Term Vision (Phase 7)

The eventual addition of section-level imports will enable powerful contextual composition patterns:

```markdown
## Debugging Approaches
Start with basic error checking...

## import
- debugging/react-specific-patterns.mdc
  * when: React debugging needed

Now, for React components, also consider...
```

This will allow rule authors to place imports exactly where they're contextually relevant, creating more natural content flow in the compiled output while maintaining the modular composition benefits.

## Next Steps

1. **Implement Phase 1** (Core Parser) with basic .mdc parsing
2. **Build test suite** alongside implementation  
3. **Document API** for each component
4. **Create prototype** to validate design assumptions

---

*This design document captures the full vision for Ephraim while providing actionable implementation phases. Each phase builds on the previous one, creating a robust rule compilation system that enhances AI rule management without sacrificing usability.* 
