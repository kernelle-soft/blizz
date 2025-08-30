use bentley::*;

#[test]
fn test_basic_logging_functions() {
  // Test that basic logging functions can be called without panicking
  info("Test info message");
  warn("Test warning message");
  error("Test error message");
  debug("Test debug message");
  success("Test success message");
}

// init() function removed - no longer needed

#[test]
fn test_multiline_messages() {
  // Test multiline message handling
  let multiline_msg = "First line\nSecond line\nThird line";
  info(multiline_msg);
  warn(multiline_msg);
  error(multiline_msg);
  debug(multiline_msg);
  success(multiline_msg);
}

// event_* functions removed - no longer needed
