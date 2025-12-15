// Script to generate fake jazz data for demo screenshots
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, Result};
use std::env;

// Famous jazz artists
const JAZZ_ARTISTS: &[&str] = &[
    "Miles Davis",
    "John Coltrane",
    "Charlie Parker",
    "Thelonious Monk",
    "Bill Evans",
    "Sonny Rollins",
    "Dizzy Gillespie",
    "Art Blakey",
    "Herbie Hancock",
    "Wayne Shorter",
    "Cannonball Adderley",
    "Chet Baker",
    "Dave Brubeck",
    "Coleman Hawkins",
    "Dexter Gordon",
    "Charles Mingus",
    "Wes Montgomery",
    "Stan Getz",
    "Oscar Peterson",
    "Ella Fitzgerald",
];

// Famous jazz standards (track name, common album)
const JAZZ_STANDARDS: &[(&str, &str)] = &[
    ("All Blues", "Kind of Blue"),
    ("So What", "Kind of Blue"),
    ("Freddie Freeloader", "Kind of Blue"),
    ("Blue in Green", "Kind of Blue"),
    ("Flamenco Sketches", "Kind of Blue"),
    ("My Favorite Things", "My Favorite Things"),
    ("Giant Steps", "Giant Steps"),
    ("Naima", "Giant Steps"),
    ("Take Five", "Time Out"),
    ("Blue Rondo à la Turk", "Time Out"),
    ("Round Midnight", "Genius of Modern Music"),
    ("Straight, No Chaser", "Genius of Modern Music"),
    ("Autumn Leaves", "Portrait in Jazz"),
    ("Waltz for Debby", "Sunday at the Village Vanguard"),
    ("Solar", "Walkin'"),
    ("Moanin'", "Moanin'"),
    ("Blue Monk", "Monk's Music"),
    ("Well You Needn't", "Monk's Music"),
    ("Stella by Starlight", "Sonny Rollins Plus 4"),
    ("St. Thomas", "Saxophone Colossus"),
    ("A Night in Tunisia", "The Complete RCA Victor Recordings"),
    ("Groovin' High", "Dizzy Gillespie"),
    ("Summertime", "Porgy and Bess"),
    ("The Girl from Ipanema", "Getz/Gilberto"),
    ("Cantaloupe Island", "Empyrean Isles"),
    ("Maiden Voyage", "Maiden Voyage"),
    ("Watermelon Man", "Takin' Off"),
    ("Footprints", "Adam's Apple"),
    ("Now's the Time", "The Complete Savoy Sessions"),
    ("Billie's Bounce", "The Complete Savoy Sessions"),
    ("Confirmation", "Bird: The Complete Charlie Parker on Verve"),
    ("Ornithology", "Charlie Parker with Strings"),
    ("A Love Supreme", "A Love Supreme"),
    ("Acknowledgement", "A Love Supreme"),
    ("Resolution", "A Love Supreme"),
    ("Pursuance", "A Love Supreme"),
    ("Psalm", "A Love Supreme"),
    ("Body and Soul", "Body and Soul"),
    ("Cherokee", "Cherokee"),
    ("All the Things You Are", "The Jazz Giants '56"),
    ("How High the Moon", "Jazz at Massey Hall"),
    ("I Got Rhythm", "Bebop"),
    ("Salt Peanuts", "Shaw 'Nuff"),
    ("Epistrophy", "Genius of Modern Music Vol. 2"),
    ("Lush Life", "Lush Life"),
    ("In a Sentimental Mood", "Duke Ellington & John Coltrane"),
    ("Afro Blue", "Olé Coltrane"),
    ("Blue Train", "Blue Train"),
    ("Moment's Notice", "Blue Train"),
];

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let db_path = if args.len() > 1 {
        &args[1]
    } else {
        "demo_jazz.db"
    };

    println!("Creating demo database at: {}", db_path);

    // Remove existing database if it exists
    let _ = std::fs::remove_file(db_path);

    let conn = Connection::open(db_path)?;

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scrobbles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            artist TEXT NOT NULL,
            album TEXT,
            track TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            source TEXT NOT NULL,
            source_id TEXT,
            UNIQUE(artist, track, timestamp, source)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timestamp ON scrobbles(timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_artist ON scrobbles(artist)",
        [],
    )?;

    // Create image cache table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS image_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            entity_name TEXT NOT NULL,
            entity_album TEXT,
            image_url TEXT,
            image_size TEXT NOT NULL,
            fetched_at INTEGER NOT NULL,
            last_accessed INTEGER NOT NULL,
            UNIQUE(entity_type, entity_name, entity_album, image_size)
        )",
        [],
    )?;

    // Generate scrobbles over the past year
    let now = Utc::now();
    // Use a deterministic seed for reproducible demo data
    let mut rng_state = 12345u64; // Simple pseudo-random number generator seed

    let mut scrobble_count = 0;

    println!("Generating scrobbles for the past year...");

    // Generate approximately 5000-6000 scrobbles over the past year
    for days_ago in 0..365 {
        let day = now - Duration::days(days_ago);
        
        // Vary number of scrobbles per day (5-25 tracks per day)
        let daily_scrobbles = 5 + (lcg(&mut rng_state) % 21);

        for _ in 0..daily_scrobbles {
            // Pick a random artist (weighted towards top artists)
            let rand_val = lcg(&mut rng_state);
            let artist_idx = if rand_val % 100 < 40 {
                // 40% chance: top 5 artists
                (lcg(&mut rng_state) % 5) as usize
            } else if rand_val % 100 < 70 {
                // 30% chance: next 5 artists
                (5 + lcg(&mut rng_state) % 5) as usize
            } else {
                // 30% chance: any artist
                (lcg(&mut rng_state) % JAZZ_ARTISTS.len() as u64) as usize
            };
            
            let artist = JAZZ_ARTISTS[artist_idx];

            // Pick a random track
            let track_idx = (lcg(&mut rng_state) % JAZZ_STANDARDS.len() as u64) as usize;
            let (track, album) = JAZZ_STANDARDS[track_idx];

            // Random time during the day
            let hours_offset = lcg(&mut rng_state) % 24;
            let minutes_offset = lcg(&mut rng_state) % 60;
            let timestamp = day
                - Duration::hours(hours_offset as i64)
                - Duration::minutes(minutes_offset as i64);

            // Insert scrobble
            conn.execute(
                "INSERT OR IGNORE INTO scrobbles (artist, album, track, timestamp, source, source_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    artist,
                    album,
                    track,
                    timestamp.timestamp(),
                    "demo",
                    format!("demo-{}-{}", timestamp.timestamp(), lcg(&mut rng_state)),
                ],
            )?;

            scrobble_count += 1;
        }

        if days_ago % 50 == 0 {
            println!("Generated scrobbles for {} days ago...", days_ago);
        }
    }

    println!("\nDatabase generation complete!");
    println!("Total scrobbles generated: {}", scrobble_count);

    // Print some statistics
    let top_artists: Vec<(String, i64)> = conn
        .prepare("SELECT artist, COUNT(*) as count FROM scrobbles GROUP BY artist ORDER BY count DESC LIMIT 10")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>>>()?;

    println!("\nTop 10 Artists:");
    for (i, (artist, count)) in top_artists.iter().enumerate() {
        println!("{}. {} - {} plays", i + 1, artist, count);
    }

    let total: i64 = conn.query_row("SELECT COUNT(*) FROM scrobbles", [], |row| row.get(0))?;
    println!("\nTotal scrobbles in database: {}", total);

    Ok(())
}

// Simple Linear Congruential Generator for pseudo-random numbers
fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}
