#[tokio::test]
async fn test_spotify_playlist_extraction() {
    let url = "https://open.spotify.com/playlist/1oEjOO11lwWrALoPE3hBWJ?si=aohmyELKRsGCwtkJC5Nmcw";
    let entity = brain_media_downloader::MediaDownloader::extract_spotify_entity(url).await;
    
    assert!(entity.is_some(), "Entity must be extracted");
    let entity = entity.unwrap();
    println!("Playlist Title: {}", entity.title);
    println!("Tracks count: {}", entity.tracks.len());

    assert_eq!(entity.tracks.len(), 5, "Expected exactly 5 tracks");
    
    for (i, t) in entity.tracks.iter().enumerate() {
        println!("{}. {} — {} ({})", i + 1, t.title, t.artist, t.url);
    }
}

#[tokio::test]
async fn test_spotify_playlist_download_full() {
    let url = "https://open.spotify.com/playlist/1oEjOO11lwWrALoPE3hBWJ?si=aohmyELKRsGCwtkJC5Nmcw";
    let downloader = brain_media_downloader::MediaDownloader::new("./downloads");
    let items = downloader.download_playlist(url, true).await;
    assert!(items.is_ok(), "Download playlist must succeed: {:?}", items.err());
    let items = items.unwrap();
    println!("Successfully downloaded {} tracks!", items.len());
    assert_eq!(items.len(), 5, "Must download all 5 tracks");

    for (idx, (path, item)) in items.iter().enumerate() {
        println!("Track {}: file={:?}, title={}, uploader={:?}, cover={:?}", idx + 1, path, item.title, item.uploader, item.cover_file);
        assert!(path.exists(), "Audio file must exist");
        if let Some(ref cover) = item.cover_file {
            let cover_path = downloader.download_dir().join(cover);
            assert!(cover_path.exists(), "Cover file must exist");
        }
    }
}
