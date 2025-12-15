use anyhow::Result;
use serde::Deserialize;

use super::types::ImageSize;

#[derive(Debug, Deserialize)]
struct ArtistInfo {
    artist: Artist,
}

#[derive(Debug, Deserialize)]
struct AlbumInfo {
    album: Album,
}

#[derive(Debug, Deserialize)]
struct Artist {
    #[serde(default)]
    image: Vec<Image>,
}

#[derive(Debug, Deserialize)]
struct Album {
    #[serde(default)]
    image: Vec<Image>,
}

#[derive(Debug, Deserialize)]
struct Image {
    #[serde(rename = "#text")]
    url: String,
    size: String,
}

pub struct LastFmImageClient {
    api_key: String,
    client: reqwest::Client,
}

impl LastFmImageClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_artist_image(
        &self,
        artist: &str,
        size: ImageSize,
    ) -> Result<Option<String>> {
        if self.api_key.is_empty() {
            return Ok(None);
        }

        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=artist.getinfo&artist={}&api_key={}&format=json",
            urlencoding::encode(artist),
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<ArtistInfo>()
            .await?;

        Ok(self.extract_image_url(&response.artist.image, size))
    }

    pub async fn fetch_album_image(
        &self,
        artist: &str,
        album: &str,
        size: ImageSize,
    ) -> Result<Option<String>> {
        if self.api_key.is_empty() {
            return Ok(None);
        }

        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=album.getinfo&artist={}&album={}&api_key={}&format=json",
            urlencoding::encode(artist),
            urlencoding::encode(album),
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<AlbumInfo>()
            .await?;

        Ok(self.extract_image_url(&response.album.image, size))
    }

    fn extract_image_url(&self, images: &[Image], size: ImageSize) -> Option<String> {
        images
            .iter()
            .find(|img| img.size == size.as_str())
            .and_then(|img| {
                if img.url.is_empty() {
                    None
                } else {
                    Some(img.url.clone())
                }
            })
    }
}
