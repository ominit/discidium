use anyhow::Result;
use api::client::Client;
// use ui::create_ui;

mod api;
mod ui;

fn main() -> Result<()> {
    // create_ui();
    test()?;
    Ok(())
}

fn test() -> Result<()> {
    let token = "";
    let client = Client::from_user_token(token);
    let connection = client.connect()?;
    Ok(())
}
