mod app;
mod brep;
mod commands;
mod import;
mod renderer;
mod sketch;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    app::run()
}
