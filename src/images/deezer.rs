use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ArtistSearchResponse {
    data: Vec<ArtistSearchResult>,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchResult {
    #[serde(rename = "picture_xl")]
    picture_xl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlbumSearchResponse {
    data: Vec<AlbumSearchResult>,
}

#[derive(Debug, Deserialize)]
struct AlbumSearchResult {
    #[serde(rename = "cover_xl")]
    cover_xl: Option<String>,
}

pub struct DeezerImageClient {
    client: reqwest::Client,
}

impl DeezerImageClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_artist_image(&self, artist_name: &str) -> Result<Option<String>> {
        let search_url = format!(
            "https://api.deezer.com/search/artist?q={}",
            urlencoding::encode(artist_name)
        );

        let response = self
            .client
            .get(&search_url)
            .send()
            .await?
            .json::<ArtistSearchResponse>()
            .await?;

        if let Some(artist) = response.data.first() {
            return Ok(artist.picture_xl.clone());
        }

        Ok(None)
    }

    pub async fn fetch_album_image(
        &self,
        artist_name: &str,
        album_name: &str,
    ) -> Result<Option<String>> {
        let search_query = format!("artist:\"{}\" album:\"{}\"", artist_name, album_name);
        let search_url = format!(
            "https://api.deezer.com/search/album?q={}",
            urlencoding::encode(&search_query)
        );

        let response = self
            .client
            .get(&search_url)
            .send()
            .await?
            .json::<AlbumSearchResponse>()
            .await?;

        if let Some(album) = response.data.first() {
            return Ok(album.cover_xl.clone());
        }

        Ok(None)
    }
}
