use chrono::{TimeZone, Utc};
use jerrod::display::{
  display_discussion_thread, display_file_context, display_file_diff, display_replies,
  display_thread_header, format_timestamp,
};
use jerrod::platform::{Discussion, FileDiff, Note, User};

fn create_test_user() -> User {
  User {
    id: "user123".to_string(),
    username: "testuser".to_string(),
    display_name: "Test User".to_string(),
    avatar_url: Some("https://example.com/avatar.png".to_string()),
  }
}

fn create_test_note(body: &str) -> Note {
  Note {
    id: "note123".to_string(),
    author: create_test_user(),
    body: body.to_string(),
    created_at: Utc.with_ymd_and_hms(2023, 6, 15, 14, 30, 0).unwrap(),
    updated_at: Utc.with_ymd_and_hms(2023, 6, 15, 14, 30, 0).unwrap(),
  }
}

#[test]
fn test_format_timestamp() {
  let test_time = Utc.with_ymd_and_hms(2023, 6, 15, 14, 30, 0).unwrap();
  let formatted = format_timestamp(test_time);

  // Should contain day, month, date, year and time
  assert!(formatted.contains("June"));
  assert!(formatted.contains("15"));
  assert!(formatted.contains("2023"));
  // Time format varies by locale (24h vs 12h), so just check for reasonable time patterns
  assert!(
    formatted.contains("14:")
      || formatted.contains("2:")
      || formatted.contains("PM")
      || formatted.contains("AM")
  );
}

#[test]
fn test_format_timestamp_different_times() {
  let morning = Utc.with_ymd_and_hms(2023, 1, 1, 9, 15, 0).unwrap();
  let evening = Utc.with_ymd_and_hms(2023, 12, 31, 21, 45, 0).unwrap();

  let morning_fmt = format_timestamp(morning);
  let evening_fmt = format_timestamp(evening);

  assert!(morning_fmt.contains("January"));
  assert!(morning_fmt.contains("2023"));
  assert!(evening_fmt.contains("December"));
  assert!(evening_fmt.contains("2023"));
  assert_ne!(morning_fmt, evening_fmt);
}

#[test]
fn test_display_thread_header_formatting() {
  let note = create_test_note("This is a test comment body");
  let thread_id = "thread_123";

  // This function outputs to stdout, so we can't easily capture it
  // But we can verify it doesn't panic
  display_thread_header(&note, thread_id);
}

#[test]
fn test_display_thread_header_with_long_content() {
  let long_content = "A".repeat(200); // Very long content
  let note = create_test_note(&long_content);
  let thread_id = "long_thread";

  // Should handle word wrapping without panicking
  display_thread_header(&note, thread_id);
}

#[test]
fn test_display_thread_header_with_multiline_content() {
  let multiline_content = "Line 1\nLine 2\nLine 3\nVery long line that should get wrapped because it exceeds the typical width limit of 80 characters per line";
  let note = create_test_note(multiline_content);

  display_thread_header(&note, "multiline_thread");
}

#[test]
fn test_display_file_context_with_line_number() {
  let file_path = "src/main.rs";
  let line_number = Some(42);

  // Should not panic when displaying file context
  display_file_context(file_path, line_number);
}

#[test]
fn test_display_file_context_without_line_number() {
  let file_path = "README.md";
  let line_number = None;

  // Should handle missing line number gracefully
  display_file_context(file_path, line_number);
}

#[test]
fn test_display_replies_with_single_note() {
  let discussion = Discussion {
    id: "disc123".to_string(),
    resolved: false,
    resolvable: true,
    file_path: Some("src/test.rs".to_string()),
    line_number: Some(25),
    notes: vec![create_test_note("Single note")],
  };

  // Should handle single note (no replies) gracefully
  display_replies(&discussion);
}

#[test]
fn test_display_replies_with_multiple_notes() {
  let user1 = User {
    id: "user1".to_string(),
    username: "user1".to_string(),
    display_name: "User One".to_string(),
    avatar_url: None,
  };

  let user2 = User {
    id: "user2".to_string(),
    username: "user2".to_string(),
    display_name: "User Two".to_string(),
    avatar_url: None,
  };

  let note1 = Note {
    id: "note1".to_string(),
    author: user1,
    body: "Original comment".to_string(),
    created_at: Utc.with_ymd_and_hms(2023, 6, 15, 14, 30, 0).unwrap(),
    updated_at: Utc.with_ymd_and_hms(2023, 6, 15, 14, 30, 0).unwrap(),
  };

  let note2 = Note {
    id: "note2".to_string(),
    author: user2,
    body: "Reply comment".to_string(),
    created_at: Utc.with_ymd_and_hms(2023, 6, 15, 15, 0, 0).unwrap(),
    updated_at: Utc.with_ymd_and_hms(2023, 6, 15, 15, 0, 0).unwrap(),
  };

  let discussion = Discussion {
    id: "disc456".to_string(),
    resolved: false,
    resolvable: true,
    file_path: Some("src/lib.rs".to_string()),
    line_number: Some(100),
    notes: vec![note1, note2],
  };

  // Should display replies section
  display_replies(&discussion);
}

#[test]
fn test_display_discussion_thread_complete() {
  let discussion = Discussion {
    id: "complete_thread".to_string(),
    resolved: false,
    resolvable: true,
    file_path: Some("src/components/Button.tsx".to_string()),
    line_number: Some(67),
    notes: vec![
      create_test_note("This component needs refactoring"),
      create_test_note("I agree, let's extract the logic"),
    ],
  };

  // Should display complete thread with header, file context, and replies
  display_discussion_thread(&discussion);
}

#[test]
fn test_display_discussion_thread_no_file_context() {
  let discussion = Discussion {
    id: "general_thread".to_string(),
    resolved: false,
    resolvable: true,
    file_path: None,
    line_number: None,
    notes: vec![create_test_note("General discussion comment")],
  };

  // Should handle threads without file context
  display_discussion_thread(&discussion);
}

#[test]
fn test_display_discussion_thread_empty_notes() {
  let discussion = Discussion {
    id: "empty_thread".to_string(),
    resolved: true,
    resolvable: true,
    file_path: Some("src/empty.rs".to_string()),
    line_number: Some(1),
    notes: vec![],
  };

  // Should handle empty notes gracefully
  display_discussion_thread(&discussion);
}

#[test]
fn test_display_file_diff_basic() {
  let diff = FileDiff {
    old_path: Some("old_file.rs".to_string()),
    new_path: "new_file.rs".to_string(),
    diff: "@@ -1,4 +1,4 @@\n-old line\n+new line\n context line".to_string(),
  };

  // Should display diff with proper formatting
  display_file_diff(&diff);
}

#[test]
fn test_display_file_diff_renamed_file() {
  let diff = FileDiff {
    old_path: Some("old_name.rs".to_string()),
    new_path: "new_name.rs".to_string(),
    diff: "@@ -1,2 +1,2 @@\n context\n-removed\n+added".to_string(),
  };

  // Should show rename information
  display_file_diff(&diff);
}

#[test]
fn test_display_file_diff_new_file() {
  let diff = FileDiff {
    old_path: None,
    new_path: "brand_new_file.rs".to_string(),
    diff: "@@ -0,0 +1,3 @@\n+fn main() {\n+    println!(\"Hello\");\n+}".to_string(),
  };

  // Should handle new file creation
  display_file_diff(&diff);
}

#[test]
fn test_display_file_diff_complex_diff() {
  let complex_diff = r#"@@ -1,10 +1,12 @@
fn main() {
-    let x = 5;
+    let x = 10;
+    let y = 20;
     println!("Hello");
     
-    // Old comment
+    // New comment
     if x > 0 {
         println!("Positive");
     }
+    
+    // Additional code
 }"#;

  let diff = FileDiff {
    old_path: Some("src/main.rs".to_string()),
    new_path: "src/main.rs".to_string(),
    diff: complex_diff.to_string(),
  };

  // Should handle complex diff with multiple changes
  display_file_diff(&diff);
}

#[test]
fn test_display_edge_cases() {
  // Test with empty strings
  let empty_note = Note {
    id: "empty".to_string(),
    author: create_test_user(),
    body: "".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  display_thread_header(&empty_note, "");

  // Test with special characters
  let special_note = Note {
    id: "special".to_string(),
    author: create_test_user(),
    body: "Special chars: 🎉 «»\"\"'' \\n\\t\\r".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  display_thread_header(&special_note, "special_chars");
}
