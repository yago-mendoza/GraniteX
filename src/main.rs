mod app;
mod brep;
mod commands;
mod construction;
mod export;
mod import;
mod project;
mod renderer;
mod sketch;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    app::run()
}
