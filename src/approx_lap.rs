/// Approximate Linear Assignment Problem (LAP) solver with FAIRNESS ENFORCEMENT
/// For fair mode: ensures each tile is used approximately equal number of times
///
/// Uses capacity-constrained greedy algorithm: O(n²)
/// Enforces: no tile used more than ceil(blocks/tiles) times
/// This differs from cost-minimization LAP - trades optimality for fairness

pub fn fair_greedy_lap(cost_matrix: &[u32], num_blocks: usize, num_tiles: usize) -> Vec<Option<usize>> {
    // For small problems, return None to fall back to exact Hungarian
    if num_blocks * num_tiles < 1_000_000 {
        return vec![None; num_blocks];
    }

    let mut assignment = vec![None; num_blocks];
    let mut tile_used_count = vec![0usize; num_tiles];

    // Calculate capacity: how many times each tile should be used
    let base_capacity = num_blocks / num_tiles;
    let remainder = num_blocks % num_tiles;

    // Greedy matching with FAIRNESS: enforce equal tile usage
    for block_idx in 0..num_blocks {
        let mut best_tile = 0usize;
        let mut best_cost = u32::MAX;

        // Find tile with lowest cost that hasn't exceeded capacity
        for tile_idx in 0..num_tiles {
            let cost = cost_matrix[block_idx * num_tiles + tile_idx];
            let usage = tile_used_count[tile_idx];

            // Calculate capacity for this tile (some get +1 to handle remainder)
            let capacity = base_capacity + if tile_idx < remainder { 1 } else { 0 };

            // Only consider tiles that haven't hit their capacity
            if usage < capacity && cost < best_cost {
                best_tile = tile_idx;
                best_cost = cost;
            }
        }

        // If all tiles at capacity (shouldn't happen), pick least-used
        if tile_used_count[best_tile] >= (base_capacity + if best_tile < remainder { 1 } else { 0 }) {
            let mut min_usage = usize::MAX;
            for tile_idx in 0..num_tiles {
                if tile_used_count[tile_idx] < min_usage {
                    best_tile = tile_idx;
                    min_usage = tile_used_count[tile_idx];
                }
            }
        }

        assignment[block_idx] = Some(best_tile);
        tile_used_count[best_tile] += 1;
    }

    assignment
}

pub fn fair_auction_lap(cost_matrix: &[u32], num_blocks: usize, num_tiles: usize) -> Vec<Option<usize>> {
    // Auction with capacity constraints: enforces fairness while minimizing cost
    // Each tile has a capacity: ceil(blocks / tiles)

    if num_blocks * num_tiles < 1_000_000 {
        return vec![None; num_blocks];
    }

    let mut assignment = vec![None; num_blocks];
    let mut tile_used_count = vec![0usize; num_tiles];
    let mut tile_prices = vec![0u32; num_tiles];

    let base_capacity = num_blocks / num_tiles;
    let remainder = num_blocks % num_tiles;
    let epsilon = 1u32;

    // Auction phase with capacity constraints
    for block_idx in 0..num_blocks {
        let mut best_tile = 0usize;
        let mut best_value = i32::MIN;
        let mut second_best_value = i32::MIN;

        // Find best and second-best tiles that have available capacity
        for tile_idx in 0..num_tiles {
            let cost = cost_matrix[block_idx * num_tiles + tile_idx] as i32;
            let capacity = base_capacity + if tile_idx < remainder { 1 } else { 0 };
            let usage = tile_used_count[tile_idx];

            // Skip tiles at capacity
            if usage >= capacity {
                continue;
            }

            let value = cost - tile_prices[tile_idx] as i32;

            if value > best_value {
                second_best_value = best_value;
                best_value = value;
                best_tile = tile_idx;
            } else if value > second_best_value {
                second_best_value = value;
            }
        }

        // Assign block to tile
        assignment[block_idx] = Some(best_tile);
        tile_used_count[best_tile] += 1;
        tile_prices[best_tile] = (best_value - second_best_value + epsilon as i32).max(0) as u32;
    }

    assignment
}

/// Choose appropriate algorithm based on problem size
/// All algorithms enforce FAIRNESS (equal tile usage) for fair mode
pub fn solve_lap(
    cost_matrix: &[u32],
    num_blocks: usize,
    num_tiles: usize,
    use_exact: bool,
) -> Vec<Option<usize>> {
    let problem_size = num_blocks * num_tiles;

    // Use exact Hungarian for small problems
    if problem_size < 100_000 || use_exact {
        return vec![None; num_blocks]; // Return None to signal use exact solver
    }

    // For 100K-10M: use auction with fairness (good quality, reasonable speed)
    if problem_size < 10_000_000 {
        return fair_auction_lap(cost_matrix, num_blocks, num_tiles);
    }

    // For >10M: use greedy with fairness (fast, enforces equal distribution)
    fair_greedy_lap(cost_matrix, num_blocks, num_tiles)
}
