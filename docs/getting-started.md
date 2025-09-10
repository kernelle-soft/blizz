# Getting Started with Blizz

This guide will take you from zero to productive with Blizz in about 10 minutes.

## System Requirements

- **OS**: Linux (x86_64) or macOS (ARM64)
- **Memory**: 4GB RAM minimum, 8GB recommended for embedding models
- **Storage**: 2GB free space for models and data
- **Network**: Internet connection for initial model download

## Installation

### Quick Install
```bash
curl -fsSL https://raw.githubusercontent.com/kernelle-soft/blizz/refs/heads/dev/scripts/install.sh | sh
```

### Manual Shell Setup
Add to your `~/.bashrc`, `~/.zshrc`, or equivalent:
```bash
source ~/.blizz.source
```

Then reload your shell:
```bash
source ~/.bashrc  # or ~/.zshrc
```

### Verify Installation
```bash
blizz --help
insights --help
violet --help
secrets --help
```

## First Workflow: Knowledge Management

### 1. Add Your First Insight
```bash
insights add "projects" "blizz-setup" "Successfully installed Blizz" "Completed the installation process and verified all tools are working. The local RAG system is now ready for capturing development knowledge."
```

### 2. Test Search
```bash
# Exact search
insights search "setup"

# Semantic search (finds related concepts)
insights search "installation" --semantic
```

### 3. Explore Your Knowledge Base
```bash
# List all topics
insights topics

# List insights in a topic
insights list projects

# Get a specific insight
insights get projects blizz-setup
```

## Second Workflow: Code Quality

### 1. Analyze Code Legibility
```bash
# Check current directory
violet .

# Check specific files with details
violet src/main.rs

# Only show violations
violet --quiet .
```

### 2. Capture Code Quality Insights
```bash
insights add "code-quality" "violet-first-run" "Analyzed codebase with Violet" "Ran Violet on the project and found X violations. The tool provides specific feedback about code legibility and complexity. Key issues: [list main findings]"
```

## Third Workflow: Task Automation

### 1. Explore Available Tasks
```bash
blizz tasks
```

### 2. Run Common Tasks
```bash
# Run tests
blizz do test

# Format code
blizz do format

# Run quality checks
blizz do checks
```

### 3. Create Custom Tasks
Edit `blizz.yaml` in your project root:
```yaml
my-workflow:
  - blizz do format
  - blizz do test
  - violet --quiet .

deploy-prep:
  - do: checks
  - insights add "deployment" "$(date +%Y-%m-%d)-deploy" "Deployment checkpoint" "All checks passed, ready for deployment"
```

## Fourth Workflow: Secrets Management

### 1. Store Your First Secret
```bash
secrets store anthropic api_key
# Enter your API key when prompted
```

### 2. Read Secrets
```bash
secrets read anthropic api_key
```

### 3. List All Secrets
```bash
secrets list
```

## Building Your Knowledge Base

### Effective Insight Organization

**Good insight topics:**
- `projects` - Project-specific learnings
- `debugging` - Problem-solving patterns
- `architecture` - Design decisions and patterns
- `tools` - Tool configurations and workflows
- `team` - Team processes and standards

**Writing effective insights:**
```bash
# Good: Specific, actionable
insights add "debugging" "memory-leak-pattern" "Found memory leak in event listeners" "React components were not cleaning up event listeners in useEffect. Solution: always return cleanup function from useEffect hooks that add listeners."

# Avoid: Too vague
insights add "general" "stuff" "Some things I learned" "Today was interesting..."
```

### Search Strategies

```bash
# Exact search - finds literal matches
insights search "memory leak" --exact

# Semantic search - finds related concepts
insights search "performance issues" --semantic

# Topic-specific search
insights search "debugging patterns" --topic debugging

# Overview-only search (faster)
insights search "architecture" --overview-only
```

## Integrating with Your Workflow

### 1. Link Blizz to New Projects
```bash
cd /path/to/new-project
blizz link
```
This copies your Blizz rules and workflows to the new project.

### 2. Capture Development Sessions
```bash
# At start of work session
insights add "daily" "$(date +%Y-%m-%d)-session" "Working on feature X" "Goals: [list goals]. Starting context: [current state]"

# At end of session
insights add "daily" "$(date +%Y-%m-%d)-results" "Completed work on feature X" "Accomplished: [list completions]. Key learnings: [insights]. Next steps: [list next steps]"
```

### 3. Code Review Workflow
```bash
# Before code review
violet --quiet . && echo "Code quality check passed"

# Capture review insights
insights add "code-review" "feature-x-review" "Code review findings" "Reviewer feedback: [key points]. Changes made: [list changes]. Patterns to remember: [extract patterns]"
```

## Troubleshooting

### Common Issues

**"insights command not found"**
- Make sure you ran `source ~/.blizz.source`
- Verify installation completed without errors

**Embedding model download fails**
- Check internet connection
- Ensure you have sufficient disk space (2GB)
- Try restarting the insights server: `pkill insights_server`

**Searches return no results**
- Run `insights index` to rebuild embeddings
- Verify insights exist: `insights list`
- Try exact search first: `insights search "term" --exact`

**Secrets won't decrypt**
- Verify you're entering the correct master password
- Check that the keeper daemon is running: `secrets agent status`

### Getting Help

1. **Check logs**: `insights logs` for insights issues
2. **Search existing insights**: `insights search "error message"`
3. **File an issue**: [GitHub Issues](https://github.com/kernelle-soft/blizz/issues)
4. **Join discussion**: [GitHub Discussions](https://github.com/kernelle-soft/blizz/discussions)

## Next Steps

- [**Examples**](./examples.md) - See real-world workflows
- [**Alpha Guide**](./alpha.md) - Understand current limitations
- [**Enterprise**](./enterprise.md) - Learn about team features

## Alpha Feedback

We're actively improving the user experience. Please share:

```bash
insights add "alpha-feedback" "experience-$(date +%Y-%m-%d)" "My Blizz alpha experience" "Setup experience: [rate 1-10, describe issues]. Most useful feature: [describe]. Biggest pain point: [describe]. Would recommend to colleague: [yes/no, why]"
```

Or [join the discussion](https://github.com/kernelle-soft/blizz/discussions) to share feedback directly.
