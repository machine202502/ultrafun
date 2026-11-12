use game::{Actions, NavNode};
use glam::Vec3;

mod common;
use common::*;

#[test]
fn grunt_follows_the_graph_around_an_obstructing_wall() {
    let wall = solid(Vec3::new(0.0, 2.0, -4.0), Vec3::new(4.0, 4.0, 1.0));
    let mut game = game_with_enemies(vec![wall], vec![Vec3::ZERO], Vec3::new(0.0, 0.0, -10.0));
    let mut level = game.level().clone();
    level.navigation = vec![
        NavNode {
            position: Vec3::ZERO,
            links: vec![1],
        },
        NavNode {
            position: Vec3::new(-4.0, 0.0, 0.0),
            links: vec![0, 2],
        },
        NavNode {
            position: Vec3::new(-4.0, 0.0, -8.0),
            links: vec![1, 3],
        },
        NavNode {
            position: Vec3::new(0.0, 0.0, -10.0),
            links: vec![2],
        },
    ];
    game.set_level(level);
    let mut farthest_left = 0.0_f32;
    for _ in 0..600 {
        game.update(&Actions::default());
        farthest_left = farthest_left.min(game.enemies[0].position.x);
    }
    assert!(farthest_left < -2.5);
    assert!(
        game.enemies[0].position.distance(game.player.position) < 3.0,
        "at {}",
        game.enemies[0].position
    );
}

#[test]
fn grunt_climbs_a_connected_staircase_to_the_player_floor() {
    let mut boxes = Vec::new();
    for step in 0..20 {
        let top = (step + 1) as f32 * 0.25;
        boxes.push(solid(
            Vec3::new(0.0, top * 0.5, -(step as f32) * 0.8 - 0.4),
            Vec3::new(6.0, top, 0.8),
        ));
    }
    boxes.push(solid(Vec3::new(0.0, 2.5, -20.0), Vec3::new(12.0, 5.0, 8.0)));
    let mut game = game_with_enemies(
        boxes,
        vec![Vec3::new(0.0, 0.0, 2.0)],
        Vec3::new(0.0, 5.0, -21.0),
    );
    let mut level = game.level().clone();
    let points = [
        Vec3::new(0.0, 0.0, 2.0),
        Vec3::new(0.0, 1.25, -3.6),
        Vec3::new(0.0, 2.5, -7.6),
        Vec3::new(0.0, 3.75, -11.6),
        Vec3::new(0.0, 5.0, -16.0),
        Vec3::new(0.0, 5.0, -21.0),
    ];
    level.navigation = points
        .iter()
        .enumerate()
        .map(|(index, position)| NavNode {
            position: *position,
            links: [
                index.checked_sub(1),
                (index + 1 < points.len()).then_some(index + 1),
            ]
            .into_iter()
            .flatten()
            .collect(),
        })
        .collect();
    game.set_level(level);
    run(&mut game, Actions::default(), 8.0);
    assert!(
        game.enemies[0].position.y > 4.8,
        "at {}, waypoint {}, edges {:?}, player {}",
        game.enemies[0].position,
        game.enemies[0].waypoint,
        game.level()
            .navigation
            .windows(2)
            .map(|nodes| game
                .line_of_sight(nodes[0].position + Vec3::Y, nodes[1].position + Vec3::Y))
            .collect::<Vec<_>>(),
        game.player.position
    );
    assert!(game.enemies[0].position.distance(game.player.position) < 3.0);
}
