use brain_media_downloader::MediaDownloaderPlugin;
use brain_plugin::Plugin;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting NodeBook Media Downloader Standalone Service...");

    let plugin = MediaDownloaderPlugin::new("./downloads");
    info!("Plugin manifest: {:?}", plugin.manifest());

    println!("Media Downloader Service Ready.");
    Ok(())
}
