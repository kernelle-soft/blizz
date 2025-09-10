# Getting Started with Blizz

This guide will help you get up and started with blizz's AI development toolset.

After you've gotten comfortable with the tools and how they work together with your agentic IDE, give your AIDE a few sessions to gather insights and begin establishing your areas of expertise, projects, and other context. 

Alternatively, to start seeing the advantages more quickly, spend a few chat sessions pulling in local insights from the web, internal wikis, or your project management system to seed your RAG search with information about what you're working on.

## System Requirements

- **OS**: Linux (x86_64) or macOS (ARM64) 
- **Memory**: 8GB RAM recommended for embedding models
- **Storage**: ~6GB free space for models and data
- **Network**: Internet connection for initial model download

## Installation

### Quick Install
```bash
curl -fsSL https://raw.githubusercontent.com/kernelle-soft/blizz/refs/heads/dev/scripts/install.sh | sh
source ~/.blizz.source
```

### Verifying your Installation
```bash
blizz --version 
insights --version 
violet --version
secrets --version
```

### (Linux only) Setting up CUDA dependencies
If you would like our embedding and reranking model to run on a GPU to speed up insights searching, we've provided a setuptool that automatically detects and installs the needed NVIDIA dependencies to make this possible.

Apple users don't need to do this. GPU acceleration should come out of the box. It's also worth noting that the insights system is designed to run with acceptable times even when not using GPU acceleration.

## Linking Blizz to Repositories

### 1. Link AI Agent Rulesets to Your Project
```bash
cd /path/to/your/project
blizz link
```

**What this does:**
- Creates `.cursor/blizz/` folder with AI agent rulesets and tools
- Sets up local RAG system for persistent AI memory and automatic insight generation
- Configures AI-actionable code quality analysis
- Establishes secure credential access for MCPs


### 2. Customized Rules

Open your project and notice the folder `.cursor/rules/blizz/`. Under that folder, there's another subfolder called `personal`

Try adding a rule `my-first-rule.mdc` of your own there:

```md
---
description: Greet me
globs:
alwaysApply: true
---

At the beginning of every single new chat session, greet me with "Hello, World!" before doing anything else.
```

Test out the rule in a new chat session.

Once you've verified your rule is working, try using `blizz link` on another project, then opening that. Note that `my-first-rule.mdc` has carried over!


### 3. Configure MCP Credentials (Optional but Recommended)
```bash
# Store credentials securely for AI agent MCP access
secrets store anthropic api_key      # For Claude MCP integration
secrets store github access_token    # For GitHub MCP integration
secrets store openai api_key         # For OpenAI MCP integration
secrets store jira api_key           # For JIRA tickets

# etc
```

[TODO] add details on hooking up MCPs and/or shell files that use the secrets tool to manage secrets.

## How Your Agent Will Use These Tools

### Persistent Memory (Insights)
At the beginning of most sessions, most thinking models will make calls out to the `insights` system to pull in relevant information from previous discussions.

As the discussion goes on, discoveries and key insights you or the model deem important will automatically be saved by adding new insights

```bash
# (You can also add insights manually, but agents do this automatically)
insights add "architecture" "auth-system" "User auth flow" "Using JWT tokens with refresh rotation, stored in httpOnly cookies. Main auth middleware in src/auth/middleware.ts"
```

In the above example, the next time you ask about authentication days, weeks, or months down the road, your AI agent remembers these specific decisions and implementation details from your codebase and why those decisions were made, along with any other insights relating to authentication.

```bash
# AI agent searches its memory for relevant context
insights search "authentication"
```

### Code Quality Analysis (Violet)

As the agent generates code, you may notice it making calls to `violet` from time to time:

```bash
violet src/components/UserForm.tsx
```

This is to analyze how readable its own code is.

Your agent can then use the feedback provided by the tool to refactor its own code automatically for better readability.

The agent will perform checks like this automatically from time to time if it feels its own code is getting unwieldy. However, there's nothing stopping you from running the tool yourself project-wide and having your agent automatically clean it up and organize it for legibility and reusability.

### The Task Runner
Handle "works on my machine" problems:

**Project-level config** (`blizz.yaml` at repo root):
```yaml
# Team standard tasks
test:
  - cargo test --workspace
  - npm test
  
deploy:
  - blizz do test
  - docker build -t app:latest .
```

**Personal config** (`.cursor/blizz/blizz.yaml`):
```yaml
# Your personal environment quirks
test:
  - export RUST_BACKTRACE=1  # You like verbose errors
  - cargo test --workspace
  - npm test
  
my-setup:
  - docker stop old-postgres || true  # Your Docker conflicts
  - export NODE_OPTIONS="--max-old-space-size=8192"  # Your machine needs more memory
```

**The Magic:** AI agent knows both team standards AND your personal workarounds.

## Real-World AI-Assisted Workflows

### 1. Context-Aware Development Sessions

**Start a development session:**
```bash
# AI agent can run this automatically when you start work
blizz do my-setup  # Your personal environment setup
```

**During development:** Your AI agent remembers:
- Previous architectural decisions (stored in insights)
- Code quality patterns (from violet analysis)
- Your personal workflow preferences (from configs)
- Organizational context (from MCP integrations)

### 2. AI-Enhanced Code Review

```bash
# AI agent runs quality analysis
violet --quiet . || echo "Quality issues found - see AI agent for specific suggestions"

# AI agent captures and remembers review patterns
# (This happens automatically based on your conversations)
```

### 3. Organizational Knowledge Integration

**With MCPs configured, your AI agent can:**
- Reference similar solutions from other repositories (GitHub MCP)
- Remember past incidents and solutions (Jira/Linear MCP)
- Understand team decisions from documentation (Confluence/Notion MCP)
- Recall team discussions and context (Slack MCP)

## Understanding the File System

### Insights Are Just Markdown Files
```bash
# Your AI agent's memory is stored as readable files
ls ~/.blizz/persistent/insights/

# You can inspect, backup, or modify with standard tools
grep -r "authentication" ~/.blizz/persistent/insights/
git init ~/.blizz/persistent/insights/  # Version control AI memory
```

### Team Collaboration
```bash
# Teams can share AI knowledge via git
cd ~/.blizz/persistent/insights/
git clone git@company.com:team/shared-insights.git team/

# Now your AI agent has both personal and team context
```

## Advanced Configuration

### Personal vs Team Config Layers

**Team violet config** (project root `violet.yaml`):
```yaml
# Team code quality standards
penalties:
  depth: 1.2
  verbosity: 1.1
  syntactics: 1.3
```

**Personal violet config** (`.cursor/blizz/violet.yaml`):
```yaml
# Your personal preferences (merges with team config)
ignore_patterns:
  - "legacy/*"  # You're not responsible for fixing legacy code
penalties:
  verbosity: 1.0  # You prefer more verbose code
```

### MCP Integration Examples

**GitHub MCP** - AI agent learns from other repositories:
```bash
# AI agent can reference solutions from your other projects
# "I see you solved similar caching issues in the billing-service repo..."
```

**Jira MCP** - AI agent remembers past incidents:
```bash
# AI agent recalls incident context
# "This error is similar to PROD-1847 from last month. The solution was..."
```

## Troubleshooting

### AI Agent Not Finding Context
```bash
# Rebuild AI agent's memory index
insights index

# Check if insights server is running
insights logs
```

### Configuration Issues
```bash
# Check that rulesets are properly linked
ls -la .cursor/blizz/

# Verify configs are loading
blizz tasks  # Should show both team and personal tasks
```

### MCP Credential Issues
```bash
# Check credential storage
secrets list

# Test credential access
secrets read github access_token
```

## What Happens Next

Once setup is complete:

1. **Your AI agent builds context** - Every conversation adds to its memory
2. **Code quality improves** - AI suggestions become specific to your codebase  
3. **Workflows become consistent** - AI agent learns your preferences and team patterns
4. **Knowledge compounds** - Organizational context grows through MCP integrations

## Validation Checklist

✅ **Installation complete** - All commands work
✅ **Project linked** - `.cursor/blizz/` folder exists
✅ **AI memory active** - Insights server running
✅ **Code analysis working** - Violet provides specific output
✅ **Credentials configured** - MCPs can access external systems
✅ **Configs layered** - Personal + team settings merge correctly

## Next Steps

- **[Real-World Examples](./examples.md)** - See AI-assisted development workflows
- **[Team Collaboration](./team-collaboration.md)** - Share AI knowledge via git
- **[Organizational Knowledge](./organizational-knowledge.md)** - Enterprise MCP integration
- **[Alpha Guide](./alpha.md)** - Current limitations and feedback

## Alpha Feedback

Help us improve the AI-assisted development experience:

```bash
# Your AI agent can help collect this feedback
insights add "alpha-feedback" "setup-experience-$(date +%Y-%m-%d)" "Blizz setup experience" "Setup time: X minutes. Blockers: [list]. AI agent effectiveness: [rating]. Most valuable feature: [describe]. Improvement suggestions: [list]"
```

---

*The goal isn't to replace your development workflow - it's to give your AI assistant the tools and memory to actually understand and enhance it.*