# Kernelle

**A Rust-powered AI development toolshed**

It's an artificial intelligence gold rush, and let's be honest, there are a hell of a lot of shovels piling up.

This repository is a start-to-end development toolkit meant to provide semantically programmed agent workflows and AI-ready CLIs. 

Kernelle is a collection of Rust tools inspired by the characters and world of a human- and robot-populated village called Kernelle in a world known as Nataal. This toolshed's been designed and tested for real-world development, and has been architected to work effectively with AI-powered IDEs like Cursor for both hobby and enterprise development contexts.


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

**The Engineer's Engineer** - Jerrod approaches every merge request with the methodical precision that made him legendary in Kernelle's Engineering Clan. His systematic, queue-based review process ensures no detail escapes scrutiny.

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

**The Swift Curator** - Jerrod's AI creation, Blizz embodies pure curiosity and lightning-fast organization. She manages your project's collective wisdom with the efficiency that comes from being purpose-built for knowledge work.

*"Every insight is a snowflake - unique, beautiful, and part of something much larger."*

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

### 📚 Adam - The Wisdom Keeper
*"Not all knowledge is created equal"*

**The Chief Historian** - Adam weighs the value of every piece of knowledge with the gravity befitting Kernelle's most respected Historian. His algorithms don't just store information - they distill it into wisdom.

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

**The Aesthetic Perfectionist** - Violet brings the same meticulous attention to detail that she applies to her crafts to your codebase. Her complexity analysis ensures your code remains as elegant and readable as her finest artwork.

*"Code that's hard to read is like a painting with muddy colors - technically functional, but missing its soul."*

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
- Seamless integration with review workflows
- Beauty and function in perfect harmony

---

<table>
<tr>
<td width="200" align="center">
<img src="images/bentley.png" alt="Bentley" width="180"/>
</td>
<td>

### 🎭 Bentley - Theatrical Logging Maestro
*"Bringing drama and delight to every debug session"*

**The Ringmaster of Code** - From his days entertaining fellow Orphans in the streets to founding Overclock Park, Bentley transforms mundane logging into theatrical performance. Every message becomes part of the grand show!

</td>
</tr>
</table>

**Theatrical Arsenal:**
- Standard log levels with dramatic flair (info, warn, error, debug, success)
- Signature performances (announce, spotlight, flourish, showstopper)
- Multi-line message support with perfect staging
- Timestamp integration for historical accuracy
- Banner displays for major announcements
- stderr output for bash compatibility

**Performance Examples:**
```rust
use bentley::{info, announce, spotlight, flourish};

info!("Starting the show...");
announce!("Ladies and gentlemen, welcome to the circus!");
spotlight!("Featuring our star performer");
flourish!("The performance was magnificent!");
```

## Core Philosophy

This project embodies the core themes I'm hoping to explore in the story of *The Journey of the Return.*

### Be fearless

A lot of folks are afraid of what AI can accomplish, or who it can replace. As an artist, developer, and one-time ML research engineer, I firmly believe out of both faith and personal experience that AI tooling can co-exist with existing development and with developers, but it'll take being willing to learn and give it a shot yourself.

Don't be afraid to innovate, work together with others, think laterally, and try crazy ideas. You never know what might pan out. And if you do fail, congrats -- you're human like the rest of us. 

### Become more than you were

Who you were yesterday can't face tomorrow's challenges. But we're all agents, so we're all


- **"Becoming more than you were"** - Strive for excellence in craftsmanship
- **"Let the stranger be thy brother"** - Welcome new contributors
- **"Waste not thy soul"** - Don't create destructive or wasteful code

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

## Donations

This is a huge undertaking driven entirely by one person, who'd like to take the time and energy to make this toolset free for everyone--individual and enterprise. Please consider donating at:

- patreon
- kofi
- some other thingamajig.

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

Yes, I did actually draw those. Give me *some* credit, guys. It might take a village, but I made the damn village, so

![](./images/kernelle.png)