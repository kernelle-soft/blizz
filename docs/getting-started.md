<style>
  .note h3 {
    margin: 0;
    margin-bottom: 6px;
  }

  .note span {
    display: block;
    margin-bottom: 6px;
  }

  .note span:last-of-type {
    margin-bottom: 0;
  }

  .note {
    background-color: #ffff0011;
    padding: 8px;
    border-radius: 4px;
    margin-bottom: 4px; 
  }
</style>

# Getting Started with Blizz

This guide will help you get up and started with blizz's AI development toolset.

<div class="note">
<h3>
Note
</h3>
<span>
After you've gotten comfortable with the tools and how they work together with your agentic IDE, give your AIDE a few sessions to gather insights and begin establishing your areas of expertise, projects, and other context. 
</span>
<span>Alternatively, to start seeing the advantages more quickly, spend a few chat sessions pulling in local insights from the web, internal wikis, or your project management system to seed your RAG search with information about what you're working on.</span>
</div>

## System Requirements

- **OS**: Linux (x86_64) or macOS (ARM64) 
- **Memory**: 16GB RAM or 8GB VRAM to have enough headroom for embedding models in addition to typical development work
- **Storage**: ~2GB free space for models and data

## Installation

### Quick Install
```bash
curl -fsSL https://raw.githubusercontent.com/kernelle-soft/blizz/refs/heads/dev/scripts/install.sh | sh
source ~/.blizz.source
```

If you would like to help with the development of this tool and have your crash reports and usage statistics sent to us automatically, say `yes` when asked about telemetry. Note that this is the only online component to the toolset.

See [here](/TODO/) for more information about what our telemetry logs look like.

### Verifying your Installation
```bash
blizz --version 
insights --version 
violet --version
secrets --version
```

## Linking Blizz to Repositories

### 1. Linking the agentic rules to your project
```bash
cd /path/to/your/project
blizz link
```

**What this does:**
- If needed, creates a `.cursor/` folder in your repository and links Blizz's AI agent rules and workflows
- Adds rules for automatic usage of the insights system for persistent AI memory and automatic insight generation
- Adds rules AI-actionable code quality analysis
- Establishes secure credential access for MCPs


### 2. Customized Rules

Open your project and notice the folder `.cursor/rules/blizz/`. Under that folder, there's another subfolder called `personal` [TODO]

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

### Automatic Code Readability Improvements (Violet)

As the agent generates code, you may notice it making calls to `violet` from time to time and making adjustments.

To get a better idea of what it's doing, run `violet` on _that_ part of your code. You know the one.

```bash
violet src/path/to/messy/code.tsx
```

Violet (the Versatile, Intuitive, and Objective Legibility Evaluation Tool) is designed to analyze how readable your code is and provide insights as to what issues it has.

Your agent can now use the feedback provided by violet to refactor its own code automatically for better readability.

The agent will perform checks like this automatically from time to time if it feels its own code is getting unwieldy. However, there's nothing stopping you from running the tool yourself on certain parts of your project (or even project-wide) and having your agent automatically clean up issues and organize your code for legibility and reusability.

### The Task Runner

Blizz comes with a task runner out of the box that's designed to work within project-defined tasks.

**Project-level tasks** (`blizz.yaml` at repo root):
```yaml
# ./blizz.yaml: Project task file

test:
  - cargo test --workspace
  - npm test
  - yarn test-coverage
  - etc

setup:
  - yarn install
  - yarn tsc
  - ./scripts/get-messages.sh

deploy:
  - blizz do test
  - docker build -t app:latest .

# etc...
```

**Personal project tasks** (`.cursor/blizz.yaml`):
```yaml
# .cursor/blizz.yaml: Your personal task file

# You can override project level tasks
test:
  - export RUST_BACKTRACE=1  # You like verbose errors
  - cargo test --workspace
  - npm test
  - yarn test-coverage
  - etc

# You can also add additional tasks on top of the project level tasks and hook into them
my-setup:
  - docker stop old-postgres || true
  - export NODE_OPTIONS="--max-old-space-size=8192"
  - do: setup # invokes project "setup" task
```

### Secrets

<div class="note">
<h3>
Note
</h3>
<span>
If you're just dipping your toes into MCP servers or Password managers, see [here](/TODO: find a trustworthy external link/) for a useful guide on MCPs, and read [here](/TODO: find a trustworthy external link/) for the benefits of using a password manager for access tokens and API keys.
</span>
</div>

If you're running MCPs and would like to extract your access tokens and API keys into a specialized local vault, follow the instructions below.

When the `secrets` CLI is useful:

- When your current setup uses vaults like the 1password or dashlane CLIs, which require a separate system authentication for every MCP server spinning up.
- When you want to pull plain text keys out of your configs for a better experience rotating them down the line.
- When you want to configure and run an MCP server from a separate shell script to clean up your `mcp.json`

[TODO] finish up this section


## Understanding the Insights System

### Insights Are Just Markdown Files
```bash
# Your AI agent's memory is stored as readable files
ls ~/.blizz/persistent/insights/

# If desired, you can inspect, backup, or modify with standard tools
grep -r "authentication" ~/.blizz/persistent/insights/
git init ~/.blizz/persistent/insights/  # Version control AI memory
```

### Team Collaboration
```bash
# Teams can share AI knowledge via git
cd ~/.blizz/persistent/insights/
git clone git@company.com:team/shared-insights.git .

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
# First, store your GitHub token securely
secrets store github access_token

# Method 1: Direct embedding (simplest)
{
  "mcpServers": {
    "github": {
      "command": "bash",
      "args": ["-c", "GITHUB_PERSONAL_ACCESS_TOKEN=$(secrets read github access_token) npx -y @modelcontextprotocol/server-github"]
    }
  }
}

# Method 2: Wrapper script (for complex setups)
echo '#!/bin/bash
export GITHUB_PERSONAL_ACCESS_TOKEN=$(secrets read github access_token)
npx -y @modelcontextprotocol/server-github' > ~/.blizz/github-mcp.sh
chmod +x ~/.blizz/github-mcp.sh
```

**Jira MCP** - AI agent remembers past incidents:
```bash
# First, store your Jira credentials securely
secrets store jira email
secrets store jira api_token

# Method 1: Direct embedding (simplest)
{
  "mcpServers": {
    "jira": {
      "command": "bash",
      "args": ["-c", "JIRA_EMAIL=$(secrets read jira email) JIRA_API_TOKEN=$(secrets read jira api_token) JIRA_INSTANCE_URL=https://yourcompany.atlassian.net npx -y @modelcontextprotocol/server-jira"]
    }
  }
}

# Method 2: Wrapper script (for complex setups)
cat > ~/.blizz/jira-mcp.sh << 'EOF'
#!/bin/bash
export JIRA_EMAIL=$(secrets read jira email)
export JIRA_API_TOKEN=$(secrets read jira api_token)
export JIRA_INSTANCE_URL="https://yourcompany.atlassian.net"
npx -y @modelcontextprotocol/server-jira
EOF
chmod +x ~/.blizz/jira-mcp.sh
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