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

#[test]
fn test_init() {
  // Test that init doesn't panic
  init();
}

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

#[test]
fn test_event_functions() {
  // Test timestamped event functions
  event_info("Info event");
  event_warn("Warning event");
  event_error("Error event");
  event_debug("Debug event");
  event_success("Success event");
}
