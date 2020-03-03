mod component;
mod rpc;
mod rekordbox;
mod utils;

use crate::component::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();

    app.run().await;

    Ok(())
}
