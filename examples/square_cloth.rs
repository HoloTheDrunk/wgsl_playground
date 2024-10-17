fn main() -> anyhow::Result<()> {
    pollster::block_on(cloth_sim::run());

    Ok(())
}
