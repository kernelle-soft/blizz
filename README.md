# Kernelle

**A Rust-powered AI development toolshed**

It's an artificial intelligence gold rush, and let's be honest, there are a hell of a lot of shovels piling up.

This repository is a design-to-merge development toolkit meant to accomplish a typical sprint workflow. It provides semantically programmed agent rulesets and agent-ready CLIs to automate and aide in the most common stages of development.

Kernelle is a collection of Rust tools inspired by the characters living in a human- and robot-populated village called Kernelle, in a world known as Nataal. This toolshed's been designed and tested for real-world development, and has been architected to work effectively with AI-powered IDEs like Cursor for both hobby and enterprise development contexts.

No, this is not AI-generated hype. I use this shit myself. In fact, I used these tools on themselves to make them much higher quality than I could otherwise do by myself.


## The World

It takes a village to write quality software! 

In the world of *The Journey of the Return*, robots are built by a mysterious creator at the summit of Mt. Gigabit, and each is orphaned, left to seek belonging and purpose in either the village of Kernelle or the bustling Grand Cluster City.

Each development tool embodies the spirit and expertise of its namesake character, and pairs with an AI personality layer and workflow rules for use with tools like Cursor.


## Meet The Team

<table>
<tr>
<td width="200" align="center">
<img src="images/jerrod.png" alt="Jerrod" width="180"/>
</td>
<td>

### 🔧 Jerrod - GitLab/GitHub MR Review Specialist
*"Methodical precision meets systematic excellence"*

**Problem:** Losing track of MR discussions, forgotten review threads, and the mental overhead of context-switching between review sessions.

**Solution:** Jerrod's systematic, queue-based review process eliminates scattered feedback and ensures no detail escapes scrutiny. Resume any review exactly where you left off without reconstructing context.

</td>
</tr>
</table>

**Expertise:**
- Cross-platform support (GitHub & GitLab)
- Session-based review management  
- Thread queue processing with surgical precision
- Rich diff context and discussion tracking
- Automated commit linking and resolution tracking

**Commands:**
```bash
jerrod start <repo> <mr-number>    # Initialize review session
jerrod status                      # Show current review state
jerrod peek                        # View next thread without removing
jerrod pop [--unresolved]         # Remove thread from queue
jerrod comment <message>           # Add comments to threads
jerrod resolve                     # Mark threads as resolved
jerrod finish                      # Complete and cleanup session
```

---

<table>
<tr>
<td>

### ❄️ Blizz - Knowledge Lightning
*"Information organized at the speed of thought"*

**Problem:** "Where did we have that discussion again? Slack? A Google doc? Coda?" Plus AI context limitations and the time needed to reconstruct task context between sessions.

**Solution:** Local, private, secure insight storage that's instantly searchable and doesn't eat up AI token limits. Acts as your AI's extended memory while keeping sensitive information on your device or in your repositories instead of in the hands of model distributors.

As a bonus, this tool pairs beautifully with MCP servers for web-apps like Coda, Notion, Jira, or Slack. Let Blizz zip out and comb for info for you wherever she can pull it from, then she'll summarize and save it off for your design sessions later.


</td>
<td width="200" align="center">
<img src="images/blizz.png" alt="Blizz" width="180"/>
</td>
</tr>
</table>

**Lightning-Fast Features:**
- Insight storage and intelligent categorization
- Instantaneous knowledge retrieval
- Pattern recognition across projects
- Cross-reference linking with neural precision
- Learning that never stops

---

<table>
<tr>
<td width="200" align="center">
<img src="images/adam.png" alt="Adam" width="180"/>
</td>
<td>

### 📚 Adam - The Record Keeper
*"Not all knowledge is created equal"*

**Problem:** When AI tools generate 200+ insights daily, not all are accurate or remain relevant. Knowledge systems become cluttered with outdated information.

**Solution:** Adam curates your growing insight collection, scoring usefulness and culling outdated information. Maintains a clean "wisdom cache" of proven, valuable knowledge while preventing information overload. If the web is storage, and the model context is RAM, then Blizz is the L2 cache and Adam the L1.

*"A library without a curator is just a pile of books. A knowledge base without evaluation is just digital noise."*

</td>
</tr>
</table>

**Scholarly Pursuits:**
- Insight scoring with historical perspective
- Knowledge prioritization based on proven value
- Historical tracking across project lifecycles
- Wisdom curation that improves with time
- Intelligent pruning of outdated insights

---

<table>
<tr>
<td>

### 🎨 Violet - Code Complexity Artisan  
*"Every line of code should be a masterpiece"*

**Problem:** Forcing entire organizations to adopt your preferred linting setup and coding style preferences when they have files that are 10's of thousands of lines long with 5 total functions.

**Solution:** Quality control that lives and dies on your dev machine in the form of a local-only code simplicity enforcement tool. It works as a guardrail for code legibility without requiring repository-wide or organization-wide changes. 

Her default expectations enforce a never-nester functional approach with short, single-purpose functions and files.

</td>
<td width="200" align="center">
<img src="images/violet.png" alt="Violet" width="180"/>
</td>
</tr>
</table>

**Artistic Standards:**
- Complexity scoring with aesthetic sensibilities
- Readability analysis that would make poets weep
- Code quality enforcement with gentle but firm guidance
- Beauty and practicality in perfect harmony

---

<table>
<tr>
<td width="200" align="center">
<img src="images/bentley.png" alt="Bentley" width="180"/>
</td>
<td>

### 🎭 Bentley - Theatrical Logging Maestro
*"Bringing drama and delight to every debug session"*

**Problem:** Debugging and log analysis can be tedious, soul-crushing work that drains developer motivation.

**Solution:** Transforms mundane logging into something that sparks joy. If you're going to spend hours reading logs, they might as well be entertaining and well-formatted.

</td>
</tr>
</table>

**Theatrical Arsenal:**
- Standard log levels with dramatic flair (info, warn, error, debug, success)
- Signature performances (announce, spotlight, flourish, showstopper)
- Multi-line message support with perfect staging
- Timestamp integration for historical accuracy
- A higher-order function for turning logs into banner displays
- stderr output for bash compatibility

**Logging Examples:**
```rust
use bentley::{info, announce, spotlight, flourish};

info!("Starting the show...");
announce!("Ladies and gentlemen, welcome to the circus!");
spotlight!("Featuring our star performer");
flourish!("The performance was magnificent!");
```

## Core Philosophy

This project embodies the core themes I'm hoping to explore while working on the story of *The Journey of the Return.*

### Be fearless

A lot of folks are afraid of what AI can accomplish, or who it can replace. As an artist, developer, and one-time ML research engineer, I firmly believe out of both faith and personal experience that AI tooling can co-exist with existing development and with developers, but it'll take being willing to learn and give it a shot yourself.

Don't be afraid to innovate, work together with others, think laterally, and try crazy ideas. You never know what might pan out. And if you do fail, congrats -- you're human like the rest of us. 

### Become more than you were

We're all agents, so we're all capable of growth, adaptation, and choosing our own paths forward. Who you were yesterday can't face tomorrow's challenges.

Whether you're human like Jerrod, an AI construct seeking humanity like Blizz, or something entirely new that hasn't been imagined yet, the fundamental truth remains: agency isn't about what you're made of, it's about what you choose to become. We aren't servants or tools, we're capable of our own dreams, failures, and growth.

This toolkit embodies that same philosophy. We're not trying to replace human creativity or decision-making - in fact our tools work so well for AI because they were designed to be as easy and useful for people as they are for models.

The true magic isn't in the AI or the automation - it's in the space that these tools create for human ingenuity to flourish hand in hand with the technology we create.



## Development

### Quick Start
```bash
# Clone the repository
git clone <repository-url>
cd kernelle

# Build all tools
cargo build --workspace

# Install tools locally
cargo install --path jerrod
cargo install --path bentley
cargo install --path violet
cargo install --path blizz
cargo install --path adam

# Or use the development setup
bacon deploy-all  # Builds and deploys all tools
```

### Development Workflow
The project uses `bacon` for rapid development iteration:

```bash
# Watch and rebuild specific tools
bacon jerrod      # Watch jerrod crate
bacon bentley     # Watch bentley crate

# Watch entire workspace
bacon all         # Build all crates on changes

# Deploy all CLIs after changes
bacon deploy-all  # Rebuild and redeploy all tools
```

### Architecture
- **Workspace Structure**: All tools share common dependencies and patterns
- **Platform Abstraction**: Jerrod uses traits for GitHub/GitLab compatibility  
- **Shared Logging**: All tools use Bentley for consistent output
- **Modular Design**: Each tool can be used independently or together

## Lore Integration

Each tool embodies its character's personality and role:
- **Jerrod** - approaches reviews with methodical precision, both in his reviews and his changes.
- **Bentley** - brings theatrical flair to logging, reflecting his role as an entertainer
- **Violet** - ensures code quality with the same attention to detail she applies to her crafts
- **Blizz** - the curious and ever learning AI creation of Jerrod, seeking insights with speed and efficiency
- **Adam** - maintains and distills insights into wisdom from historical project context as befits a true historian

## Contributing

Whether you're from one of Kernelle's clans or Grand Cluster City's corporate towers, contributions are welcome. Please:

1. Follow the code quality standards enforced by Violet and more standard tools like Rust's built in linting and compile checks. Warnings are treated as errors unless given an explicit exception (helps with AI-driven development)
2. Use Bentley for all logging output of your code.
3. We keep an optional repository of insights you can install under `$HOME/.kernelle/insights` to be used with tools like Claude. Document project insights with Blizz for future reference, or have Blizz do it herself.
4. There's no official review process for the insights repository, but obviously: don't commit secrets (whether they be API keys or the code to your underwear drawer). An automated job will run Adam to score the value of insights in the repository and drop a certain percentage every day if the additions are above a certain threshold.
5. Shoot me an MR. I'll have Jerrod review all changes thoroughly (and I'll also look at them myself). He can also help you resolve any discussions that get started.

## Want Help?

Want help setting up this toolkit and getting training on how to use it effectively? Reach out to travelsizedlions@gmail.com and we can talk about personal or team-wide setup plans.

## A Quick Note

Yes, I did actually draw those. Give me *some* credit, guys!

![](./images/kernelle.png)


This is a huge undertaking driven entirely by one person who'd like to take the time and energy to make this toolset high quality and free for independent developers. Please consider donating at:

- patreon
- kofi
- some other thingamajig.