use std::error::Error;

use rbr_sync_lib::stages;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stages = stages("token", "db_id").await?;
    println!("{stages:?}");
    Ok(())
}
