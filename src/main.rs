mod rekordbox;
mod utils;
mod component;
mod rpc;
mod library;

use component::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new("/home/jonas/Music/TermDJ");
    app.run().await;

    Ok(())
}
