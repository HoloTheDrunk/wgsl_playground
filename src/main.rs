fn main() {
    let config: wgsl_playground::Config = ron::de::from_reader(std::io::BufReader::new(
        std::fs::File::open("config.ron").expect("Config file should be available"),
    ))
    .expect("Config should be valid");

    pollster::block_on(wgsl_playground::run(config));
}
