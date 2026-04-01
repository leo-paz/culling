//! DBSCAN face clustering on L2-normalized embeddings.
//!
//! Since embeddings are L2-normalized, Euclidean distance is monotonically
//! related to cosine distance: ||a-b||^2 = 2(1 - cos(a,b)).
//!
//! We implement DBSCAN from scratch rather than using linfa-clustering because
//! linfa depends on ndarray 0.15 while this project uses ndarray 0.17, making
//! the types incompatible without an adapter layer.

use crate::error::CullingError;
use std::collections::VecDeque;

/// Cluster face embeddings using DBSCAN with Euclidean distance.
///
/// Returns a vector of cluster labels. `None` means noise (unclustered),
/// `Some(id)` means the point belongs to cluster `id`.
///
/// NOTE: This implementation has O(n^2) time and memory complexity for the
/// pairwise distance computation. It performs well for projects with up to
/// ~5,000 detected faces. For larger datasets, consider a spatial index
/// (e.g., VP-tree) or approximate nearest neighbor approach.
pub fn cluster_embeddings(
    embeddings: &[&[f32]],
    eps: f64,
    min_samples: usize,
) -> Result<Vec<Option<usize>>, CullingError> {
    if embeddings.is_empty() {
        return Ok(Vec::new());
    }

    let n = embeddings.len();

    // Pre-compute pairwise squared Euclidean distances and neighbor lists.
    // For each point, store indices of all points within eps distance.
    let eps_sq = eps * eps;
    let mut neighbors: Vec<Vec<usize>> = Vec::with_capacity(n);

    for i in 0..n {
        let mut nbrs = Vec::new();
        for j in 0..n {
            if i == j {
                // A point is always its own neighbor in DBSCAN
                nbrs.push(j);
                continue;
            }
            let dist_sq = squared_euclidean_f32(embeddings[i], embeddings[j]);
            if (dist_sq as f64) <= eps_sq {
                nbrs.push(j);
            }
        }
        neighbors.push(nbrs);
    }

    // DBSCAN core loop
    let mut labels: Vec<Option<usize>> = vec![None; n];
    let mut visited = vec![false; n];
    let mut current_cluster: usize = 0;

    for i in 0..n {
        if visited[i] {
            continue;
        }
        visited[i] = true;

        if neighbors[i].len() < min_samples {
            // Not a core point; remains None (noise) for now.
            // It may later be claimed by an expanding cluster as a border point.
            continue;
        }

        // Start a new cluster from this core point
        labels[i] = Some(current_cluster);

        let mut queue = VecDeque::new();
        for &nb in &neighbors[i] {
            if nb != i {
                queue.push_back(nb);
            }
        }

        while let Some(q) = queue.pop_front() {
            if labels[q].is_some() {
                // Already assigned to a cluster (could be this one or another)
                continue;
            }

            // Assign to current cluster (border or core)
            labels[q] = Some(current_cluster);

            if visited[q] {
                continue;
            }
            visited[q] = true;

            // If q is also a core point, expand the cluster through its neighbors
            if neighbors[q].len() >= min_samples {
                for &nb in &neighbors[q] {
                    if labels[nb].is_none() {
                        queue.push_back(nb);
                    }
                }
            }
        }

        current_cluster += 1;
    }

    Ok(labels)
}

/// Squared Euclidean distance between two f32 vectors.
#[inline]
fn squared_euclidean_f32(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let result = cluster_embeddings(&[], 0.5, 2).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn single_point_is_noise() {
        let embeddings_owned = vec![vec![1.0, 0.0, 0.0]];
        let embeddings: Vec<&[f32]> = embeddings_owned.iter().map(|v| v.as_slice()).collect();
        let result = cluster_embeddings(&embeddings, 0.5, 2).unwrap();
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn two_close_points_form_cluster() {
        let embeddings_owned = vec![vec![1.0, 0.0], vec![1.0, 0.01]];
        let embeddings: Vec<&[f32]> = embeddings_owned.iter().map(|v| v.as_slice()).collect();
        let result = cluster_embeddings(&embeddings, 0.5, 2).unwrap();
        assert_eq!(result, vec![Some(0), Some(0)]);
    }

    #[test]
    fn distant_points_are_noise() {
        let embeddings_owned = vec![vec![0.0, 0.0], vec![10.0, 10.0]];
        let embeddings: Vec<&[f32]> = embeddings_owned.iter().map(|v| v.as_slice()).collect();
        let result = cluster_embeddings(&embeddings, 0.5, 2).unwrap();
        assert_eq!(result, vec![None, None]);
    }

    #[test]
    fn two_clusters() {
        let embeddings_owned = vec![
            // Cluster A
            vec![0.0, 0.0],
            vec![0.1, 0.0],
            vec![0.0, 0.1],
            // Cluster B
            vec![10.0, 10.0],
            vec![10.1, 10.0],
            vec![10.0, 10.1],
        ];
        let embeddings: Vec<&[f32]> = embeddings_owned.iter().map(|v| v.as_slice()).collect();
        let result = cluster_embeddings(&embeddings, 0.5, 2).unwrap();
        // First three should share a cluster
        assert!(result[0].is_some());
        assert_eq!(result[0], result[1]);
        assert_eq!(result[0], result[2]);
        // Last three should share a different cluster
        assert!(result[3].is_some());
        assert_eq!(result[3], result[4]);
        assert_eq!(result[3], result[5]);
        // The two clusters should be different
        assert_ne!(result[0], result[3]);
    }
}
