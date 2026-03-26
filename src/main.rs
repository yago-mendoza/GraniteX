#![allow(dead_code)]

mod app;
mod commands;
mod renderer;
mod sketch;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    app::run()
}
