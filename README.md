# Kernelle

![Code Coverage](https://img.shields.io/badge/Code%20Coverage-44%25-critical?style=flat)
![Linux](https://github.com/TravelSizedLions/kernelle/actions/workflows/linux.yml/badge.svg?branch=dev)
![Mac OS](https://github.com/TravelSizedLions/kernelle/actions/workflows/macos.yml/badge.svg?branch=dev)
![Windows](https://github.com/TravelSizedLions/kernelle/actions/workflows/windows.yml/badge.svg?branch=dev)

**A Rust-powered, investigation-to-merge AI development toolshed**

It's an artificial intelligence gold rush, and let's be honest, there are a heck of a lot of shovels piling up.

Instead of building yet another shovel to throw on the pile, this repository is essentially a cohesive toolshed to actually take advantage of AI properly. This isn't vibe coding. It's an entire ecosystem designed to make integrating AI into real world development a practical reality.

This suite provides semantically programmed agent rulesets that leverage a host of small, focused CLIs that are both human friendly and usable by reasoning models.

This toolshed's been designed and tested with strict standards to keep real-world development needs in mind, and has been architected to work effectively with AI-powered IDEs like Cursor for both hobby and enterprise development contexts.

Most importantly, Kernelle is designed to have effectively no footprint on existing repositories. Everything needed to supercharge AI to work with a repository lives either under a single folder at the project level that can be added to the project's `.gitignore`, or under `~/.kernelle`, where it can be carried from project to project as needed.

## Setup

```bash
# Clone the repository
git clone <repository-url>
cd kernelle

# Install all tools initially
./scripts/install.sh

# Add ~/.kernelle.source to your shell configs
echo "source ~/.kernelle.source" >> ~/.zshrc && source ~/.zshrc

# CLIs are now globally available
kernelle -h
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
bacon kernelle  # watches kernelle crate
bacon blizz     # watches blizz crate
bacon violet    # etc
```

From there, bacon will spin up the rust equivalent of a development server to watch, rebuild, and re-install the tool you're working on to your PATH so that you never have to think about re-`source`ing or re-installing it yourself. Each CLI is designed to be harmless to the rest of your operating environment, so there's no risk of side-effects to your system. 

**Keep the bacon servers running!** Once you're done with your changeset, switching back to the `dev` branch will automatically rebuild and re-install the tools as they are in `dev`.

### Contributing

1. Follow the code quality standards enforced by Violet and more universal tools like Rust's built in linting and compile checks. Warnings are treated as errors unless given an explicit exception (helps with AI-driven development).
2. Use Bentley for all logging output of your code that you intend to ship.
3. Shoot me a PR whenever.
