use super::scene::base_name;
use super::*;

fn cube(center: Vec3, size: Vec3) -> SceneCube {
    SceneCube {
        center,
        size,
        rotation: Quat::IDENTITY,
        color: [0.5; 3],
    }
}

fn point(position: Vec3) -> ScenePoint {
    ScenePoint {
        position,
        rotation: Quat::IDENTITY,
    }
}

fn level_file(arenas: Vec<ArenaRules>) -> BlenderLevelFile {
    BlenderLevelFile {
        model: String::new(),
        atmosphere: Atmosphere {
            sky_color: [0.0; 3],
            fog_start: 1.0,
            fog_end: 2.0,
            sun_direction: Vec3::NEG_Y,
            sun_strength: 1.0,
            ambient: 0.5,
        },
        target_health: 3.0,
        light_scale: 1.0,
        default_light_range: 10.0,
        arenas,
    }
}

fn scene() -> SceneObjects {
    let mut objects = SceneObjects::default();
    objects
        .points
        .insert(PLAYER_START.into(), vec![point(Vec3::new(1.0, 2.0, 3.0))]);
    objects
        .cubes
        .insert("wall".into(), vec![cube(Vec3::ZERO, Vec3::ONE)]);
    objects
        .cubes
        .insert("arena_door".into(), vec![cube(Vec3::X, Vec3::ONE)]);
    objects.points.insert(
        "arena_wave".into(),
        vec![point(Vec3::X), point(Vec3::Y), point(Vec3::Z)],
    );
    objects
}

fn arena_rules(trigger: &str) -> ArenaRules {
    ArenaRules {
        trigger: trigger.into(),
        doors: vec!["arena_door".into()],
        waves: vec!["arena_wave".into()],
        machine: None,
    }
}

#[test]
fn copies_share_base_name() {
    assert_eq!(base_name("enemy.003"), "enemy");
    assert_eq!(base_name("player_start"), "player_start");
}

#[test]
fn arrow_facing_becomes_yaw() {
    let facing_negative_z = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    assert!(facing_yaw_degrees(facing_negative_z).abs() < 1e-3);
    assert_eq!(facing_yaw_degrees(Quat::IDENTITY), 0.0);
}

#[test]
fn arena_objects_are_not_solid_geometry() {
    let mut objects = scene();
    objects
        .cubes
        .insert("arena_trigger".into(), vec![cube(Vec3::Z, Vec3::ONE)]);
    let level = build_level(&objects, &level_file(vec![arena_rules("arena_trigger")])).unwrap();
    assert_eq!(level.boxes.len(), 1);
    assert_eq!(level.spawn_position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(level.arenas[0].waves[0].len(), 3);
    assert_eq!(level.arenas[0].doors[0].center, Vec3::X);
}

#[test]
fn missing_object_name_is_an_error() {
    let error = build_level(&scene(), &level_file(vec![arena_rules("arena_missing")]))
        .unwrap_err()
        .to_string();
    assert!(error.contains("arena_missing"), "{error}");
}

#[test]
fn exported_utilization_level_builds() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let text = std::fs::read_to_string(directory.join("utilization.ron")).unwrap();
    let file: BlenderLevelFile = ron::from_str(&text).unwrap();
    let (level, lights) = load_blender_level(&directory, &file).unwrap();
    assert!(lights.len() >= 8, "lights {}", lights.len());
    assert!(
        lights
            .iter()
            .all(|light| light.intensity > 0.0 && light.range > 0.0)
    );
    assert!(level.boxes.len() > 30);
    assert_eq!(level.enemies.len(), 2);
    assert_eq!(level.targets.len(), 1);
    assert_eq!(level.arenas[0].waves.iter().map(Vec::len).sum::<usize>(), 7);
    assert!(level.spawn_yaw_degrees.abs() < 1.0);
    let press = level
        .boxes
        .iter()
        .find(|level_box| (level_box.size - Vec3::new(7.0, 10.0, 5.0)).length() < 0.01)
        .expect("press box should keep its size");
    assert!((press.center - Vec3::new(0.0, 5.0, -21.5)).length() < 0.01);
}

fn shipped_game(level: Level) -> game::Game {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
    let load = |name: &str| std::fs::read_to_string(data.join(name)).unwrap();
    let config = game::GameConfig {
        movement: ron::from_str(&load("movement.ron")).unwrap(),
        weapon: ron::from_str(&load("weapons.ron")).unwrap(),
        health: ron::from_str(&load("health.ron")).unwrap(),
        grunt: ron::from_str(&load("grunt.ron")).unwrap(),
    };
    game::Game::new(level, config)
}

fn step_until(
    game: &mut game::Game,
    actions: game::Actions,
    max_seconds: f32,
    done: impl Fn(&game::Game) -> bool,
) {
    for _ in 0..(max_seconds / game::STEP_SECONDS) as usize {
        if done(game) {
            return;
        }
        game.update(&actions);
    }
}

#[test]
fn utilization_route_reaches_arena() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let text = std::fs::read_to_string(directory.join("utilization.ron")).unwrap();
    let file: BlenderLevelFile = ron::from_str(&text).unwrap();
    let mut game = shipped_game(load_blender_level(&directory, &file).unwrap().0);
    let input = |x: f32, y: f32, jump: bool, slide: bool| game::Actions {
        move_input: glam::Vec2::new(x, y),
        jump_pressed: jump,
        slide_held: slide,
        ..game::Actions::default()
    };

    let toward_doorway = glam::Vec2::new(-1.0, 3.0).normalize();
    game.update(&input(toward_doorway.x, toward_doorway.y, true, false));
    step_until(
        &mut game,
        input(toward_doorway.x, toward_doorway.y, false, false),
        3.0,
        |game| game.player.position.z < -25.0,
    );
    step_until(&mut game, input(0.0, 1.0, false, false), 4.0, |game| {
        game.player.position.z < -33.0
    });
    step_until(&mut game, input(0.0, 1.0, false, true), 2.0, |game| {
        game.player.position.z < -42.0
    });
    step_until(&mut game, input(0.0, 1.0, false, false), 4.0, |game| {
        game.arenas[0].phase == game::ArenaPhase::Active
    });

    assert_eq!(
        game.arenas[0].phase,
        game::ArenaPhase::Active,
        "player stopped at {}",
        game.player.position
    );
}

#[test]
fn exit_cube_becomes_exit_zone_not_wall() {
    let mut objects = scene();
    objects.cubes.insert(
        EXIT.into(),
        vec![cube(Vec3::new(0.0, 1.0, -5.0), Vec3::ONE)],
    );
    let level = build_level(&objects, &level_file(Vec::new())).unwrap();
    assert_eq!(level.boxes.len(), 1);
    assert_eq!(
        level.exit.map(|exit| exit.center),
        Some(Vec3::new(0.0, 1.0, -5.0))
    );
}
