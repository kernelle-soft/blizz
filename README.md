![Code Coverage](https://img.shields.io/badge/Code%20Coverage-68%25-warn?style=flat)
![Linux](https://github.com/kernelle-soft/blizz/actions/workflows/linux.yml/badge.svg?branch=dev)
![Mac OS](https://github.com/kernelle-soft/blizz/actions/workflows/macos.yml/badge.svg?branch=dev)

**Blizz: The nomadic AI development toolshed**

It's an artificial intelligence gold rush, and let's be honest, there are a hell of a lot of shovels piling up.

Most AI dev tools are either toys or are impractical for real development workflows due to security concerns, lack of performance, and low quality. They aren't designed to account for the practical realities of day-to-day development, like factoring in multiple sources of internal information or the need to track evolving design decisions.

So, instead of building yet another shovel to throw on the pile, this repository is a cohesive collection of tools, AI-centric and not, that synergize together to make existing AI-aided development strategies more practical for everyday work.

The core design philosophy of this toolset is to make our tools ***nomadic***: something the developer can take with them from project to project, codebase to codebase, and team to team without leaving a footprint.

The result is that blizz really isn't about vibe coding. It's a _personal context engineering toolset_ designed to adapt to you--an entire ecosystem meant to make automatic code generation a reality for the kind of development you want to do. It includes:
- A 100% local RAG search engine and knowledge base that grows with you and your areas of ownership
- A configurable, language agnostic tool for analyzing code legibility and recommending fixes
- A tool for securely storing and accessing secrets for MCP token authentication
- A linking tool for bringing your custom rules from repo to repo
- A task runner capable of defining, parameterizing, and composing tasks together
- Semantically programmed agent behaviors, rulesets, and modern best practices for coding style, including rules to enable whatever model you're running to use the other tools above.


## Setup

```bash
curl -fsSL https://raw.githubusercontent.com/kernelle-soft/blizz/refs/heads/dev/scripts/install.sh | sh
```
