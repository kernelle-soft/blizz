![Code Coverage](https://img.shields.io/badge/Code%20Coverage-33%25-critical?style=flat)
![Linux](https://github.com/kernelle-soft/blizz/actions/workflows/linux.yml/badge.svg?branch=dev)
![Mac OS](https://github.com/kernelle-soft/blizz/actions/workflows/macos.yml/badge.svg?branch=dev)

**Blizz: The AI development toolshed**

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

## Development

### Development Workflow

We have a philosophy of developing CLI tools on the PATH. This ensures that development choices are guided by where the tools will ultimately be installed.

The project uses `bacon` for rapid development iteration, configured to make that workflow both safe and trivial:

```bash
# During development on a particular CLI
# you can watch it's crate to re-build it and re-install it to PATH. 
bacon [crate name]

# Example
bacon blizz  # watches blizz crate
bacon insights   # watches insights crate
bacon violet    # etc
```

From there, bacon will spin up the rust equivalent of a development server to watch, rebuild, and re-install the tool you're working on to your PATH so that you never have to think about re-`source`ing or re-installing it yourself. Each CLI is designed to be harmless to the rest of your operating environment, so there's no risk of side-effects to your system. 

**Keep the bacon servers running!** Once you're done with your changeset, switching back to the `dev` branch will automatically rebuild and re-install the tools as they are in `dev`.

### Contributing

1. Follow the code quality standards enforced by Violet and more universal tools like Rust's built in linting and compile checks. Warnings are treated as errors unless given an explicit exception (helps with AI-driven development).
2. Use Bentley for all logging output of your code that you intend to ship.
3. Shoot me a PR whenever.
