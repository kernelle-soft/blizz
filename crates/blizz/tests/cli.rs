use assert_cmd::prelude::*;

use predicates::str::contains;
use predicates::prelude::*;
use serial_test::serial;
use std::process::Command;

/// Helper to create a Command for the `blizz` binary with a temporary insights root.
fn blizz_cmd(insights_dir: &assert_fs::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("blizz").expect("binary exists");
    cmd.env("BLIZZ_INSIGHTS_ROOT", insights_dir.path());
    cmd
}

#[test]
#[serial]
fn test_add_get_list_topics() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Add two insights in different topics
    blizz_cmd(&temp)
        .args(&["add", "topic_one", "insight_a", "Overview A", "Details A"])
        .assert()
        .success()
        .stdout(contains("Added insight"));

    blizz_cmd(&temp)
        .args(&["add", "topic_two", "insight_b", "Overview B", "Details B"])
        .assert()
        .success();

    // List topics should show both topics
    blizz_cmd(&temp)
        .args(&["topics"])
        .assert()
        .success()
        .stdout(contains("topic_one").and(contains("topic_two")));

    // List insights verbose
    blizz_cmd(&temp)
        .args(&["list", "--verbose"])
        .assert()
        .success()
        .stdout(contains("topic_one/insight_a").and(contains("topic_two/insight_b")));

    // Get insight should print overview and details
    blizz_cmd(&temp)
        .args(&["get", "topic_one", "insight_a"])
        .assert()
        .success()
        .stdout(contains("Overview A").and(contains("Details A")));

    temp.close().unwrap();
}

#[test]
#[serial]
fn test_search_update_delete() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Add an insight
    blizz_cmd(&temp)
        .args(&["add", "search_topic", "search_insight", "Search overview", "Initial details"])
        .assert()
        .success();

    // Search (case insensitive)
    blizz_cmd(&temp)
        .args(&["search", "search"])
        .assert()
        .success()
        .stdout(contains("search_topic/search_insight"));

    // Update details
    blizz_cmd(&temp)
        .args(&["update", "search_topic", "search_insight", "--details", "Updated details"])
        .assert()
        .success()
        .stdout(contains("Updated insight"));

    // Get to verify update
    blizz_cmd(&temp)
        .args(&["get", "search_topic", "search_insight"])
        .assert()
        .success()
        .stdout(contains("Updated details"));

    // Delete insight (force)
    blizz_cmd(&temp)
        .args(&["delete", "search_topic", "search_insight", "--force"])
        .assert()
        .success()
        .stdout(contains("Deleted insight"));

    // Ensure search no longer finds it
    blizz_cmd(&temp)
        .args(&["search", "search_insight"])
        .assert()
        .success()
        .stdout(contains("No matches"));

    temp.close().unwrap();
} 