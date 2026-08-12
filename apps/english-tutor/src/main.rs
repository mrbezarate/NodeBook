use brain_english_tutor::EnglishTutorPlugin;
use brain_plugin::Plugin;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting NodeBook English Tutor Standalone Service...");

    let plugin = EnglishTutorPlugin::new();
    info!("Plugin manifest: {:?}", plugin.manifest());

    println!("English Tutor Service Ready.");
    Ok(())
}
