use glam::Vec3;
use serde::Deserialize;

use crate::collision::CollisionWorld;

#[derive(Debug, Clone, Deserialize)]
pub struct NavNode {
    pub position: Vec3,
    pub links: Vec<usize>,
}

pub(crate) fn next_waypoint(
    nodes: &[NavNode],
    world: &CollisionWorld,
    from: Vec3,
    to: Vec3,
) -> Vec3 {
    if nodes.is_empty() {
        return to;
    }
    let anchor = |point: Vec3| {
        nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| world.line_of_sight(point + Vec3::Y, node.position + Vec3::Y))
            .min_by(|(_, a), (_, b)| {
                let cost = |node: &NavNode| {
                    let delta = node.position - point;
                    delta.x * delta.x + delta.z * delta.z + delta.y * delta.y * 9.0
                };
                cost(a).total_cmp(&cost(b))
            })
            .map(|(index, _)| index)
    };
    let (Some(start), Some(goal)) = (anchor(from), anchor(to)) else {
        return from;
    };
    let mut distances = vec![f32::INFINITY; nodes.len()];
    let mut visited = vec![false; nodes.len()];
    let mut parents = vec![None; nodes.len()];
    distances[start] = 0.0;
    for _ in 0..nodes.len() {
        let Some(current) = (0..nodes.len())
            .filter(|index| !visited[*index])
            .min_by(|a, b| distances[*a].total_cmp(&distances[*b]))
        else {
            break;
        };
        if !distances[current].is_finite() {
            break;
        }
        if current == goal {
            break;
        }
        visited[current] = true;
        for &next in &nodes[current].links {
            if next >= nodes.len()
                || visited[next]
                || !world.line_of_sight(
                    nodes[current].position + Vec3::Y,
                    nodes[next].position + Vec3::Y,
                )
            {
                continue;
            }
            let cost = distances[current] + nodes[current].position.distance(nodes[next].position);
            if cost < distances[next] {
                distances[next] = cost;
                parents[next] = Some(current);
            }
        }
    }
    if !distances[goal].is_finite() {
        return from;
    }
    let mut next = goal;
    while let Some(parent) = parents[next] {
        if parent == start {
            let waypoint = nodes[next].position;
            return if world.line_of_sight(from + Vec3::Y, waypoint + Vec3::Y) {
                waypoint
            } else {
                nodes[start].position
            };
        }
        next = parent;
    }
    to
}
