use bentley::*;

fn main() {
  println!("🎪 Welcome to the Bentley Demo! 🎪\n");

  // Standard logging functions
  info("This is an info message");
  warn("This is a warning message");
  error("This is an error message");
  debug("This is a debug message");
  success("This is a success message");

  println!(); // spacing

  
  println!(); // spacing

  // Theatrical functions - the real showstoppers!
  announce("Ladies and gentlemen, step right up!");
  spotlight("The amazing Bentley takes center stage!");
  flourish("What a spectacular performance!");
  showstopper("THE SHOW MUST GO ON!");

  println!(); // spacing

  // Multi-line message test
  let multiline = "This is a multiline message\nwith several lines\nto demonstrate formatting";
  info(multiline);

  println!("\n🎭 That's all folks! 🎭");
}
