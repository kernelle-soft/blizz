use anyhow::Result;

pub async fn handle(_text: String, _new: bool) -> Result<()> {
    bentley::info("Comment functionality not yet implemented");
    bentley::info("This will add comments to threads or MR when completed");
    Ok(())
} 