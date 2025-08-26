use clap::Parser;

#[derive(Parser)]
#[command(name = "adam")]
#[command(
  about = "Adam - Insight Management & Scoring\nKnowledge curation, scoring, and consolidation for development teams"
)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), ", courtesy of Blizz and Kernelle Software"))]
struct Cli {
  // For now, we'll just accept the version flag and show the under construction message
  // Additional commands can be added later as the tool is developed
}

fn main() {
  let _cli = Cli::parse();

  println!("Adam - Insight Management & Scoring");
  println!("Knowledge curation, scoring, and consolidation for development teams");
  println!("Under construction...");
}
