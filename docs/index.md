# Blizz: AI Development That Actually Works

**Status: Open Alpha** | [Join the Alpha](https://github.com/kernelle-soft/blizz/discussions) | [Report Issues](https://github.com/kernelle-soft/blizz/issues)

Blizz makes AI-assisted development work in the real world. Instead of starting from scratch every session, your AI agents get persistent memory, context about your codebase, and personal configuration that follows you from project to project.

**The Problem:** AI assistants forget everything between sessions, can't access your credentials, don't understand your "works on my machine" setup, and give generic advice that doesn't fit your actual codebase.

**The Solution:** AI agents that remember, understand your environment, and adapt to your real-world development workflow.

## 🚀 Quick Start

**Installation (30 seconds)**
```bash
curl -fsSL https://raw.githubusercontent.com/kernelle-soft/blizz/refs/heads/dev/scripts/install.sh | sh
source ~/.blizz.source
```

**Setup Your First Project (2 minutes)**
```bash
# Link AI agent rulesets and tools to your project
cd /path/to/your/project
blizz link

# Your AI agent now has access to:
# - Local RAG for persistent memory (insights)
# - Code quality analysis with actionable output (violet)
# - Secure credential access for MCPs (secrets)
# - Your personal task automation (blizz tasks)
```

**What Your AI Agent Can Now Do:**
- Remember context from previous conversations about this codebase
- Run code quality analysis and suggest specific improvements
- Access API keys and tokens securely for MCP integrations
- Execute your personal development workflows and workarounds

[**Complete Getting Started Guide →**](./getting-started.md)

## What Makes Blizz Different

### AI Agents That Remember Everything
Your AI assistant builds persistent context about your entire organizational ecosystem through local RAG. Through MCPs and extensions, it pulls knowledge from GitHub, Jira, Confluence, Slack, and anywhere else it can reach - then stores it locally for instant access. No more explaining the same architecture decisions, past incidents, or team context every session.

### Personal + Team Configuration Layers
Project configs for team standards, personal configs for your "works on my machine" workarounds. Both merge seamlessly and can follow you between projects.

### Real-World Development Focus
Built for messy, legacy codebases with weird deployment requirements, not toy demo projects.

### An AI-enabled Tool Ecosystem
- **Insights**: 100% local RAG search engine built on a simple markdown system. AI agents get semantic search, humans get hackable text files that work with git, grep, and standard Unix tools
- **Violet**: Patent-pending code analysis with specific,actionable output to guide automatic code generation and refactoring
- **Secrets**: Secure MCP credential access without the need for pasting secrets plain-text into your MCP configurations
- **Blizz**: Personal + team task automation with configuration layering
- **Complimentart, Customizable Rules**: Battle-tested AI behaviors you can take and adapt to your actual workflow needs

## 📚 Documentation

- [**Getting Started**](./getting-started.md) - Complete setup and first workflows
- [**Examples**](./examples.md) - Real-world usage patterns
- [**Team Collaboration**](./team-collaboration.md) - Git-based knowledge sharing workflows
- [**Organizational Knowledge**](./organizational-knowledge.md) - Integrating with enterprise systems
- [**Alpha Guide**](./alpha.md) - Current limitations and how to provide feedback

## 🏢 Enterprise Value

**The "Works On Everyone Else's Machine" Problem, Solved:**
- Developers can add personal workarounds without polluting team configs
- AI agents understand both team standards, current project sticking points, and individual environment quirks
- New team members get consistent AI assistance that adapts to their setup
- Reduce support burden from environment-specific issues

**Consistent AI Assistance Across Teams:**
- AI agents that understand your full organizational context (past incidents, architectural decisions, team processes)
- Shared insights that aggregate knowledge from GitHub, Jira, Confluence, Slack, and other enterprise tools
- Standardized code quality enforcement via violet + rules
- Secure MCP access to your entire toolchain via centralized credential management
- Scalable from 5-developer teams to enterprise engineering orgs

[Contact us about enterprise pilots →](mailto:jeff@kernelle.net)

## ⚡ Alpha Status

We're currently in **open alpha** and actively seeking feedback from developers and teams. The core functionality is stable and production-ready, but we're refining the user experience and adding enterprise features.

**What works now:**
- AI agents with persistent organizational memory (insights RAG + MCP integrations)
- Personal + team configuration layering (violet, tasks)
- Secure MCP credential management (secrets)
- AI-actionable code quality analysis (violet)
- Cross-project portable AI behaviors (rules)
- Local-first architecture with enterprise scaling path

**What's coming:**
- Enhanced AI agent onboarding workflows
- Team insight sharing and collaboration
- Enterprise deployment and management tools
- Extended MCP ecosystem integrations

[Join the alpha discussion →](https://github.com/kernelle-soft/blizz/discussions)

---

*Built for developers who want AI assistance that adapts to their workflow, not the other way around.*
