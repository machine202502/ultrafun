use std::path::Path;

use game::{Actions, ArenaPhase, Game, TargetKind, WeaponKind};
use glam::{Vec2, Vec3};

use super::test_support::{game_config, place, run};
use super::{BlenderLevelFile, load_blender_level};

fn laboratory() -> Game {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let text = std::fs::read_to_string(directory.join("laboratory.ron")).unwrap();
    let file: BlenderLevelFile = ron::from_str(&text).unwrap();
    Game::new(
        load_blender_level(&directory, &file).unwrap().0,
        game_config(),
    )
}

fn aim_and_step(game: &mut Game, target: Vec3) {
    let eye = game.eye_position();
    let enemy = game
        .enemies
        .iter()
        .filter(|enemy| game.line_of_sight(eye, enemy.position + Vec3::Y))
        .min_by(|a, b| {
            a.position
                .distance_squared(eye)
                .total_cmp(&b.position.distance_squared(eye))
        })
        .map(|enemy| enemy.position + Vec3::Y);
    let machine = game
        .targets
        .iter()
        .find(|target| target.kind == TargetKind::Machine)
        .map(|target| target.center);
    let shot = enemy.or(machine);
    let movement = target - game.player.position;
    let facing = shot.map_or(movement, |point| point - eye);
    game.player.yaw = facing.x.atan2(-facing.z);
    game.player.pitch = (facing.y / facing.length().max(0.001)).asin();
    let forward = Vec3::new(game.player.yaw.sin(), 0.0, -game.player.yaw.cos());
    let right = forward.cross(Vec3::Y);
    let wanted = Vec3::new(movement.x, 0.0, movement.z).normalize_or_zero();
    game.update(&Actions {
        move_input: Vec2::new(wanted.dot(right), wanted.dot(forward)),
        fire_held: shot.is_some(),
        ..Actions::default()
    });
}

fn walk_to(game: &mut Game, target: Vec3) {
    for _ in 0..3600 {
        let delta = game.player.position - target;
        if Vec2::new(delta.x, delta.z).length() < 0.65 && delta.y.abs() < 0.5 {
            return;
        }
        aim_and_step(game, target);
    }
    panic!(
        "route blocked: wanted {target}, reached {}, arenas {:?}, enemies {:?}",
        game.player.position,
        game.arenas
            .iter()
            .map(|arena| arena.phase)
            .collect::<Vec<_>>(),
        game.enemies
            .iter()
            .map(|enemy| (enemy.id, enemy.position))
            .collect::<Vec<_>>()
    );
}

#[test]
fn laboratory_export_has_correct_world_axes_and_nav_links() {
    let game = laboratory();
    assert_eq!(game.arenas.len(), 3);
    assert!(game.level().navigation.len() > 50);
    assert!(
        game.keys
            .iter()
            .any(|key| key.zone.center.distance(Vec3::new(-84.0, 21.2, -78.0)) < 0.1)
    );
    assert!(
        game.level()
            .exit
            .as_ref()
            .unwrap()
            .center
            .distance(Vec3::new(-66.0, -2.0, -5.0))
            < 0.1
    );
    assert!(game.level().navigation.iter().all(|node| {
        node.links
            .iter()
            .all(|link| *link < game.level().navigation.len())
    }));
}

#[test]
fn laboratory_main_route_walks_stairs_clears_gates_and_survives_the_drop() {
    let mut game = laboratory();
    game.config.grunt.damage = 0.0;
    let route = [
        (-48.0, 0.0, 40.0),
        (-32.0, 0.0, 40.0),
        (-8.0, 0.0, 40.0),
        (-8.0, 0.0, 8.0),
        (30.0, 0.0, 8.0),
        (66.0, 2.0, 8.0),
        (96.0, 2.0, 8.0),
        (96.0, 4.0, -20.0),
        (96.0, 12.0, -78.0),
        (60.0, 12.0, -78.0),
        (24.0, 12.0, -78.0),
        (-12.0, 16.0, -78.0),
        (-48.0, 16.0, -78.0),
        (-34.0, 16.0, -85.5),
        (-52.0, 20.0, -85.5),
        (-34.0, 16.0, -85.5),
        (-48.0, 16.0, -78.0),
        (-84.0, 20.0, -78.0),
        (-84.0, 30.0, -132.0),
        (-40.0, 30.0, -132.0),
        (8.0, 32.0, -132.0),
        (128.0, 36.0, -132.0),
        (128.0, 36.0, -99.0),
    ];
    for (x, y, z) in route {
        walk_to(&mut game, Vec3::new(x, y, z));
    }
    for _ in 0..120 {
        aim_and_step(&mut game, Vec3::new(128.0, 36.0, -91.0));
    }
    run(&mut game, Actions::default(), 3.0);
    assert!(game.breakables[0].broken);
    assert!(
        (game.player.position.y + 4.0).abs() < 0.2,
        "landing {}",
        game.player.position
    );
    for point in [
        Vec3::new(128.0, -4.0, -46.0),
        Vec3::new(-2.0, -4.0, -46.0),
        Vec3::new(-66.0, 0.0, -46.0),
    ] {
        walk_to(&mut game, point);
    }
    for _ in 0..6000 {
        if game
            .arenas
            .iter()
            .all(|arena| arena.phase == ArenaPhase::Cleared)
        {
            break;
        }
        aim_and_step(&mut game, Vec3::new(-66.0, 0.0, -46.0));
    }
    walk_to(&mut game, Vec3::new(-66.0, -4.0, -8.0));
    for _ in 0..120 {
        if game.result.is_some() {
            break;
        }
        aim_and_step(&mut game, Vec3::new(-66.0, -4.0, -5.0));
    }
    assert!(
        game.result.is_some(),
        "exit {:?}, at {}, phases {:?}",
        game.level().exit,
        game.player.position,
        game.arenas
            .iter()
            .map(|arena| arena.phase)
            .collect::<Vec<_>>()
    );
}

#[test]
fn laboratory_checkpoint_respawn_preserves_selected_tool_and_clears_attacks() {
    let mut game = laboratory();
    place(&mut game, Vec3::new(94.0, 2.0, 8.0), 0.0);
    game.update(&Actions {
        select_weapon: Some(WeaponKind::PlasmaCutter),
        alt_held: true,
        ..Actions::default()
    });
    assert!(game.checkpoint.is_some());
    game.player.health = 0.0;
    run(&mut game, Actions::default(), 2.0);
    assert_eq!(game.weapon.selected, WeaponKind::PlasmaCutter);
    assert_eq!(game.weapon.charge, 0.0);
    assert!(game.projectiles.is_empty());
    assert!(game.player.position.distance(Vec3::new(94.0, 2.0, 8.0)) < 0.2);
}
