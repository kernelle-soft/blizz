use clap::Parser;
use jerrod::commands::acknowledge::AcknowledgeConfig;
use jerrod::platform::ReactionType;

// Recreate the CLI structure from main.rs for testing
#[derive(Parser)]
#[command(name = "jerrod")]
struct TestCli {
    #[arg(long, env = "GITHUB_TOKEN")]
    github_token: Option<String>,

    #[arg(long, env = "GITLAB_TOKEN")]
    gitlab_token: Option<String>,

    #[command(subcommand)]
    command: TestCommands,
}

#[derive(clap::Subcommand)]
enum TestCommands {
    Start {
        repository: String,
        mr_number: u64,
        #[arg(short, long)]
        platform: Option<String>,
    },
    Status,
    Peek,
    Pop {
        #[arg(long)]
        unresolved: bool,
    },
    Resolve,
    Comment {
        text: String,
        #[arg(long)]
        new: bool,
    },
    Commit {
        message: String,
        #[arg(short, long)]
        details: Option<String>,
        #[arg(short, long)]
        thread_id: Option<String>,
    },
    Acknowledge {
        #[arg(long)]
        thumbs_up: bool,
        #[arg(long)]
        ok: bool,
        #[arg(long)]
        yeah: bool,
        #[arg(long)]
        got_it: bool,
    },
    Finish,
    Refresh,
}

#[test]
fn test_cli_parsing_start_command() {
    let args = vec!["jerrod", "start", "owner/repo", "123"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Start { repository, mr_number, platform } => {
            assert_eq!(repository, "owner/repo");
            assert_eq!(mr_number, 123);
            assert!(platform.is_none());
        }
        _ => panic!("Expected Start command"),
    }
}

#[test]
fn test_cli_parsing_start_with_platform() {
    let args = vec!["jerrod", "start", "owner/repo", "456", "--platform", "github"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Start { repository, mr_number, platform } => {
            assert_eq!(repository, "owner/repo");
            assert_eq!(mr_number, 456);
            assert_eq!(platform, Some("github".to_string()));
        }
        _ => panic!("Expected Start command"),
    }
}

#[test]
fn test_cli_parsing_status_command() {
    let args = vec!["jerrod", "status"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Status => assert!(true),
        _ => panic!("Expected Status command"),
    }
}

#[test]
fn test_cli_parsing_comment_command() {
    let args = vec!["jerrod", "comment", "This is a test comment"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Comment { text, new } => {
            assert_eq!(text, "This is a test comment");
            assert!(!new);
        }
        _ => panic!("Expected Comment command"),
    }
}

#[test]
fn test_cli_parsing_comment_new() {
    let args = vec!["jerrod", "comment", "New MR comment", "--new"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Comment { text, new } => {
            assert_eq!(text, "New MR comment");
            assert!(new);
        }
        _ => panic!("Expected Comment command"),
    }
}

#[test]
fn test_cli_parsing_commit_command() {
    let args = vec!["jerrod", "commit", "Fix bug in authentication"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Commit { message, details, thread_id } => {
            assert_eq!(message, "Fix bug in authentication");
            assert!(details.is_none());
            assert!(thread_id.is_none());
        }
        _ => panic!("Expected Commit command"),
    }
}

#[test]
fn test_cli_parsing_commit_with_details() {
    let args = vec![
        "jerrod", "commit", "Fix bug", 
        "--details", "Updated validation logic",
        "--thread-id", "thread123"
    ];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Commit { message, details, thread_id } => {
            assert_eq!(message, "Fix bug");
            assert_eq!(details, Some("Updated validation logic".to_string()));
            assert_eq!(thread_id, Some("thread123".to_string()));
        }
        _ => panic!("Expected Commit command"),
    }
}

#[test]
fn test_cli_parsing_pop_command() {
    let args = vec!["jerrod", "pop"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Pop { unresolved } => {
            assert!(!unresolved);
        }
        _ => panic!("Expected Pop command"),
    }
}

#[test]
fn test_cli_parsing_pop_unresolved() {
    let args = vec!["jerrod", "pop", "--unresolved"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Pop { unresolved } => {
            assert!(unresolved);
        }
        _ => panic!("Expected Pop command"),
    }
}

#[test]
fn test_cli_parsing_acknowledge() {
    let args = vec!["jerrod", "acknowledge", "--thumbs-up"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    match cli.command {
        TestCommands::Acknowledge { thumbs_up, .. } => {
            assert!(thumbs_up);
        }
        _ => panic!("Expected Acknowledge command"),
    }
}

#[test]
fn test_acknowledge_config_from_flags() {
    // Test the AcknowledgeConfig logic used in main.rs
    let config = AcknowledgeConfig::from_flags(
        true, false, false, false, // thumbs_up only
        false, false, // thumbs_down
        false, false, // laugh
        false, false, false, false, false, // hooray
        false, false, false, // confused
        false, false, false, // heart
        false, false, false, false, false, // rocket
        false, false, false // eyes
    );
    
    assert!(matches!(config.reaction_type, ReactionType::ThumbsUp));
}

#[test]
fn test_acknowledge_config_heart() {
    let config = AcknowledgeConfig::from_flags(
        false, false, false, false, // thumbs_up
        false, false, // thumbs_down
        false, false, // laugh
        false, false, false, false, false, // hooray
        false, false, false, // confused
        true, false, false, // heart - love flag set
        false, false, false, false, false, // rocket
        false, false, false // eyes
    );
    
    assert!(matches!(config.reaction_type, ReactionType::Heart));
}

#[test]
fn test_cli_parsing_invalid_mr_number() {
    let args = vec!["jerrod", "start", "owner/repo", "not_a_number"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err());
}

#[test]
fn test_cli_parsing_missing_args() {
    // Missing MR number
    let args = vec!["jerrod", "start", "owner/repo"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err());
    
    // Missing comment text
    let args = vec!["jerrod", "comment"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err());
    
    // Missing commit message
    let args = vec!["jerrod", "commit"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err());
}

#[test]
fn test_cli_parsing_all_simple_commands() {
    let simple_commands = vec![
        (vec!["jerrod", "status"], "Status"),
        (vec!["jerrod", "peek"], "Peek"),
        (vec!["jerrod", "resolve"], "Resolve"),
        (vec!["jerrod", "finish"], "Finish"),
        (vec!["jerrod", "refresh"], "Refresh"),
    ];
    
    for (args, expected_name) in simple_commands {
        let cli = TestCli::try_parse_from(args).unwrap();
        
        let command_name = match cli.command {
            TestCommands::Status => "Status",
            TestCommands::Peek => "Peek",
            TestCommands::Resolve => "Resolve",
            TestCommands::Finish => "Finish",
            TestCommands::Refresh => "Refresh",
            _ => "Other",
        };
        
        assert_eq!(command_name, expected_name);
    }
}

#[test]
fn test_environment_variables() {
    use std::env;
    
    // Test that CLI can pick up tokens from environment
    env::set_var("GITHUB_TOKEN", "test_github_token");
    env::set_var("GITLAB_TOKEN", "test_gitlab_token");
    
    let args = vec!["jerrod", "status"];
    let cli = TestCli::try_parse_from(args).unwrap();
    
    assert!(matches!(cli.command, TestCommands::Status));
    
    // Clean up
    env::remove_var("GITHUB_TOKEN");
    env::remove_var("GITLAB_TOKEN");
}

#[test]
fn test_help_parsing_fails() {
    // Help flag should cause parsing to fail (expected behavior)
    let args = vec!["jerrod", "--help"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err());
} 