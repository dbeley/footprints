use crate::db::DbPool;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransitionsReport {
    pub transitions: Vec<Transition>,
    pub top_transitions: Vec<Transition>,
    pub network_data: NetworkGraph,
    pub summary: TransitionsSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transition {
    pub from_artist: String,
    pub to_artist: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub weight: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransitionsSummary {
    pub total_transitions: i64,
    pub unique_transitions: usize,
    pub most_common_transition: Option<Transition>,
    pub most_connected_artist: String,
    pub avg_transitions_per_session: f64,
}

pub fn generate_transitions_report(
    pool: &DbPool,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    gap_minutes: i64,
    min_count: i64,
    include_self_transitions: bool,
) -> Result<TransitionsReport> {
    // Get scrobbles from database
    let scrobbles = if let (Some(s), Some(e)) = (start, end) {
        crate::db::get_scrobbles_in_range(pool, s, e)?
    } else {
        crate::db::get_scrobbles(pool, Some(1_000_000), Some(0))?
    };

    // Extract transitions directly from scrobbles
    let mut transition_counts: HashMap<(String, String), i64> = HashMap::new();
    let mut artist_counts: HashMap<String, i64> = HashMap::new();
    let mut session_count = 0;

    if scrobbles.is_empty() {
        return Ok(TransitionsReport {
            transitions: vec![],
            top_transitions: vec![],
            network_data: NetworkGraph {
                nodes: vec![],
                edges: vec![],
            },
            summary: TransitionsSummary {
                total_transitions: 0,
                unique_transitions: 0,
                most_common_transition: None,
                most_connected_artist: String::new(),
                avg_transitions_per_session: 0.0,
            },
        });
    }

    // Process scrobbles and detect transitions based on gap
    let mut prev_scrobble = &scrobbles[0];
    let mut current_session_has_transition = false;

    for curr_scrobble in scrobbles.iter().skip(1) {
        // Calculate gap between consecutive scrobbles in minutes
        let gap = (curr_scrobble.timestamp - prev_scrobble.timestamp).num_minutes();

        // If gap is too large, start a new session
        if gap > gap_minutes {
            if current_session_has_transition {
                session_count += 1;
            }
            current_session_has_transition = false;
        } else {
            // Within same session, count transition
            let from = &prev_scrobble.artist;
            let to = &curr_scrobble.artist;

            // Skip self-transitions if not requested
            if include_self_transitions || from != to {
                let key = (from.clone(), to.clone());
                *transition_counts.entry(key).or_insert(0) += 1;
                current_session_has_transition = true;

                // Count artist appearances
                *artist_counts.entry(from.clone()).or_insert(0) += 1;
            }
        }

        prev_scrobble = curr_scrobble;
    }

    // Count last session if it had transitions
    if current_session_has_transition {
        session_count += 1;
    }

    // Count last artist
    *artist_counts
        .entry(scrobbles.last().unwrap().artist.clone())
        .or_insert(0) += 1;

    // Build transitions list
    let total_transitions: i64 = transition_counts.values().sum();
    let mut transitions: Vec<Transition> = transition_counts
        .iter()
        .filter(|&(_, &count)| count >= min_count)
        .map(|((from, to), &count)| Transition {
            from_artist: from.clone(),
            to_artist: to.clone(),
            count,
            percentage: if total_transitions > 0 {
                (count as f64 / total_transitions as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    // Sort by count descending
    transitions.sort_by(|a, b| b.count.cmp(&a.count));

    // Get top transitions (limit to 50)
    let top_transitions: Vec<Transition> = transitions.iter().take(50).cloned().collect();

    // Build network graph
    let network_data = build_network_graph(&transitions, &artist_counts);

    // Compute summary
    let summary = compute_summary(
        &transitions,
        &artist_counts,
        session_count,
        total_transitions,
    );

    Ok(TransitionsReport {
        transitions,
        top_transitions,
        network_data,
        summary,
    })
}

fn build_network_graph(
    transitions: &[Transition],
    artist_counts: &HashMap<String, i64>,
) -> NetworkGraph {
    // Build nodes from unique artists in transitions
    let mut nodes_map: HashMap<String, i64> = HashMap::new();

    for transition in transitions {
        *nodes_map.entry(transition.from_artist.clone()).or_insert(0) += artist_counts
            .get(&transition.from_artist)
            .copied()
            .unwrap_or(0);
        *nodes_map.entry(transition.to_artist.clone()).or_insert(0) += artist_counts
            .get(&transition.to_artist)
            .copied()
            .unwrap_or(0);
    }

    let nodes: Vec<Node> = nodes_map
        .into_iter()
        .map(|(artist, size)| Node {
            id: artist.clone(),
            label: artist,
            size,
        })
        .collect();

    // Build edges from transitions
    let edges: Vec<Edge> = transitions
        .iter()
        .map(|t| Edge {
            source: t.from_artist.clone(),
            target: t.to_artist.clone(),
            weight: t.count,
        })
        .collect();

    NetworkGraph { nodes, edges }
}

fn compute_summary(
    transitions: &[Transition],
    artist_counts: &HashMap<String, i64>,
    session_count: usize,
    total_transitions: i64,
) -> TransitionsSummary {
    let most_common_transition = transitions.first().cloned();

    let most_connected_artist = artist_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(artist, _)| artist.clone())
        .unwrap_or_default();

    let avg_transitions_per_session = if session_count > 0 {
        total_transitions as f64 / session_count as f64
    } else {
        0.0
    };

    TransitionsSummary {
        total_transitions,
        unique_transitions: transitions.len(),
        most_common_transition,
        most_connected_artist,
        avg_transitions_per_session,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_extraction() {
        let mut transition_counts: HashMap<(String, String), i64> = HashMap::new();

        let artists = ["Artist A", "Artist B", "Artist C", "Artist A"];

        for i in 0..artists.len() - 1 {
            let from = artists[i].to_string();
            let to = artists[i + 1].to_string();
            *transition_counts.entry((from, to)).or_insert(0) += 1;
        }

        assert_eq!(transition_counts.len(), 3);
        assert_eq!(
            transition_counts.get(&("Artist A".to_string(), "Artist B".to_string())),
            Some(&1)
        );
    }

    #[test]
    fn test_self_transitions_excluded() {
        let mut transition_counts: HashMap<(String, String), i64> = HashMap::new();

        let artists = ["Artist A", "Artist A", "Artist B"];

        for i in 0..artists.len() - 1 {
            let from = artists[i];
            let to = artists[i + 1];

            if from != to {
                *transition_counts
                    .entry((from.to_string(), to.to_string()))
                    .or_insert(0) += 1;
            }
        }

        // Should only have A->B, not A->A
        assert_eq!(transition_counts.len(), 1);
        assert_eq!(
            transition_counts.get(&("Artist A".to_string(), "Artist B".to_string())),
            Some(&1)
        );
    }

    #[test]
    fn test_network_graph_building() {
        let transitions = vec![
            Transition {
                from_artist: "Artist A".to_string(),
                to_artist: "Artist B".to_string(),
                count: 10,
                percentage: 50.0,
            },
            Transition {
                from_artist: "Artist B".to_string(),
                to_artist: "Artist C".to_string(),
                count: 5,
                percentage: 25.0,
            },
        ];

        let mut artist_counts = HashMap::new();
        artist_counts.insert("Artist A".to_string(), 15);
        artist_counts.insert("Artist B".to_string(), 20);
        artist_counts.insert("Artist C".to_string(), 10);

        let network = build_network_graph(&transitions, &artist_counts);

        assert_eq!(network.nodes.len(), 3);
        assert_eq!(network.edges.len(), 2);

        let edge_ab = network
            .edges
            .iter()
            .find(|e| e.source == "Artist A" && e.target == "Artist B")
            .unwrap();
        assert_eq!(edge_ab.weight, 10);
    }
}
