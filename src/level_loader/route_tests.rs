use super::test_support::*;
use game::Actions;
use glam::Vec2;

#[test]
fn reception_ledge_drop_rides_the_belt_into_sorting() {
    let mut game = foundry_game();
    run_toward(&mut game, glam::Vec3::new(0.0, 0.0, -30.0), 3.0);
    assert!(game.player.health > 0.0);
    assert!(
        game.player.position.z < -17.0,
        "z {}",
        game.player.position.z
    );
}

#[test]
fn catwalk_collapse_drops_player_onto_the_island() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(0.0, 8.0, -70.0), 0.0);
    run(&mut game, forward(), 1.0);
    run(&mut game, Actions::default(), 1.0);
    assert!(game.player.health > 0.0);
    assert!(
        (game.player.position.y + 2.0).abs() < 0.4,
        "y {}",
        game.player.position.y
    );
    assert!(
        game.player.position.z > -100.0 && game.player.position.z < -70.0,
        "z {}",
        game.player.position.z
    );
}

#[test]
fn slag_pipes_stop_a_standing_player_and_let_a_sliding_one_pass() {
    let mut standing = foundry_game();
    place(&mut standing, glam::Vec3::new(-44.0, -2.0, -80.0), 180.0);
    run(&mut standing, forward(), 2.0);
    assert!(
        standing.player.position.z < -76.0,
        "z {}",
        standing.player.position.z
    );

    let mut sliding = foundry_game();
    place(&mut sliding, glam::Vec3::new(-44.0, -2.0, -74.0), 180.0);
    let slide = Actions {
        move_input: Vec2::Y,
        slide_held: true,
        slide_pressed: true,
        ..Actions::default()
    };
    run(&mut sliding, slide, 2.0);
    assert!(
        sliding.player.position.z > -70.0,
        "z {}",
        sliding.player.position.z
    );
}

#[test]
fn shaft_pad_lifts_player_above_the_duct_floor() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(-39.0, -2.0, -54.0), 0.0);
    let mut highest = f32::MIN;
    for _ in 0..120 {
        game.update(&Actions::default());
        highest = highest.max(game.player.position.y);
    }
    assert!(highest > 8.0, "highest {highest}");
}

#[test]
fn gate_stays_shut_without_the_key_and_opens_with_it() {
    let mut locked = foundry_game();
    place(&mut locked, glam::Vec3::new(18.0, 0.0, -44.0), 90.0);
    run(&mut locked, forward(), 1.5);
    assert!(
        locked.player.position.x < 24.0,
        "x {}",
        locked.player.position.x
    );
    assert!(
        !locked
            .doors
            .iter()
            .any(|door| door.name == "gate" && door.open)
    );

    let mut opened = foundry_game();
    opened.keys[0].collected = true;
    place(&mut opened, glam::Vec3::new(18.0, 0.0, -44.0), 90.0);
    run(&mut opened, forward(), 3.0);
    assert!(
        opened
            .doors
            .iter()
            .any(|door| door.name == "gate" && door.open)
    );
    assert!(
        opened.player.position.x > 25.0,
        "x {}",
        opened.player.position.x
    );
}

#[test]
fn standing_under_a_press_kills_the_player() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(50.0, 0.0, -48.0), 0.0);
    run(&mut game, Actions::default(), 1.0);
    assert_eq!(game.player.health, 0.0);
}

#[test]
fn press_corridor_can_be_crossed_at_the_right_moment() {
    let mut log = Vec::new();
    for step in 0..20 {
        let mut game = foundry_game();
        place(&mut game, glam::Vec3::new(45.5, 0.0, -48.0), 90.0);
        run(&mut game, Actions::default(), step as f32 * 0.1);
        run(&mut game, forward(), 3.0);
        log.push(format!(
            "{}: h={} x={}",
            step, game.player.health, game.player.position.x
        ));
        if game.player.health > 0.0 && game.player.position.x > 85.0 {
            return;
        }
    }
    panic!("no start delay crossed the corridor: {}", log.join(", "));
}
#[test]
fn casting_arena_starts_and_closes_both_doors() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(116.0, 0.0, -48.0), 90.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.arenas[0].phase, game::ArenaPhase::Active);
    assert_eq!(game.closed_doors().count(), 2);
    assert_eq!(game.enemies.len(), 18 + 4);
}

#[test]
fn river_kills_on_foot_until_the_valve_is_shot() {
    let mut burning = foundry_game();
    place(&mut burning, glam::Vec3::new(142.0, -2.0, -100.0), 0.0);
    run(&mut burning, forward(), 1.5);
    assert_eq!(burning.player.health, 0.0);

    let mut safe = foundry_game();
    place(&mut safe, glam::Vec3::new(142.0, -2.0, -100.0), 0.0);
    safe.switches
        .iter_mut()
        .find(|switch| switch.name == "valve")
        .unwrap()
        .health = 0.0;
    run(&mut safe, Actions::default(), 0.2);
    assert!(
        safe.switches
            .iter()
            .any(|switch| switch.name == "valve" && switch.activated)
    );
    assert!(
        safe.hazards
            .iter()
            .all(|hazard| hazard.name != "river" || !hazard.enabled)
    );
    run(&mut safe, forward(), 1.5);
    assert!(safe.player.health > 0.0);
    assert!(
        safe.player.position.z < -108.0,
        "z {}",
        safe.player.position.z
    );
}

#[test]
fn every_crane_platform_is_reachable_from_the_previous_one() {
    let mut game = foundry_game();
    let chain = [
        (
            glam::Vec3::new(146.0, 12.0, -196.6),
            glam::Vec3::new(146.0, 14.5, -204.0),
        ),
        (
            glam::Vec3::new(143.4, 14.5, -204.0),
            glam::Vec3::new(136.0, 17.0, -207.0),
        ),
        (
            glam::Vec3::new(136.0, 17.0, -214.6),
            glam::Vec3::new(137.0, 19.5, -222.0),
        ),
        (
            glam::Vec3::new(139.6, 19.5, -222.0),
            glam::Vec3::new(147.0, 22.0, -222.0),
        ),
        (
            glam::Vec3::new(147.0, 22.0, -224.6),
            glam::Vec3::new(142.0, 24.0, -232.0),
        ),
    ];
    for (from, to) in chain {
        jump_toward(&mut game, from, to);
        assert!(
            (game.player.position.y - to.y).abs() < 0.6,
            "from {from} to {to}: at {}",
            game.player.position
        );
    }
}
#[test]
fn island_tunnels_lead_to_the_foreman_key() {
    let mut game = foundry_game();
    game.player.health = 1000.0;
    place(&mut game, glam::Vec3::new(-6.0, -2.0, -86.0), 270.0);
    run(&mut game, forward(), 3.0);
    assert!(
        game.player.position.x < -40.0,
        "x {}",
        game.player.position.x
    );

    game.player.yaw = 180.0f32.to_radians();
    let slide = Actions {
        move_input: Vec2::Y,
        slide_held: true,
        slide_pressed: true,
        ..Actions::default()
    };
    run(&mut game, slide, 4.0);
    assert!(
        game.player.position.z > -70.0,
        "z {}",
        game.player.position.z
    );

    run(&mut game, forward(), 1.2);
    assert!(
        game.player.position.z > -60.0,
        "z {}",
        game.player.position.z
    );

    run_toward(&mut game, glam::Vec3::new(-54.3, -2.0, -53.0), 2.5);
    assert!(game.has_key("master"), "no key at {}", game.player.position);
}
#[test]
fn standing_still_hurts_nowhere() {
    let spots = [
        ("reception", glam::Vec3::new(0.0, 0.0, -8.0)),
        ("sorting", glam::Vec3::new(0.0, 0.0, -30.0)),
        ("sorting north", glam::Vec3::new(10.0, 0.0, -50.0)),
        ("island", glam::Vec3::new(0.0, -2.0, -80.0)),
        ("tunnel A", glam::Vec3::new(-38.0, -2.0, -86.0)),
        ("tunnel B", glam::Vec3::new(-44.0, -2.0, -70.0)),
        ("shack", glam::Vec3::new(-48.0, -2.0, -54.0)),
        ("compressor", glam::Vec3::new(-30.0, 8.0, -48.0)),
        ("molds", glam::Vec3::new(34.0, 0.0, -46.0)),
        ("presses", glam::Vec3::new(46.0, 0.0, -48.0)),
        ("furnace", glam::Vec3::new(86.0, 0.0, -48.0)),
        ("casting", glam::Vec3::new(116.0, 0.0, -48.0)),
        ("pouring", glam::Vec3::new(142.0, 0.0, -85.0)),
        ("crossing", glam::Vec3::new(140.0, 0.0, -128.0)),
        ("tank", glam::Vec3::new(142.0, 0.0, -160.0)),
        ("crane landing", glam::Vec3::new(146.0, 12.0, -194.0)),
        ("crane floor", glam::Vec3::new(140.0, 0.0, -200.0)),
    ];
    let mut broken = Vec::new();
    for (name, position) in spots {
        let mut game = foundry_game();
        game.enemies.clear();
        place(&mut game, position, 0.0);
        run(&mut game, Actions::default(), 1.5);
        if game.player.health < 100.0 {
            broken.push(format!(
                "{name} at {position}: health {} at {}",
                game.player.health, game.player.position
            ));
        }
    }
    assert!(broken.is_empty(), "damage while idle: {broken:?}");
}
#[test]
fn every_enemy_spawn_is_outside_solid_geometry() {
    let game = foundry_game();
    let level = game.level();
    let mut spots: Vec<(String, glam::Vec3)> = Vec::new();
    for spawn in &level.enemies {
        spots.push(("ambient".into(), spawn.position));
    }
    for group in &level.spawns {
        for position in &group.positions {
            spots.push((format!("spawn {}", group.name), *position));
        }
    }
    for arena in &level.arenas {
        for wave in &arena.waves {
            for position in wave {
                spots.push(("wave".into(), *position));
            }
        }
        if let Some(machine) = &arena.machine {
            spots.push(("machine spawn".into(), machine.spawn_point));
        }
    }
    let mut stuck = Vec::new();
    for (name, position) in &spots {
        let feet = *position;
        let center = glam::Vec3::new(feet.x, feet.y + 1.0, feet.z);
        for level_box in &level.boxes {
            let box_half = level_box.size * 0.5;
            let delta = (center - level_box.center).abs();
            let overlap = box_half - delta;
            if overlap.x > 0.05 && overlap.y > 0.05 && overlap.z > 0.05 {
                stuck.push(format!(
                    "{name} at {feet} inside box at {} size {}",
                    level_box.center, level_box.size
                ));
                break;
            }
        }
    }
    assert!(stuck.is_empty(), "enemies inside geometry: {stuck:#?}");

    let mut floating = Vec::new();
    for (name, position) in &spots {
        let highest = level
            .boxes
            .iter()
            .filter(|box_| {
                let half = box_.size * 0.5;
                (position.x - box_.center.x).abs() <= half.x
                    && (position.z - box_.center.z).abs() <= half.z
            })
            .map(|box_| box_.center.y + box_.size.y * 0.5)
            .filter(|top| *top <= position.y + 0.05)
            .fold(f32::MIN, f32::max);
        if (position.y - highest).abs() > 0.1 {
            floating.push(format!("{name} at {position} floor {highest}"));
        }
    }
    assert!(floating.is_empty(), "spawns without a floor: {floating:#?}");
}
#[test]
fn sorting_stairs_lead_to_the_catwalk() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(15.0, 0.0, -62.0), 270.0);
    for _ in 0..400 {
        if game.player.position.y > 7.5 {
            break;
        }
        game.update(&forward());
    }
    game.player.velocity = glam::Vec3::ZERO;
    assert!(
        game.player.position.y > 7.0,
        "y {} at {}",
        game.player.position.y,
        game.player.position
    );
    game.player.yaw = 0.0;
    run(&mut game, forward(), 1.3);
    assert!(
        game.player.position.z < -64.0 && game.player.position.y > 7.0,
        "stuck at {}",
        game.player.position
    );
}

#[test]
fn foreman_shack_opens_into_the_shaft() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(-48.0, -2.0, -56.0), 90.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.x > -37.0,
        "x {}",
        game.player.position.x
    );
}

#[test]
fn shaft_pad_lifts_player_through_the_duct() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(-39.0, -1.0, -54.0), 90.0);
    run(&mut game, forward(), 1.8);
    assert!(
        game.player.position.x > -35.0 && game.player.position.y > 7.5,
        "at {}",
        game.player.position
    );
}

#[test]
fn compressor_vent_leads_to_the_sorting_balcony() {
    let mut game = foundry_game();
    game.switches
        .iter_mut()
        .find(|switch| switch.name == "vent")
        .unwrap()
        .health = 0.0;
    run(&mut game, Actions::default(), 0.2);
    place(&mut game, glam::Vec3::new(-30.0, 8.0, -48.0), 90.0);
    run(&mut game, forward(), 2.0);
    assert!(
        game.player.position.x > -23.0 && game.player.position.y > 7.4,
        "at {}",
        game.player.position
    );
}

#[test]
fn furnace_pad_reaches_the_ring() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(98.0, 1.0, -56.0), 0.0);
    run(&mut game, forward(), 1.05);
    assert!(
        game.player.position.y > 9.4,
        "y {} at {}",
        game.player.position.y,
        game.player.position
    );
}

#[test]
fn furnace_ring_switch_opens_the_way_to_casting() {
    let mut game = foundry_game();
    game.switches
        .iter_mut()
        .find(|switch| switch.name == "furnace")
        .unwrap()
        .health = 0.0;
    run(&mut game, Actions::default(), 0.2);
    assert!(
        game.doors
            .iter()
            .any(|door| door.name == "furnace" && door.open),
        "door did not open"
    );
    place(&mut game, glam::Vec3::new(109.0, 0.0, -48.0), 90.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.x > 112.5,
        "x {}",
        game.player.position.x
    );
}

#[test]
fn pouring_crossing_and_tank_are_walkable() {
    let mut game = foundry_game();
    game.switches
        .iter_mut()
        .find(|switch| switch.name == "valve")
        .unwrap()
        .health = 0.0;
    run(&mut game, Actions::default(), 0.2);

    place(&mut game, glam::Vec3::new(142.0, 0.0, -80.0), 0.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.z < -80.0,
        "pouring: {}",
        game.player.position
    );

    place(&mut game, glam::Vec3::new(140.0, 0.0, -100.0), 0.0);
    run(&mut game, forward(), 2.0);
    assert!(
        game.player.position.z < -111.0,
        "river: {}",
        game.player.position
    );

    place(&mut game, glam::Vec3::new(140.0, 0.0, -126.0), 0.0);
    run(&mut game, forward(), 2.5);
    assert!(
        game.player.position.z < -145.0,
        "crossing: {}",
        game.player.position
    );

    place(&mut game, glam::Vec3::new(142.0, 0.0, -152.0), 0.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.z < -157.0,
        "tank link: {}",
        game.player.position
    );
}

#[test]
fn tank_stairs_reach_the_top() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(135.0, 0.0, -161.0), 0.0);
    run(&mut game, forward(), 3.0);
    assert!(
        game.player.position.y > 11.4,
        "y {} at {}",
        game.player.position.y,
        game.player.position
    );
}

#[test]
fn tank_exit_leads_to_the_crane_landing() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(140.0, 12.0, -188.0), 0.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.y > 11.4 && game.player.position.z < -190.0,
        "at {}",
        game.player.position
    );
}
#[test]
fn casting_arena_ends_only_after_the_machine_breaks() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(116.0, 0.0, -48.0), 90.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.arenas[0].phase, game::ArenaPhase::Active);

    let mut announced = false;
    for _ in 0..600 {
        game.enemies.clear();
        game.update(&Actions::default());
        announced |= game.events.iter().any(|event| {
            matches!(event, game::GameEvent::Say { key } if key == "dispatcher.casting_machine")
        });
        if game.arenas[0].phase == game::ArenaPhase::Cleared {
            break;
        }
    }
    assert!(announced, "machine was never announced");
    assert_eq!(
        game.arenas[0].phase,
        game::ArenaPhase::Active,
        "arena cleared without breaking the machine"
    );
    assert_eq!(game.closed_doors().count(), 2);

    place(&mut game, glam::Vec3::new(160.0, 0.0, -62.0), 90.0);
    let fire = Actions {
        fire_held: true,
        ..Actions::default()
    };
    for _ in 0..900 {
        game.enemies.clear();
        game.update(&fire);
        if game.arenas[0].phase == game::ArenaPhase::Cleared {
            break;
        }
    }
    assert_eq!(
        game.arenas[0].phase,
        game::ArenaPhase::Cleared,
        "machine did not break the arena"
    );
    assert_eq!(game.closed_doors().count(), 0);

    place(&mut game, glam::Vec3::new(142.0, 0.0, -74.0), 0.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.player.position.z < -78.0,
        "exit door closed: {}",
        game.player.position
    );
}
#[test]
fn machine_killed_before_the_waves_still_ends_the_arena() {
    let mut game = foundry_game();
    game.player.health = 1000.0;
    place(&mut game, glam::Vec3::new(116.0, 0.0, -48.0), 90.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.arenas[0].phase, game::ArenaPhase::Active);
    let machine_id = game.arenas[0].machine_target_id.unwrap();

    place(&mut game, glam::Vec3::new(160.0, 0.0, -62.0), 90.0);
    let fire = Actions {
        fire_held: true,
        ..Actions::default()
    };
    for _ in 0..900 {
        game.update(&fire);
        if game.targets.iter().all(|target| target.id != machine_id) {
            break;
        }
    }
    assert!(
        game.targets.iter().all(|target| target.id != machine_id),
        "machine still alive: {:?}",
        game.targets
            .iter()
            .map(|target| (target.id, target.health))
            .collect::<Vec<_>>()
    );

    for _ in 0..600 {
        game.enemies.clear();
        game.update(&Actions::default());
        if game.arenas[0].phase == game::ArenaPhase::Cleared {
            break;
        }
    }
    assert_eq!(
        game.arenas[0].phase,
        game::ArenaPhase::Cleared,
        "arena stuck: phase {:?}, wave {}, doors {}",
        game.arenas[0].phase,
        game.arenas[0].wave,
        game.closed_doors().count()
    );
    assert_eq!(game.closed_doors().count(), 0);
}
#[test]
fn arena_waves_burn_when_they_fall_into_the_channel() {
    let mut game = foundry_game();
    game.player.health = 1000.0;
    place(&mut game, glam::Vec3::new(116.0, 0.0, -48.0), 90.0);
    run(&mut game, Actions::default(), 0.2);
    let machine_id = game.arenas[0].machine_target_id.unwrap();
    game.targets.retain(|target| target.id != machine_id);

    for _ in 0..1800 {
        for enemy in game.enemies.iter_mut() {
            enemy.position = glam::Vec3::new(140.0, -2.5, -48.0);
            enemy.velocity = glam::Vec3::ZERO;
        }
        game.update(&Actions::default());
        if game.arenas[0].phase == game::ArenaPhase::Cleared {
            break;
        }
    }
    assert_eq!(
        game.arenas[0].phase,
        game::ArenaPhase::Cleared,
        "arena stuck on enemies that fell into the lava: wave {}, alive {}",
        game.arenas[0].wave,
        game.enemies.len()
    );
    assert_eq!(game.closed_doors().count(), 0);
}
#[test]
fn falling_off_the_foundry_map_kills_the_player() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(220.0, 30.0, 0.0), 0.0);
    run(&mut game, Actions::default(), 2.5);
    assert_eq!(game.player.health, 0.0, "player survived the void");
}
#[test]
fn manual_respawn_returns_to_the_last_checkpoint_without_losing_progress() {
    let mut game = foundry_game();
    place(&mut game, glam::Vec3::new(-16.0, -2.0, -86.0), 0.0);
    run(&mut game, Actions::default(), 0.2);
    let checkpoint = game.checkpoint.clone().expect("no checkpoint captured");

    game.keys[0].collected = true;
    place(&mut game, glam::Vec3::new(150.0, 20.0, 0.0), 0.0);
    let respawn = Actions {
        respawn: true,
        ..Actions::default()
    };
    game.update(&respawn);
    assert!(
        (game.player.position - checkpoint.position).length() < 0.6,
        "respawned at {} instead of {:?}",
        game.player.position,
        checkpoint.position
    );
    assert!(game.keys[0].collected, "respawn lost the collected key");
    assert!(game.player.health > 0.0);
}
#[test]
fn crane_exit_room_stays_locked_until_the_arena_is_cleared() {
    let mut game = foundry_game();
    game.player.health = 1000.0;
    place(&mut game, glam::Vec3::new(140.0, 24.0, -231.0), 0.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.arenas[1].phase, game::ArenaPhase::Active);
    run(&mut game, forward(), 1.0);
    assert!(
        game.player.position.z > -233.4,
        "player slipped into the exit room while the arena is active: {}",
        game.player.position
    );
    assert!(game.result.is_none());

    for _ in 0..900 {
        game.enemies.clear();
        game.update(&Actions::default());
        if game.arenas[1].phase == game::ArenaPhase::Cleared {
            break;
        }
    }
    assert_eq!(game.arenas[1].phase, game::ArenaPhase::Cleared);
    game.arenas[0].phase = game::ArenaPhase::Cleared;
    place(&mut game, glam::Vec3::new(140.0, 24.0, -231.0), 0.0);
    run(&mut game, forward(), 1.5);
    assert!(
        game.result.is_some(),
        "exit did not complete the level: {}",
        game.player.position
    );
}
