#![forbid(unsafe_code)]

use anyhow::Result;
use ui::create_ui;

mod api;
mod ui;

fn main() -> Result<()> {
    create_ui()?;
    Ok(())
}
