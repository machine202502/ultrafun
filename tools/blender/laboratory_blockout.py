import bpy
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROOMS = [
    ("arrival", -48, 0, 40, 8, 10, 6),
    ("sanitary", -32, 0, 40, 16, 8, 6),
    ("overlook", -8, 0, 40, 18, 14, 14),
    ("hub", -8, 0, 8, 26, 24, 42),
    ("workshop", 30, 0, 8, 20, 18, 10),
    ("abrasive", 66, 2, 8, 30, 28, 12),
    ("permit", 96, 2, 8, 14, 14, 7),
    ("lower_bridge", 96, 4, -20, 10, 10, 14),
    ("stairs", 96, 12, -78, 20, 20, 16),
    ("spectrum", 60, 12, -78, 22, 12, 10),
    ("load_test", 24, 12, -78, 22, 18, 10),
    ("screens", -12, 16, -78, 14, 12, 7),
    ("calibration", -48, 16, -78, 30, 28, 12),
    ("operator", -84, 20, -78, 14, 12, 8),
    ("upper_stairs", -84, 30, -132, 16, 14, 16),
    ("observation", -40, 30, -132, 34, 14, 14),
    ("archive", -40, 30, -160, 20, 14, 8),
    ("upper_bridge", 8, 32, -132, 34, 10, 12),
    ("crown", 128, 36, -132, 18, 18, 12),
    ("chute", 128, -4, -88, 10, 18, 46),
    ("bowl", 128, -4, -46, 24, 18, 12),
    ("service", -2, -4, -46, 18, 14, 8),
    ("final", -66, 0, -46, 30, 28, 16),
    ("dispatch", -66, -4, -8, 14, 12, 8),
]
LINKS = [(i, i + 1) for i in range(15)] + [(15, 16), (15, 17), (17, 18), (19, 20), (20, 21), (21, 22), (22, 23)]
OPENINGS = {i: set() for i in range(len(ROOMS))}
MATERIALS = {}
NAV = []


def position(point):
    x, y, z = point
    return (x, -z, y)


def material(name, color=(0.9, 0.9, 0.9)):
    key = (name, color)
    if key not in MATERIALS:
        item = bpy.data.materials.new(name)
        item.diffuse_color = (*color, 1)
        item.use_nodes = True
        item.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = (*color, 1)
        if name == "lab_stripe":
            item.node_tree.nodes.get("Principled BSDF").inputs["Emission Color"].default_value = (*color, 1)
            item.node_tree.nodes.get("Principled BSDF").inputs["Emission Strength"].default_value = 1.0
        MATERIALS[key] = item
    return MATERIALS[key]


def cube(name, center, size, mat="lab_panel", color=(0.9, 0.9, 0.9)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=position(center))
    obj = bpy.context.object
    obj.name = name
    obj.scale = (size[0], size[2], size[1])
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material(mat, color))
    return obj


def marker(name, point):
    obj = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(obj)
    obj.location = position(point)
    return obj


def lamp(name, point, color, power, radius=20):
    data = bpy.data.lights.new(name, "POINT")
    data.energy = power
    data.color = color
    data.use_custom_distance = True
    data.cutoff_distance = radius
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    obj.location = position(point)
    return obj


def nav(point):
    NAV.append({"position": point, "links": []})
    return len(NAV) - 1


def edge(a, b):
    NAV[a]["links"].append(b)
    NAV[b]["links"].append(a)


def room_openings():
    for a, b in LINKS:
        ax, ay, az = ROOMS[a][1:4]
        bx, by, bz = ROOMS[b][1:4]
        if abs(bx - ax) > abs(bz - az):
            OPENINGS[a].add("e" if bx > ax else "w")
            OPENINGS[b].add("w" if bx > ax else "e")
        else:
            OPENINGS[a].add("s" if bz > az else "n")
            OPENINGS[b].add("n" if bz > az else "s")
    OPENINGS[18].add("s")
    OPENINGS[19].add("n")


VIEW_SIDES = {2: "n", 3: "n", 4: "n", 8: "w", 9: "s", 12: "s", 15: "s", 17: "s"}


def tube_mesh(name, start, end, radius, mat="metal_brushed", color=(0.8, 0.85, 0.88)):
    from mathutils import Vector
    a, b = Vector(position(start)), Vector(position(end))
    direction = b - a
    bpy.ops.mesh.primitive_cylinder_add(vertices=10, radius=radius, depth=direction.length, location=(a + b) / 2)
    obj = bpy.context.object
    obj.name = "deco_" + name
    obj.rotation_euler = direction.to_track_quat("Z", "Y").to_euler()
    obj.data.materials.append(material(mat, color))
    return obj


def apparatus(name, x, y, z):
    cube("deco_" + name + "_base", (x, y + 0.25, z), (2.2, 0.5, 2.2), "concrete_dark")
    tube_mesh(name + "_vessel", (x, y + 0.5, z), (x, y + 3.5, z), 0.8, "metal_panel")
    for height in (0.7, 3.3):
        tube_mesh(name + "_collar", (x, y + height, z), (x, y + height + 0.15, z), 0.9, "warning")
    tube_mesh(name + "_feed", (x + 0.8, y + 2.6, z), (x + 1.6, y + 2.6, z), 0.18)
    tube_mesh(name + "_elbow", (x + 1.6, y + 2.6, z), (x + 1.6, y + 4.8, z), 0.18)
    cube("deco_" + name + "_gauge", (x, y + 2, z + 0.82), (0.55, 0.45, 0.1), "lab_stripe", (0.1, 0.7, 0.8))


def room(index, entry):
    name, x, y, z, width, depth, height = entry
    floor_mat = "floor_plate" if index in (7, 14, 15, 17, 18) else "lab_tile"
    wall_mat = "concrete_dark" if index in (0, 11, 19, 21) else "lab_panel"
    if index in (4, 10, 22):
        floor_mat, wall_mat = "metal_panel", "concrete"
    elif index in (5, 12):
        floor_mat, wall_mat = "tile_blue", "lab_tile"
    elif index in (6, 13, 16, 23):
        floor_mat, wall_mat = "lab_tile", "tile_dirty"
    cube(name + "_floor", (x, y - 0.5, z), (width, 1, depth), floor_mat)
    if index not in (2, 3, 7, 14, 15, 17, 18, 19, 20):
        cube(name + "_ceiling", (x, y + height + 0.5, z), (width, 1, depth), "metal_panel")
    for side in ("n", "s", "w", "e"):
        axis = 0 if side in ("n", "s") else 2
        span = width if axis == 0 else depth - 2
        fixed = z + (-depth / 2 + 0.5 if side == "n" else depth / 2 - 0.5) if axis == 0 else x + (-width / 2 + 0.5 if side == "w" else width / 2 - 0.5)
        center = x if axis == 0 else z
        opening = side in OPENINGS[index]
        window = VIEW_SIDES.get(index) == side and not opening
        side_height = 1.2 if window else height
        pieces = [(-span / 2, -3), (3, span / 2)] if opening else [(-span / 2, span / 2)]
        for number, (start, end) in enumerate(pieces):
            if end <= start:
                continue
            at = (x, y + side_height / 2, z)
            at = list(at)
            at[axis] = center + (start + end) / 2
            at[2 if axis == 0 else 0] = fixed
            size = (end - start, side_height, 1) if axis == 0 else (1, side_height, end - start)
            cube(f"{name}_{side}_{number}", at, size, wall_mat)
        if opening and height > 6 and index != 19:
            at = (x, y + 6 + (height - 6) / 2, fixed) if axis == 0 else (fixed, y + 6 + (height - 6) / 2, z)
            size = (6, height - 6, 1) if axis == 0 else (1, height - 6, 6)
            cube(f"{name}_{side}_lintel", at, size, wall_mat)
    color = (0.4, 0.75, 1.0) if index % 3 else (1, 0.63, 0.24)
    lamp("lamp_" + name, (x, y + min(height - 1, 7), z), color, 1500 if width > 20 else 750, max(width, depth) * 0.8)
    for side in (-1, 1):
        for k in range(max(1, int(depth / 9))):
            cube(f"deco_{name}_wallstrip_{side}_{k}", (x + side * (width / 2 - 1.1), y + 2.8, z - depth / 2 + 3 + k * 8), (0.12, 0.18, 3), "lab_stripe", (0.4, 0.8, 0.85))
    if index not in (0, 7, 14, 17, 18, 19):
        cube(f"deco_{name}_console", (x - width / 2 + 1.8, y + 1, z - depth / 2 + 2.5), (1.4, 2, 1.2), "metal_brushed")
        cube(f"deco_{name}_screen", (x - width / 2 + 1.8, y + 1.6, z - depth / 2 + 3.12), (1.1, 0.55, 0.04), "lab_stripe", (0.25, 0.8, 0.85))
    if index in (4, 5, 10, 12, 16, 22):
        for sign in (-1, 1):
            apparatus(f"{name}_apparatus_{sign}", x + sign * (width / 2 - 2.8), y, z - depth / 2 + 3.5)
    if width > 18:
        for sign in (-1, 1):
            tube_mesh(f"{name}_ceiling_feed_{sign}", (x + sign * (width / 2 - 1.8), y + height - 0.7, z - depth / 2 + 1), (x + sign * (width / 2 - 1.8), y + height - 0.7, z + depth / 2 - 1), 0.18)
    if index in (5, 12, 22):
        for offset in (-7, 7):
            cube(f"deco_{name}_gantry_{offset}", (x, y + height - 1.0, z + offset), (width - 2, 0.7, 1.3), "metal_brushed")
    return nav((x, y, z))


def connect(a, b):
    ra, rb = ROOMS[a], ROOMS[b]
    _, ax, ay, az, aw, ad, _ = ra
    _, bx, by, bz, bw, bd, _ = rb
    along_x = abs(bx - ax) > abs(bz - az)
    sign = 1 if (bx - ax if along_x else bz - az) > 0 else -1
    start = (ax + sign * aw / 2, ay, az) if along_x else (ax, ay, az + sign * ad / 2)
    end = (bx - sign * bw / 2, by, bz) if along_x else (bx, by, bz - sign * bd / 2)
    length = abs(end[0] - start[0]) if along_x else abs(end[2] - start[2])
    count = max(1, math.ceil(abs(by - ay) / 0.25))
    step = length / count
    previous = a
    for i in range(count):
        t = (i + 0.5) / count
        height = ay + (by - ay) * (i + 1) / count
        center = (start[0] + (end[0] - start[0]) * t, height - 0.5, start[2] + (end[2] - start[2]) * t)
        size = (step, 1, 6) if along_x else (6, 1, step)
        cube(f"link_{a}_{b}_step_{i}", center, size, "floor_plate")
        if i == 0 or i == count - 1 or i % 8 == 0:
            index = nav((center[0], height, center[2]))
            edge(previous, index)
            previous = index
    edge(previous, b)
    for side in (-1, 1):
        center = ((start[0] + end[0]) / 2, max(ay, by) + 2.8, (start[2] + end[2]) / 2)
        center = list(center)
        center[2 if along_x else 0] += side * 3.5
        open_walkway = a in (6, 7, 13, 14, 15, 17)
        side_height = 1.2 if open_walkway else 5.6 + abs(by - ay)
        size = (length, side_height, 1) if along_x else (1, side_height, length)
        center[1] = (ay + by) / 2 + (0.6 if open_walkway else 2.8)
        cube(f"link_{a}_{b}_rail_{side}", center, size, "concrete_dark")
    return start, end


def zone(name, center, size=(6, 4, 2), mat="warning", color=(0.85, 0.4, 0.1)):
    cube(name, center, size, mat, color)


def arena(index, slug, waves, machine=False):
    _, x, y, z, width, depth, _ = ROOMS[index]
    entry_sign = 1 if index in (12, 22) else -1
    zone(f"arena_{slug}_trigger", (x + entry_sign * (width / 2 - 5), y + 2, z), (7, 4, depth - 2))
    zone(f"arena_{slug}_entry", (x + entry_sign * width / 2, y + 3, z), (0.5, 6, 6), "metal_panel")
    zone(f"arena_{slug}_exit", (x - entry_sign * width / 2, y + 3, z), (0.5, 6, 6), "metal_panel")
    if index == 22:
        bpy.data.objects.remove(bpy.data.objects[f"arena_{slug}_exit"], do_unlink=True)
        zone(f"arena_{slug}_exit", (x, y + 3, z + depth / 2), (6, 6, 0.5), "metal_panel")
    for wave in range(1, waves + 1):
        for i in range(3 + wave):
            angle = math.tau * i / (3 + wave)
            marker(f"arena_{slug}_wave_{wave}", (x + math.cos(angle) * 8, y + 0.05, z + math.sin(angle) * 8))
    rules = f'(trigger: "arena_{slug}_trigger", doors: ["arena_{slug}_entry", "arena_{slug}_exit"], waves: [' + ", ".join(f'"arena_{slug}_wave_{i}"' for i in range(1, waves + 1)) + "]"
    if machine:
        zone(f"arena_{slug}_machine", (x, y + 1.4, z - 10), (3, 2.8, 2), "metal_panel")
        marker(f"arena_{slug}_machine_spawn", (x + 7, y + 0.05, z - 8))
        rules += f', machine: Some((object: "arena_{slug}_machine", spawn_point: "arena_{slug}_machine_spawn", health: 12.0, spawn_interval_seconds: 3.5, max_alive: 4))'
    rules += ")"
    for dx in ((-8,) if index == 12 else (-8, 8)):
        cube(f"{slug}_cover_{dx}", (x + dx, y + 1, z + 5), (3, 2, 2), "metal_panel")
    return rules


bpy.ops.wm.read_factory_settings(use_empty=True)
room_openings()
for index, entry in enumerate(ROOMS):
    room(index, entry)
for a, b in LINKS:
    connect(a, b)

for x, z in [(-8, -10), (20, -48), (42, -102)]:
    tube_mesh(f"tower_{x}_spine", (x, -6, z), (x, 43, z), 2.4, "metal_panel", (0.5, 0.6, 0.65))
    for height in (2, 12, 22, 32, 41):
        tube_mesh(f"tower_{x}_ring_{height}", (x, height, z), (x, height + 0.7, z), 3.7, "concrete_dark")
    for dx in (-2.5, 2.5):
        tube_mesh(f"tower_{x}_light_{dx}", (x + dx, -4, z), (x + dx, 42, z), 0.16, "lab_stripe", (0.08, 0.7, 0.95))
for y in (10, 26, 40):
    tube_mesh(f"service_bridge_{y}", (-8, y, -10), (20, y, -48), 0.45, "metal_brushed")

cube("high_chute_approach", (128, 35.5, -110), (6, 1, 26), "floor_plate")
zone("breakable_chute", (128, 35.5, -92), (6, 1, 10), "floor_plate")
zone("trigger_drop", (128, 37.2, -93), (6, 4, 4))
zone("trigger_drop_hint", (128, 37.2, -117), (6, 4, 3))
for x in (124.5, 131.5):
    cube("chute_guard", (x, 37, -110), (1, 2, 26), "metal_brushed")

marker("player_start", (-48, 0, 40))
for x in (-36, -32, -28):
    zone("target", (x, 1.5, 37.5), (1.2, 2, 0.5), "warning")
zone("key_access", (96, 3.2, 8), (0.6, 0.7, 0.6), "warning")
zone("door_access", (96, 5.75, -3), (5.8, 6, 0.5), "metal_panel")
zone("trigger_access", (96, 4, 0), (6, 4, 4))
zone("key_dispatch", (-84, 21.2, -78), (0.6, 0.7, 0.6), "warning")
zone("door_dispatch", (-66, 1, -23), (6, 6, 0.5), "metal_panel")
zone("trigger_dispatch", (-66, 1, -27), (6, 4, 4))
zone("exit", (-66, -2, -5), (6, 4, 3))

checkpoints = []
for slug, p in [("workshop", (43, 0.8, 8)), ("permit", (94, 2.8, 8)), ("calibration", (-28, 16.8, -78)), ("crown", (128, 36.8, -132)), ("bowl", (128, -3.2, -46)), ("final", (-44, 2.8, -46))]:
    name = "checkpoint_" + slug
    zone(name, p, (4, 1.6, 4))
    checkpoints.append(name)

spawns = []
triggers = []
for index, slug in [(4, "workshop"), (10, "load"), (17, "bridge")]:
    _, x, y, z, width, depth, _ = ROOMS[index]
    zone("trigger_" + slug, (x - width / 2 + 2, y + 2, z), (4, 4, depth - 2))
    for i in range(3):
        marker("spawn_" + slug, (x + 2 + i * 2, y + 0.05, z + (-3 if i % 2 else 3)))
    triggers.append(f'(zone: "trigger_{slug}", actions: [Spawn("spawn_{slug}")])')

zone("hazard_service", (-2, -3.5, -46), (3, 1, 5), "warning", (0.9, 0.3, 0.04))
zone("switch_service", (2, -2.5, -42), (0.8, 0.8, 0.8), "lab_stripe", (0.1, 0.7, 0.8))
zone("trigger_arrival", (-43, 2, 40), (2, 4, 6))
triggers.extend([
    '(zone: "trigger_arrival", actions: [Say("dispatcher.laboratory_arrival")])',
    '(zone: "trigger_access", requires_key: Some("access"), actions: [Open("access")])',
    '(zone: "trigger_dispatch", requires_key: Some("dispatch"), actions: [Open("dispatch")])',
    '(zone: "trigger_drop_hint", actions: [Say("dispatcher.laboratory_drop")])',
    '(zone: "trigger_drop", actions: [Break("chute")])',
])
arenas = [arena(5, "abrasive", 2), arena(12, "calibration", 2), arena(22, "final", 3, True)]

cube("calibration_mezzanine", (-52, 19.7, -85.5), (12, 0.6, 7), "floor_plate")
base = nav((-34, 16, -85.5))
edge(12, base)
for i in range(16):
    top = 16 + (i + 1) * 0.25
    x = -34 - (i + 0.5) * 0.75
    cube(f"calibration_stair_{i}", (x, top - 0.5, -85.5), (0.75, 1, 4), "floor_plate")
    if i % 4 == 0 or i == 15:
        step_node = nav((x, top, -85.5))
        edge(base, step_node)
        base = step_node
upper = nav((-52, 20, -85.5))
edge(base, upper)
for wave in (1, 2):
    marker(f"arena_calibration_wave_{wave}", (-52, 20.05, -85.5))
zone("key_archive", (-40, 31.2, -164), (0.5, 0.6, 0.5), "lab_stripe", (0.2, 0.8, 0.9))

for name, center in [("core_lower", (-8, 4, -10)), ("core_upper", (-8, 26, -10))]:
    cube(name, center, (3, 8, 3), "lab_stripe", (0.1, 0.65, 0.9))

nav_text = ",\n".join('(position: (' + ", ".join(str(round(v, 4)) for v in node["position"]) + '), links: [' + ", ".join(map(str, node["links"])) + '])' for node in NAV)
ron = '''(
    model: "laboratory.glb",
    atmosphere: (sky_color: (0.035, 0.055, 0.075), fog_start: 50.0, fog_end: 210.0, sun_direction: (-0.3, -1.0, -0.4), sun_strength: 0.08, ambient: 0.1),
    target_health: 8.0,
    light_scale: 0.0006,
    default_light_range: 22.0,
    validate_geometry: true,
    spawn_yaw_degrees: Some(90.0),
    arenas: [ARENAS],
    switches: [(object: "switch_service", health: 1.0, actions: [DisableHazard("service")])],
    hazards: [(object: "hazard_service", damage_per_second: 20.0)],
    breakables: [(group: "chute", by_slam: true)],
    triggers: [TRIGGERS],
    checkpoints: [CHECKPOINTS],
    navigation: [NAVIGATION],
)
'''.replace("ARENAS", ",\n".join(arenas)).replace("TRIGGERS", ",\n".join(triggers)).replace("CHECKPOINTS", ", ".join('"' + name + '"' for name in checkpoints)).replace("NAVIGATION", nav_text)
(ROOT / "assets/levels/laboratory.ron").write_text(ron, encoding="utf-8")
(ROOT / "art/levels").mkdir(parents=True, exist_ok=True)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "art/levels/laboratory.blend"))
bpy.ops.export_scene.gltf(filepath=str(ROOT / "assets/levels/laboratory.glb"), export_format="GLB", export_lights=True, export_yup=True)
print(f"Laboratory: {len(ROOMS)} spaces, {len(NAV)} navigation nodes")
