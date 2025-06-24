use gitlab::{GitlabBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = "https://gitlab.com";
    let token = "test_token";
    
    println!("Testing GitlabBuilder with host: {}", host);
    
    let client = GitlabBuilder::new(host, token).build_async().await?;
    
    println!("GitLab client created successfully");
    
    Ok(())
}
