use super::scene::base_name;
use super::test_support::shipped_game;
use super::*;

fn cube(center: Vec3, size: Vec3) -> SceneCube {
    SceneCube {
        center,
        size,
        rotation: Quat::IDENTITY,
        color: [0.5; 3],
        material: None,
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
        hazards: Vec::new(),
        pads: Vec::new(),
        belts: Vec::new(),
        crushers: Vec::new(),
        tubes: Vec::new(),
        switches: Vec::new(),
        breakables: Vec::new(),
        triggers: Vec::new(),
        checkpoints: Vec::new(),
        next_level: None,
        validate_geometry: false,
        spawn_yaw_degrees: None,
        navigation: Vec::new(),
    }
}

#[test]
fn z_fighting_walls_are_rejected_when_validation_is_on() {
    let mut objects = SceneObjects::default();
    objects
        .points
        .insert(PLAYER_START.into(), vec![point(Vec3::ZERO)]);
    objects.cubes.insert(
        "wall_a".into(),
        vec![cube(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    objects.cubes.insert(
        "wall_b".into(),
        vec![cube(Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    let mut file = level_file(Vec::new());
    file.validate_geometry = true;
    let error = build_level(&objects, &file).unwrap_err().to_string();
    assert!(error.contains("z-fighting"), "{error}");
    assert!(error.contains("wall_a"), "{error}");
    assert!(error.contains("wall_b"), "{error}");
    assert!(error.contains("share plane"), "{error}");
}

#[test]
fn z_fighting_walls_load_when_validation_is_off() {
    let mut objects = SceneObjects::default();
    objects
        .points
        .insert(PLAYER_START.into(), vec![point(Vec3::ZERO)]);
    objects.cubes.insert(
        "wall_a".into(),
        vec![cube(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    objects.cubes.insert(
        "wall_b".into(),
        vec![cube(Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    let file = level_file(Vec::new());
    assert!(build_level(&objects, &file).is_ok());
}

#[test]
fn butt_jointed_walls_pass_validation() {
    let mut objects = SceneObjects::default();
    objects
        .points
        .insert(PLAYER_START.into(), vec![point(Vec3::new(-5.0, 0.0, 0.0))]);
    objects.cubes.insert(
        "wall_a".into(),
        vec![cube(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    objects.cubes.insert(
        "wall_b".into(),
        vec![cube(Vec3::new(2.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0))],
    );
    let mut file = level_file(Vec::new());
    file.validate_geometry = true;
    assert!(build_level(&objects, &file).is_ok());
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
    let (level, lights, _) = load_blender_level(&directory, &file).unwrap();
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

#[test]
fn exported_foundry_level_builds() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let text = std::fs::read_to_string(directory.join("foundry.ron")).unwrap();
    let file: BlenderLevelFile = ron::from_str(&text).unwrap();
    let (level, lights, _) = load_blender_level(&directory, &file).unwrap();
    assert!(lights.len() >= 30, "lights {}", lights.len());
    assert!(level.boxes.len() > 150, "boxes {}", level.boxes.len());
    assert_eq!(level.arenas.len(), 2);
    assert_eq!(level.pads.len(), 7);
    assert_eq!(level.belts.len(), 5);
    assert_eq!(level.crushers.len(), 5);
    assert_eq!(level.tubes.len(), 3);
    assert_eq!(level.doors.len(), 3);
    assert_eq!(level.keys.len(), 1);
    assert_eq!(level.keys[0].name, "master");
    assert_eq!(level.breakables.len(), 3);
    assert_eq!(level.switches.len(), 3);
    assert_eq!(level.triggers.len(), 16);
    assert_eq!(level.checkpoints.len(), 12);
    assert_eq!(level.spawns.len(), 3);
    assert_eq!(level.enemies.len(), 18);
    assert!(
        level
            .hazards
            .iter()
            .any(|hazard| hazard.name == "steam" && hazard.damage_per_second < 100.0)
    );
    assert!(level.exit.is_some());
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

fn environment_scene() -> SceneObjects {
    let mut objects = scene();
    objects.cubes.insert(
        "belt_intro".into(),
        vec![cube(Vec3::new(0.0, 0.25, 0.0), Vec3::new(4.0, 0.5, 8.0))],
    );
    objects.cubes.insert(
        "hazard_river".into(),
        vec![
            cube(Vec3::new(0.0, -1.0, 0.0), Vec3::new(4.0, 2.0, 8.0)),
            cube(Vec3::new(0.0, 6.0, 0.0), Vec3::new(2.0, 14.0, 2.0)),
        ],
    );
    objects
        .cubes
        .insert("hazard_steam".into(), vec![cube(Vec3::X, Vec3::ONE)]);
    objects
        .cubes
        .insert("pad_shaft".into(), vec![cube(Vec3::ZERO, Vec3::ONE)]);
    objects
        .cubes
        .insert("crusher_1".into(), vec![cube(Vec3::Y, Vec3::ONE)]);
    objects
        .cubes
        .insert("tube_arena_a".into(), vec![cube(Vec3::Z, Vec3::ONE)]);
    objects.points.insert(
        "tube_arena_a_exit".into(),
        vec![point(Vec3::new(5.0, 1.0, 0.0))],
    );
    objects
}

fn environment_file() -> BlenderLevelFile {
    let mut file = level_file(Vec::new());
    file.hazards = vec![HazardRules {
        object: "hazard_river".into(),
        damage_per_second: 20.0,
    }];
    file.pads = vec![PadRules {
        object: "pad_shaft".into(),
        launch_speed: 31.0,
    }];
    file.belts = vec![BeltRules {
        object: "belt_intro".into(),
        velocity: Vec3::new(0.0, 0.0, -4.0),
    }];
    file.crushers = vec![CrusherRules {
        object: "crusher_1".into(),
        travel: 6.0,
        period_seconds: 2.0,
        offset_seconds: 0.0,
    }];
    file.tubes = vec![TubeRules {
        object: "tube_arena_a".into(),
        exit_speed: 20.0,
    }];
    file
}

#[test]
fn environment_objects_are_not_walls() {
    let level = build_level(&environment_scene(), &environment_file()).unwrap();
    assert_eq!(level.boxes.len(), 2);
    assert_eq!(level.pads.len(), 1);
    assert_eq!(level.pads[0].zone.center, Vec3::ZERO);
    assert_eq!(level.pads[0].launch_speed, 31.0);
    assert_eq!(level.belts.len(), 1);
    assert_eq!(level.belts[0].velocity, Vec3::new(0.0, 0.0, -4.0));
    assert_eq!(level.crushers.len(), 1);
    assert_eq!(level.crushers[0].center, Vec3::Y);
    assert_eq!(level.crushers[0].travel, 6.0);
    assert_eq!(level.tubes.len(), 1);
    assert_eq!(level.tubes[0].exit, Vec3::new(5.0, 1.0, 0.0));
    assert_eq!(level.tubes[0].exit_speed, 20.0);
}

#[test]
fn hazard_group_shares_a_name_and_keeps_its_objects_damage() {
    let level = build_level(&environment_scene(), &environment_file()).unwrap();
    let mut hazards = level.hazards;
    hazards.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(hazards.len(), 3);
    assert_eq!(hazards[0].name, "river");
    assert_eq!(hazards[0].damage_per_second, 20.0);
    assert_eq!(hazards[1].name, "river");
    assert_eq!(hazards[1].damage_per_second, 20.0);
    assert_eq!(hazards[2].name, "steam");
    assert_eq!(
        hazards[2].damage_per_second,
        game::DEFAULT_HAZARD_DAMAGE_PER_SECOND
    );
}

#[test]
fn tube_without_exit_empty_is_an_error() {
    let mut objects = scene();
    objects
        .cubes
        .insert("tube_lonely".into(), vec![cube(Vec3::ZERO, Vec3::ONE)]);
    let mut file = level_file(Vec::new());
    file.tubes = vec![TubeRules {
        object: "tube_lonely".into(),
        exit_speed: 20.0,
    }];
    let error = build_level(&objects, &file).unwrap_err().to_string();
    assert!(error.contains("tube_lonely_exit"), "{error}");
}

#[test]
fn rotated_environment_cube_is_an_error() {
    let mut objects = scene();
    objects.cubes.insert(
        "pad_tilted".into(),
        vec![SceneCube {
            center: Vec3::ZERO,
            size: Vec3::ONE,
            rotation: Quat::from_rotation_z(0.3),
            color: [0.5; 3],
            material: None,
        }],
    );
    let mut file = level_file(Vec::new());
    file.pads = vec![PadRules {
        object: "pad_tilted".into(),
        launch_speed: 31.0,
    }];
    let error = build_level(&objects, &file).unwrap_err().to_string();
    assert!(error.contains("pad_tilted"), "{error}");
}

fn script_scene() -> SceneObjects {
    let mut objects = scene();
    objects.cubes.insert(
        "door_gate".into(),
        vec![cube(Vec3::new(0.0, 2.0, -5.0), Vec3::new(4.0, 4.0, 1.0))],
    );
    objects.cubes.insert(
        "key_master".into(),
        vec![cube(Vec3::new(1.0, 1.25, 0.0), Vec3::splat(0.5))],
    );
    objects.cubes.insert(
        "breakable_catwalk".into(),
        vec![
            cube(Vec3::new(0.0, 8.0, -10.0), Vec3::new(4.0, 0.5, 4.0)),
            cube(Vec3::new(0.0, 8.0, -14.0), Vec3::new(4.0, 0.5, 4.0)),
        ],
    );
    objects.cubes.insert(
        "switch_valve".into(),
        vec![cube(Vec3::new(-2.0, 3.0, -5.0), Vec3::splat(1.0))],
    );
    objects.cubes.insert(
        "trigger_gate".into(),
        vec![cube(Vec3::new(0.0, 1.0, -3.0), Vec3::new(2.0, 3.0, 2.0))],
    );
    objects.cubes.insert(
        "checkpoint_tunnels".into(),
        vec![cube(Vec3::new(0.0, 1.0, 2.0), Vec3::new(2.0, 3.0, 2.0))],
    );
    objects.cubes.insert(
        "hazard_steam".into(),
        vec![cube(Vec3::new(3.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 2.0))],
    );
    objects.points.insert(
        "ambush".into(),
        vec![
            point(Vec3::new(5.0, 0.0, 0.0)),
            point(Vec3::new(6.0, 0.0, 0.0)),
        ],
    );
    objects
}

fn script_file() -> BlenderLevelFile {
    let mut file = level_file(Vec::new());
    file.breakables = vec![BreakableRules {
        group: "catwalk".into(),
        by_slam: true,
    }];
    file.switches = vec![SwitchRules {
        object: "switch_valve".into(),
        health: 1.0,
        actions: vec![
            game::ScriptAction::Open("gate".into()),
            game::ScriptAction::Break("catwalk".into()),
            game::ScriptAction::DisableHazard("steam".into()),
            game::ScriptAction::Spawn("ambush".into()),
            game::ScriptAction::Say("valve_closed".into()),
        ],
    }];
    file.triggers = vec![TriggerRules {
        zone: "trigger_gate".into(),
        requires_key: Some("master".into()),
        actions: vec![
            game::ScriptAction::Open("gate".into()),
            game::ScriptAction::Say("gate_opened".into()),
        ],
    }];
    file.checkpoints = vec!["checkpoint_tunnels".into()];
    file
}

#[test]
fn script_objects_become_groups_and_actions() {
    let level = build_level(&script_scene(), &script_file()).unwrap();
    assert_eq!(level.boxes.len(), 1);
    assert_eq!(level.doors.len(), 1);
    assert_eq!(level.doors[0].name, "gate");
    assert_eq!(level.doors[0].zones.len(), 1);
    assert_eq!(level.keys.len(), 1);
    assert_eq!(level.keys[0].name, "master");
    assert_eq!(level.breakables.len(), 1);
    assert_eq!(level.breakables[0].name, "catwalk");
    assert_eq!(level.breakables[0].zones.len(), 2);
    assert!(level.breakables[0].by_slam);
    assert_eq!(level.switches.len(), 1);
    assert_eq!(level.switches[0].name, "valve");
    assert_eq!(level.switches[0].health, 1.0);
    assert_eq!(level.switches[0].actions.len(), 5);
    assert_eq!(level.triggers.len(), 1);
    assert_eq!(level.triggers[0].requires_key.as_deref(), Some("master"));
    assert_eq!(level.spawns.len(), 1);
    assert_eq!(level.spawns[0].name, "ambush");
    assert_eq!(level.spawns[0].positions.len(), 2);
    assert_eq!(level.checkpoints.len(), 1);
    let steam = level
        .hazards
        .iter()
        .find(|hazard| hazard.name == "steam")
        .unwrap();
    assert_eq!(
        steam.damage_per_second,
        game::DEFAULT_HAZARD_DAMAGE_PER_SECOND
    );
}

#[test]
fn unknown_object_in_script_action_is_an_error() {
    let mut file = script_file();
    file.triggers[0].actions = vec![game::ScriptAction::Open("nowhere".into())];
    let error = build_level(&script_scene(), &file).unwrap_err().to_string();
    assert!(error.contains("nowhere"), "{error}");
}

#[test]
fn breakable_rule_without_a_group_is_an_error() {
    let mut file = script_file();
    file.breakables.push(BreakableRules {
        group: "ghost".into(),
        by_slam: false,
    });
    let error = build_level(&script_scene(), &file).unwrap_err().to_string();
    assert!(error.contains("ghost"), "{error}");
}

#[test]
fn checkpoint_without_a_cube_is_an_error() {
    let mut file = script_file();
    file.checkpoints.push("checkpoint_ghost".into());
    let error = build_level(&script_scene(), &file).unwrap_err().to_string();
    assert!(error.contains("checkpoint_ghost"), "{error}");
}
#[test]
fn every_say_key_has_text() {
    let level = std::fs::read_to_string("assets/levels/foundry.ron").unwrap();
    let text = std::fs::read_to_string("assets/text/en.ron").unwrap();
    let mut missing = Vec::new();
    for chunk in level.split("Say(\"").skip(1) {
        let key = &chunk[..chunk.find('"').unwrap()];
        if !text.contains(&format!("\"{key}\"")) {
            missing.push(key.to_owned());
        }
    }
    assert!(missing.is_empty(), "no text for: {missing:?}");
}

#[test]
fn decor_meshes_load_as_render_only_triangles() {
    let bytes = std::fs::read("assets/models/decor_sample.glb").unwrap();
    let objects = read_scene(&bytes).unwrap();
    assert_eq!(objects.decor.len(), 1);
    let count = objects
        .decor
        .iter()
        .map(|decor| decor.primitive.indices.len() / 3)
        .sum::<usize>();
    assert!(count >= 20, "rock has too few triangles: {count}");
    for decor in &objects.decor {
        for vertex in &decor.primitive.vertices {
            assert!(vertex.position.is_finite());
            assert!(vertex.normal.is_finite());
            assert!(vertex.uv.is_finite());
        }
    }
    assert!(
        objects.cubes.is_empty(),
        "deco meshes must not become walls"
    );
}

#[test]
fn blender_material_name_becomes_the_box_material() {
    let mut objects = scene();
    objects.cubes.insert(
        "brick_wall".into(),
        vec![SceneCube {
            center: Vec3::ZERO,
            size: Vec3::ONE,
            rotation: Quat::IDENTITY,
            color: [0.9; 3],
            material: Some("brick".into()),
        }],
    );
    let level = build_level(&objects, &level_file(Vec::new())).unwrap();
    let brick = level
        .boxes
        .iter()
        .find(|level_box| level_box.material.is_some())
        .expect("brick wall lost its material");
    assert_eq!(brick.material.as_deref(), Some("brick"));
}

#[test]
fn exported_cooling_zones_preserve_y_up_centers_and_dimensions() {
    let bytes = std::fs::read("assets/levels/cooling.glb").unwrap();
    let objects = read_scene(&bytes).unwrap();
    for (name, center, size) in [
        (
            "arena_settling_trigger",
            Vec3::new(79.0, 1.0, -28.0),
            Vec3::splat(2.0),
        ),
        (
            "key_cooling",
            Vec3::new(102.5, 1.2, -47.0),
            Vec3::splat(0.7),
        ),
        (
            "pad_tower_0",
            Vec3::new(80.0, 0.1, -44.0),
            Vec3::new(3.0, 0.4, 3.0),
        ),
        (
            "checkpoint_junction",
            Vec3::new(106.0, 1.0, -28.0),
            Vec3::splat(2.0),
        ),
        ("exit", Vec3::new(-29.0, 14.5, -91.5), Vec3::splat(2.0)),
    ] {
        let cube = single_cube(&objects, name).unwrap();
        assert!(
            cube.center.distance(center) < 0.001,
            "{name}: center {} instead of {center}",
            cube.center
        );
        assert!(
            cube.size.distance(size) < 0.001,
            "{name}: size {} instead of {size}",
            cube.size
        );
    }
    let environment = environment_boxes(&objects);
    assert!(
        environment
            .iter()
            .any(|(name, _)| *name == "settling_column_0")
    );
    assert!(
        !objects
            .cubes
            .keys()
            .any(|name| name.starts_with("arena_settling_column"))
    );
}

#[test]
fn exported_cooling_level_builds() {
    let file: BlenderLevelFile =
        ron::from_str(&std::fs::read_to_string("assets/levels/cooling.ron").unwrap()).unwrap();
    let (level, _, decor) = load_blender_level(Path::new("assets/levels"), &file).unwrap();
    assert!(
        level.boxes.len() > 150,
        "cooling level lost its geometry: {}",
        level.boxes.len()
    );
    assert!(!decor.is_empty(), "cooling level has no decor");
    assert_eq!(level.arenas.len(), 3);
    assert_eq!(level.checkpoints.len(), 4);
    assert_eq!(level.pads.len(), 4);
    assert_eq!(level.crushers.len(), 3);
    assert!(level.keys.iter().any(|key| key.name == "cooling"));
}
