mod commands;
mod core;

fn main() -> anyhow::Result<()> {
    commands::run()
}
