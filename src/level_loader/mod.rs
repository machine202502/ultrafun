pub mod scene;

use std::path::Path;

use anyhow::{Context, Result, bail};
use game::{
    ArenaSpawn, BeltSpawn, BreakableSpawn, CrusherSpawn, DEFAULT_HAZARD_DAMAGE_PER_SECOND,
    DoorSpawn, EnemySpawn, HazardSpawn, KeySpawn, Level, LevelBox, MachineSpawn, PadSpawn,
    ScriptAction, SpawnGroup, SwitchSpawn, TargetSpawn, TriggerSpawn, TubeSpawn, Zone,
    coplanar_face_overlaps,
};
use glam::{EulerRot, Quat, Vec3};
use serde::Deserialize;

use crate::render::{Atmosphere, PointLight};

pub use scene::SceneDecor;
use scene::{LINEAR_TO_DISPLAY_GAMMA, SceneCube, SceneObjects, ScenePoint, read_scene};

const PLAYER_START: &str = "player_start";
const ENEMY: &str = "enemy";
const TARGET: &str = "target";
const EXIT: &str = "exit";
const ARENA_PREFIX: &str = "arena_";
const HAZARD_PREFIX: &str = "hazard_";
const DOOR_PREFIX: &str = "door_";
const KEY_PREFIX: &str = "key_";
const BREAKABLE_PREFIX: &str = "breakable_";
const SWITCH_PREFIX: &str = "switch_";
const NON_SOLID_PREFIXES: [&str; 11] = [
    ARENA_PREFIX,
    HAZARD_PREFIX,
    "pad_",
    "crusher_",
    "tube_",
    "door_",
    "key_",
    "breakable_",
    "switch_",
    "trigger_",
    "checkpoint_",
];

#[derive(Debug, Clone, Deserialize)]
pub struct BlenderLevelFile {
    pub model: String,
    pub atmosphere: Atmosphere,
    pub target_health: f32,
    pub light_scale: f32,
    pub default_light_range: f32,
    #[serde(default)]
    pub arenas: Vec<ArenaRules>,
    #[serde(default)]
    pub hazards: Vec<HazardRules>,
    #[serde(default)]
    pub pads: Vec<PadRules>,
    #[serde(default)]
    pub belts: Vec<BeltRules>,
    #[serde(default)]
    pub crushers: Vec<CrusherRules>,
    #[serde(default)]
    pub tubes: Vec<TubeRules>,
    #[serde(default)]
    pub switches: Vec<SwitchRules>,
    #[serde(default)]
    pub breakables: Vec<BreakableRules>,
    #[serde(default)]
    pub triggers: Vec<TriggerRules>,
    #[serde(default)]
    pub checkpoints: Vec<String>,
    #[serde(default)]
    pub next_level: Option<String>,
    #[serde(default)]
    pub validate_geometry: bool,
    #[serde(default)]
    pub spawn_yaw_degrees: Option<f32>,
    #[serde(default)]
    pub navigation: Vec<game::NavNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaRules {
    pub trigger: String,
    pub doors: Vec<String>,
    #[serde(default)]
    pub waves: Vec<String>,
    #[serde(default)]
    pub machine: Option<MachineRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineRules {
    pub object: String,
    pub spawn_point: String,
    pub health: f32,
    pub spawn_interval_seconds: f32,
    pub max_alive: usize,
    #[serde(default)]
    pub say: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HazardRules {
    pub object: String,
    pub damage_per_second: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PadRules {
    pub object: String,
    pub launch_speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeltRules {
    pub object: String,
    pub velocity: Vec3,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrusherRules {
    pub object: String,
    pub travel: f32,
    pub period_seconds: f32,
    pub offset_seconds: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TubeRules {
    pub object: String,
    pub exit_speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwitchRules {
    pub object: String,
    pub health: f32,
    pub actions: Vec<ScriptAction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BreakableRules {
    pub group: String,
    #[serde(default)]
    pub by_slam: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerRules {
    pub zone: String,
    #[serde(default)]
    pub requires_key: Option<String>,
    pub actions: Vec<ScriptAction>,
}

pub fn load_blender_level(
    directory: &Path,
    file: &BlenderLevelFile,
) -> Result<(Level, Vec<PointLight>, Vec<SceneDecor>)> {
    let path = directory.join(&file.model);
    let bytes = std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let objects = read_scene(&bytes).with_context(|| format!("cannot load {}", path.display()))?;
    let lights = level_lights(&objects, file);
    let level = build_level(&objects, file)?;
    Ok((level, lights, objects.decor))
}

pub fn level_lights(objects: &SceneObjects, file: &BlenderLevelFile) -> Vec<PointLight> {
    objects
        .lights
        .iter()
        .map(|light| PointLight {
            position: light.position,
            color: light
                .color
                .map(|channel| channel.powf(LINEAR_TO_DISPLAY_GAMMA)),
            intensity: light.candela * file.light_scale,
            range: light.range.unwrap_or(file.default_light_range),
        })
        .collect()
}

pub fn facing_yaw_degrees(rotation: Quat) -> f32 {
    let arrow = rotation * Vec3::Y;
    let flat = Vec3::new(arrow.x, 0.0, arrow.z);
    if flat.length() < 1e-3 {
        return 0.0;
    }
    flat.x.atan2(-flat.z).to_degrees()
}

pub fn build_level(objects: &SceneObjects, file: &BlenderLevelFile) -> Result<Level> {
    for (index, node) in file.navigation.iter().enumerate() {
        if !node.position.is_finite()
            || node
                .links
                .iter()
                .any(|link| *link >= file.navigation.len() || *link == index)
        {
            bail!("invalid navigation node {index}");
        }
    }
    let start = single_point(objects, PLAYER_START)?;
    let environment = environment_boxes(objects);
    let boxes = environment
        .iter()
        .map(|(_, cube)| level_box(cube, cube.material.as_deref()))
        .collect();
    let targets = cubes(objects, TARGET)
        .iter()
        .map(|cube| TargetSpawn {
            center: cube.center,
            size: cube.size,
            health: file.target_health,
        })
        .collect();
    let enemies = points(objects, ENEMY)
        .iter()
        .map(|point| EnemySpawn {
            position: point.position,
        })
        .collect();
    let arenas = file
        .arenas
        .iter()
        .map(|rules| build_arena(objects, rules))
        .collect::<Result<_>>()?;
    let hazards = build_hazards(objects, &file.hazards)?;
    let doors = build_doors(objects)?;
    let keys = build_keys(objects)?;
    let breakables = build_breakables(objects, &file.breakables)?;
    let switches = build_switches(objects, &file.switches)?;
    let triggers = build_triggers(objects, &file.triggers)?;
    let spawns = build_spawns(objects, &triggers, &switches)?;
    ensure_known_script_names(
        &doors,
        &breakables,
        &hazards,
        &switches,
        &triggers,
        &spawns,
        &file.breakables,
    )?;
    if file.validate_geometry {
        ensure_no_z_fighting(&environment, &doors, &breakables, &switches, &hazards)?;
    }
    Ok(Level {
        spawn_position: start.position,
        spawn_yaw_degrees: file
            .spawn_yaw_degrees
            .unwrap_or_else(|| facing_yaw_degrees(start.rotation)),
        boxes,
        navigation: file.navigation.clone(),
        targets,
        enemies,
        arenas,
        hazards,
        pads: build_pads(objects, &file.pads)?,
        belts: build_belts(objects, &file.belts)?,
        crushers: build_crushers(objects, &file.crushers)?,
        tubes: build_tubes(objects, &file.tubes)?,
        doors,
        keys,
        breakables,
        switches,
        triggers,
        spawns,
        checkpoints: build_checkpoints(objects, &file.checkpoints)?,
        next_level: file.next_level.clone(),
        exit: cubes(objects, EXIT).first().map(zone),
    })
}

fn prefixed<'a>(objects: &'a SceneObjects, prefix: &str) -> Vec<(&'a str, &'a [SceneCube])> {
    let mut groups: Vec<_> = objects
        .cubes
        .iter()
        .filter(|(name, _)| name.starts_with(prefix))
        .map(|(name, cubes)| (name.as_str(), cubes.as_slice()))
        .collect();
    groups.sort_by_key(|(name, _)| *name);
    groups
}

fn stripped<'a>(name: &'a str, prefix: &str) -> &'a str {
    name.strip_prefix(prefix).unwrap_or(name)
}

fn group_zones(name: &str, cubes: &[SceneCube]) -> Result<Vec<Zone>> {
    let mut zones = Vec::new();
    for cube in cubes {
        ensure_upright(cube, name)?;
        zones.push(zone(cube));
    }
    Ok(zones)
}

fn build_doors(objects: &SceneObjects) -> Result<Vec<DoorSpawn>> {
    let mut doors = Vec::new();
    for (name, cubes) in prefixed(objects, DOOR_PREFIX) {
        doors.push(DoorSpawn {
            name: stripped(name, DOOR_PREFIX).to_owned(),
            zones: group_zones(name, cubes)?,
            color: cubes[0].color,
        });
    }
    Ok(doors)
}

fn build_keys(objects: &SceneObjects) -> Result<Vec<KeySpawn>> {
    let mut keys = Vec::new();
    for (name, cubes) in prefixed(objects, KEY_PREFIX) {
        let cube = &cubes[0];
        ensure_upright(cube, name)?;
        keys.push(KeySpawn {
            name: stripped(name, KEY_PREFIX).to_owned(),
            zone: zone(cube),
            color: cube.color,
        });
    }
    Ok(keys)
}

fn build_breakables(
    objects: &SceneObjects,
    rules: &[BreakableRules],
) -> Result<Vec<BreakableSpawn>> {
    let mut breakables = Vec::new();
    for (name, cubes) in prefixed(objects, BREAKABLE_PREFIX) {
        let group = stripped(name, BREAKABLE_PREFIX);
        let by_slam = rules.iter().any(|rule| rule.group == group && rule.by_slam);
        breakables.push(BreakableSpawn {
            name: group.to_owned(),
            zones: group_zones(name, cubes)?,
            color: cubes[0].color,
            by_slam,
        });
    }
    Ok(breakables)
}

fn build_switches(objects: &SceneObjects, rules: &[SwitchRules]) -> Result<Vec<SwitchSpawn>> {
    let mut switches = Vec::new();
    for rule in rules {
        let cube = single_cube(objects, &rule.object)?;
        ensure_upright(cube, &rule.object)?;
        switches.push(SwitchSpawn {
            name: stripped(&rule.object, SWITCH_PREFIX).to_owned(),
            zone: zone(cube),
            color: cube.color,
            health: rule.health,
            actions: rule.actions.clone(),
        });
    }
    Ok(switches)
}

fn build_triggers(objects: &SceneObjects, rules: &[TriggerRules]) -> Result<Vec<TriggerSpawn>> {
    let mut triggers = Vec::new();
    for rule in rules {
        let cube = single_cube(objects, &rule.zone)?;
        ensure_upright(cube, &rule.zone)?;
        triggers.push(TriggerSpawn {
            zone: zone(cube),
            requires_key: rule.requires_key.clone(),
            actions: rule.actions.clone(),
        });
    }
    Ok(triggers)
}

fn build_checkpoints(objects: &SceneObjects, names: &[String]) -> Result<Vec<Zone>> {
    let mut checkpoints = Vec::new();
    for name in names {
        let cube = single_cube(objects, name)?;
        ensure_upright(cube, name)?;
        checkpoints.push(zone(cube));
    }
    Ok(checkpoints)
}

fn build_spawns(
    objects: &SceneObjects,
    triggers: &[TriggerSpawn],
    switches: &[SwitchSpawn],
) -> Result<Vec<SpawnGroup>> {
    let mut names: Vec<&str> = triggers
        .iter()
        .flat_map(|trigger| trigger.actions.iter())
        .chain(switches.iter().flat_map(|switch| switch.actions.iter()))
        .filter_map(|action| match action {
            ScriptAction::Spawn(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let positions = required_points(objects, name)?
                .iter()
                .map(|point| point.position)
                .collect();
            Ok(SpawnGroup {
                name: name.to_owned(),
                positions,
            })
        })
        .collect()
}

fn ensure_known_script_names(
    doors: &[DoorSpawn],
    breakables: &[BreakableSpawn],
    hazards: &[HazardSpawn],
    switches: &[SwitchSpawn],
    triggers: &[TriggerSpawn],
    spawns: &[SpawnGroup],
    breakable_rules: &[BreakableRules],
) -> Result<()> {
    for rule in breakable_rules {
        if !breakables
            .iter()
            .any(|breakable| breakable.name == rule.group)
        {
            bail!("level has no breakable group named \"{}\"", rule.group);
        }
    }
    let actions = switches
        .iter()
        .flat_map(|switch| switch.actions.iter())
        .chain(triggers.iter().flat_map(|trigger| trigger.actions.iter()));
    for action in actions {
        let known = match action {
            ScriptAction::Open(name) | ScriptAction::Close(name) => {
                doors.iter().any(|door| door.name == *name)
            }
            ScriptAction::Break(name) => breakables.iter().any(|breakable| breakable.name == *name),
            ScriptAction::DisableHazard(name) => hazards.iter().any(|hazard| hazard.name == *name),
            ScriptAction::Spawn(name) => spawns.iter().any(|group| group.name == *name),
            ScriptAction::Say(_) => true,
        };
        if !known {
            bail!("level script refers to an object that does not exist: {action:?}");
        }
    }
    Ok(())
}

fn build_hazards(objects: &SceneObjects, rules: &[HazardRules]) -> Result<Vec<HazardSpawn>> {
    let mut hazards = Vec::new();
    for (name, cubes) in &objects.cubes {
        let Some(hazard_name) = name.strip_prefix(HAZARD_PREFIX) else {
            continue;
        };
        let damage_per_second = rules
            .iter()
            .find(|rule| rule.object == *name)
            .map_or(DEFAULT_HAZARD_DAMAGE_PER_SECOND, |rule| {
                rule.damage_per_second
            });
        for cube in cubes {
            ensure_upright(cube, name)?;
            hazards.push(HazardSpawn {
                name: hazard_name.to_owned(),
                zone: zone(cube),
                color: cube.color,
                damage_per_second,
            });
        }
    }
    Ok(hazards)
}

fn build_pads(objects: &SceneObjects, rules: &[PadRules]) -> Result<Vec<PadSpawn>> {
    let mut pads = Vec::new();
    for rule in rules {
        let cube = single_cube(objects, &rule.object)?;
        ensure_upright(cube, &rule.object)?;
        pads.push(PadSpawn {
            zone: zone(cube),
            launch_speed: rule.launch_speed,
        });
    }
    Ok(pads)
}

fn build_belts(objects: &SceneObjects, rules: &[BeltRules]) -> Result<Vec<BeltSpawn>> {
    let mut belts = Vec::new();
    for rule in rules {
        for cube in required_cubes(objects, &rule.object)? {
            belts.push(BeltSpawn {
                zone: zone(cube),
                velocity: rule.velocity,
            });
        }
    }
    Ok(belts)
}

fn build_crushers(objects: &SceneObjects, rules: &[CrusherRules]) -> Result<Vec<CrusherSpawn>> {
    let mut crushers = Vec::new();
    for rule in rules {
        let cube = single_cube(objects, &rule.object)?;
        ensure_upright(cube, &rule.object)?;
        crushers.push(CrusherSpawn {
            center: cube.center,
            size: cube.size,
            color: cube.color,
            travel: rule.travel,
            period_seconds: rule.period_seconds,
            offset_seconds: rule.offset_seconds,
        });
    }
    Ok(crushers)
}

fn build_tubes(objects: &SceneObjects, rules: &[TubeRules]) -> Result<Vec<TubeSpawn>> {
    let mut tubes = Vec::new();
    for rule in rules {
        let exit = single_point(objects, &format!("{}_exit", rule.object))?;
        let exit_yaw_degrees = facing_yaw_degrees(exit.rotation);
        for cube in required_cubes(objects, &rule.object)? {
            ensure_upright(cube, &rule.object)?;
            tubes.push(TubeSpawn {
                zone: zone(cube),
                exit: exit.position,
                exit_yaw_degrees,
                exit_speed: rule.exit_speed,
            });
        }
    }
    Ok(tubes)
}

fn ensure_upright(cube: &SceneCube, name: &str) -> Result<()> {
    let (x, y, z) = cube.rotation.to_euler(EulerRot::XYZ);
    if x.abs() > 1e-3 || y.abs() > 1e-3 || z.abs() > 1e-3 {
        bail!("cube \"{name}\" must not be rotated");
    }
    Ok(())
}

fn build_arena(objects: &SceneObjects, rules: &ArenaRules) -> Result<ArenaSpawn> {
    let doors = rules
        .doors
        .iter()
        .map(|name| required_cubes(objects, name))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map(zone)
        .collect();
    let waves = rules
        .waves
        .iter()
        .map(|name| {
            Ok(required_points(objects, name)?
                .iter()
                .map(|point| point.position)
                .collect())
        })
        .collect::<Result<_>>()?;
    let machine = rules
        .machine
        .as_ref()
        .map(|machine| -> Result<MachineSpawn> {
            let object = single_cube(objects, &machine.object)?;
            Ok(MachineSpawn {
                center: object.center,
                size: object.size,
                health: machine.health,
                spawn_point: single_point(objects, &machine.spawn_point)?.position,
                spawn_interval_seconds: machine.spawn_interval_seconds,
                max_alive: machine.max_alive,
                say: machine.say.clone(),
            })
        })
        .transpose()?;
    Ok(ArenaSpawn {
        trigger: zone(single_cube(objects, &rules.trigger)?),
        doors,
        waves,
        machine,
    })
}

fn level_box(cube: &SceneCube, material: Option<&str>) -> LevelBox {
    let (x, y, z) = cube.rotation.to_euler(EulerRot::XYZ);
    LevelBox {
        center: cube.center,
        size: cube.size,
        rotation_degrees: Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        material: material.map(str::to_owned),
        color: cube.color,
    }
}

fn environment_boxes(objects: &SceneObjects) -> Vec<(&str, &SceneCube)> {
    objects
        .cubes
        .iter()
        .filter(|(name, _)| {
            !NON_SOLID_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
                && name.as_str() != TARGET
                && name.as_str() != EXIT
        })
        .flat_map(|(name, cubes)| cubes.iter().map(move |cube| (name.as_str(), cube)))
        .collect()
}

fn ensure_no_z_fighting(
    environment: &[(&str, &SceneCube)],
    doors: &[DoorSpawn],
    breakables: &[BreakableSpawn],
    switches: &[SwitchSpawn],
    hazards: &[HazardSpawn],
) -> Result<()> {
    let mut named: Vec<(String, LevelBox)> = environment
        .iter()
        .map(|(name, cube)| ((*name).to_owned(), level_box(cube, None)))
        .collect();
    for door in doors {
        for zone in &door.zones {
            named.push((format!("door_{}", door.name), zone_box(zone, door.color)));
        }
    }
    for breakable in breakables {
        for zone in &breakable.zones {
            named.push((
                format!("breakable_{}", breakable.name),
                zone_box(zone, breakable.color),
            ));
        }
    }
    for switch in switches {
        named.push((
            format!("switch_{}", switch.name),
            zone_box(&switch.zone, switch.color),
        ));
    }
    for hazard in hazards {
        named.push((
            format!("hazard_{}", hazard.name),
            zone_box(&hazard.zone, hazard.color),
        ));
    }
    let boxes: Vec<LevelBox> = named
        .iter()
        .map(|(_, level_box)| level_box.clone())
        .collect();
    let overlaps = coplanar_face_overlaps(&boxes);
    if overlaps.is_empty() {
        return Ok(());
    }
    let mut lines: Vec<String> = overlaps
        .iter()
        .map(|overlap| {
            format!(
                "{} and {} share plane {}={:.3} ({} / {})",
                named[overlap.first].0,
                named[overlap.second].0,
                axis_name(overlap.axis),
                overlap.plane,
                describe_box(&boxes[overlap.first]),
                describe_box(&boxes[overlap.second]),
            )
        })
        .collect();
    lines.sort();
    lines.dedup();
    bail!("z-fighting in level geometry:\n  {}", lines.join("\n  "))
}

fn zone_box(zone: &Zone, color: [f32; 3]) -> LevelBox {
    LevelBox {
        center: zone.center,
        size: zone.size,
        rotation_degrees: Vec3::ZERO,
        material: None,
        color,
    }
}

fn axis_name(axis: usize) -> char {
    match axis {
        0 => 'x',
        1 => 'y',
        _ => 'z',
    }
}

fn describe_box(level_box: &LevelBox) -> String {
    let min = level_box.center - level_box.size * 0.5;
    let max = level_box.center + level_box.size * 0.5;
    format!(
        "[{:.1}..{:.1} x {:.1}..{:.1} x {:.1}..{:.1}]",
        min.x, max.x, min.y, max.y, min.z, max.z
    )
}

fn zone(cube: &SceneCube) -> Zone {
    Zone {
        center: cube.center,
        size: cube.size,
    }
}

fn cubes<'a>(objects: &'a SceneObjects, name: &str) -> &'a [SceneCube] {
    objects.cubes.get(name).map_or(&[], Vec::as_slice)
}

fn points<'a>(objects: &'a SceneObjects, name: &str) -> &'a [ScenePoint] {
    objects.points.get(name).map_or(&[], Vec::as_slice)
}

fn required_cubes<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a [SceneCube]> {
    match cubes(objects, name) {
        [] => bail!("level has no cube named \"{name}\""),
        found => Ok(found),
    }
}

fn required_points<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a [ScenePoint]> {
    match points(objects, name) {
        [] => bail!("level has no empty named \"{name}\""),
        found => Ok(found),
    }
}

fn single_cube<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a SceneCube> {
    Ok(&required_cubes(objects, name)?[0])
}

fn single_point<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a ScenePoint> {
    Ok(&required_points(objects, name)?[0])
}

#[cfg(test)]
mod laboratory_tests;
#[cfg(test)]
mod route_tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
