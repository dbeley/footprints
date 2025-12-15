use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ArtistSearchResponse {
    artists: Vec<ArtistSearchResult>,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroupSearchResponse {
    #[serde(rename = "release-groups")]
    release_groups: Vec<ReleaseGroup>,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroup {
    id: String,
}

pub struct MusicBrainzImageClient {
    client: reqwest::Client,
}

impl MusicBrainzImageClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Footprints/0.1.0 (https://github.com/yourusername/footprints)")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_artist_image(&self, artist_name: &str) -> Result<Option<String>> {
        // Step 1: Search for artist MBID (MusicBrainz ID)
        let search_url = format!(
            "https://musicbrainz.org/ws/2/artist/?query=artist:{}&fmt=json&limit=1",
            urlencoding::encode(artist_name)
        );

        let search_response = self
            .client
            .get(&search_url)
            .send()
            .await?
            .json::<ArtistSearchResponse>()
            .await?;

        if search_response.artists.is_empty() {
            return Ok(None);
        }

        let artist_id = &search_response.artists[0].id;

        // Step 2: Get artist's release groups to find one with cover art
        let release_groups_url = format!(
            "https://musicbrainz.org/ws/2/release-group?artist={}&type=album&fmt=json&limit=10",
            artist_id
        );

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await; // Rate limit: 1 req/sec

        let release_groups_response = self
            .client
            .get(&release_groups_url)
            .send()
            .await?
            .json::<ReleaseGroupSearchResponse>()
            .await?;

        // Step 3: Try to fetch cover art from Cover Art Archive
        for release_group in &release_groups_response.release_groups {
            let cover_art_url = format!(
                "https://coverartarchive.org/release-group/{}/front",
                release_group.id
            );

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // HEAD request to check if image exists
            if let Ok(response) = self.client.head(&cover_art_url).send().await {
                if response.status().is_success() {
                    return Ok(Some(cover_art_url));
                }
            }
        }

        Ok(None)
    }

    pub async fn fetch_album_image(
        &self,
        artist_name: &str,
        album_name: &str,
    ) -> Result<Option<String>> {
        // Search for release group
        let search_query = format!("artist:{} AND releasegroup:{}", artist_name, album_name);
        let search_url = format!(
            "https://musicbrainz.org/ws/2/release-group/?query={}&fmt=json&limit=1",
            urlencoding::encode(&search_query)
        );

        let search_response = self
            .client
            .get(&search_url)
            .send()
            .await?
            .json::<ReleaseGroupSearchResponse>()
            .await?;

        if search_response.release_groups.is_empty() {
            return Ok(None);
        }

        let release_group_id = &search_response.release_groups[0].id;

        // Try to fetch cover art
        let cover_art_url = format!(
            "https://coverartarchive.org/release-group/{}/front",
            release_group_id
        );

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        if let Ok(response) = self.client.head(&cover_art_url).send().await {
            if response.status().is_success() {
                return Ok(Some(cover_art_url));
            }
        }

        Ok(None)
    }
}
