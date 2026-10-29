import math
import os

import bpy
from mathutils import Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BLEND_PATH = os.path.join(ROOT, "art", "levels", "foundry.blend")
GLB_PATH = os.path.join(ROOT, "assets", "levels", "foundry.glb")

STEEL = (0.45, 0.47, 0.50)
SOOT = (0.22, 0.20, 0.20)
SLAG = (0.30, 0.24, 0.20)
BRICK = (0.55, 0.25, 0.18)
MOLTEN = (1.0, 0.45, 0.08)
KEY_GOLD = (1.0, 0.85, 0.2)
VALVE_RED = (0.9, 0.1, 0.1)
HAZARD = (0.85, 0.65, 0.10)
CONVEYOR = (0.18, 0.18, 0.20)
DARK_CONCRETE = (0.30, 0.28, 0.27)
RUST = (0.55, 0.32, 0.18)
PIPE = (0.35, 0.40, 0.42)
ZONE = (0.9, 0.1, 0.9)
SODIUM = (1.0, 0.62, 0.28)
FLUORESCENT = (0.75, 0.88, 1.0)
EMERGENCY = (1.0, 0.12, 0.08)
ORANGE_GLOW = (1.0, 0.45, 0.08)

WALL = 1.0
materials = {}


def to_blender(vector):
    x, y, z = vector
    return (x, -z, y)


def size_to_blender(size):
    x, y, z = size
    return (x, z, y)


def material(color):
    if color in materials:
        return materials[color]
    linear = tuple(channel**2.2 for channel in color)
    result = bpy.data.materials.new(name=f"color_{len(materials)}")
    result.diffuse_color = (*linear, 1.0)
    if result.node_tree is None:
        result.use_nodes = True
    principled = result.node_tree.nodes.get("Principled BSDF")
    principled.inputs["Base Color"].default_value = (*linear, 1.0)
    materials[color] = result
    return result


def cube(name, center, size, color, yaw_degrees=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=to_blender(center))
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = size_to_blender(size)
    obj.rotation_euler = (0.0, 0.0, math.radians(yaw_degrees))
    obj.data.materials.append(material(color))
    return obj


def zone(name, center, size):
    obj = cube(name, center, size, ZONE)
    obj.display_type = "WIRE"
    return obj


def slab(name, x0, x1, y0, y1, z0, z1, color):
    cube(
        name,
        ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2),
        (x1 - x0, y1 - y0, z1 - z0),
        color,
    )


def subtract_rect(rect, hole):
    ax0, ax1, az0, az1 = rect
    hx0, hx1, hz0, hz1 = hole
    ix0, ix1 = max(ax0, hx0), min(ax1, hx1)
    iz0, iz1 = max(az0, hz0), min(az1, hz1)
    if ix0 >= ix1 or iz0 >= iz1:
        return [rect]
    pieces = []
    if iz0 > az0:
        pieces.append((ax0, ax1, az0, iz0))
    if iz1 < az1:
        pieces.append((ax0, ax1, iz1, az1))
    if ix0 > ax0:
        pieces.append((ax0, ix0, iz0, iz1))
    if ix1 < ax1:
        pieces.append((ix1, ax1, iz0, iz1))
    return pieces


def floor_slabs(name, x0, x1, z0, z1, y0, y1, holes, color):
    rects = [(min(x0, x1), max(x0, x1), min(z0, z1), max(z0, z1))]
    for hole in holes:
        rects = [piece for rect in rects for piece in subtract_rect(rect, hole)]
    for index, (ax0, ax1, az0, az1) in enumerate(rects):
        slab(f"{name}_{index}", ax0, ax1, y0, y1, az0, az1, color)


def ring_catwalk(name, cx, cz, inner, outer, bottom, top, color, segments=16):
    for index in range(segments):
        mid = 2 * math.pi * (index + 0.5) / segments
        radius = (inner + outer) / 2
        x = cx + radius * math.sin(mid)
        z = cz - radius * math.cos(mid)
        length = 2 * radius * math.tan(math.pi / segments) + 0.02
        cube(
            f"{name}_{index}",
            (x, (bottom + top) / 2, z),
            (length, top - bottom, outer - inner),
            color,
            yaw_degrees=-math.degrees(mid),
        )


def ring_rail(name, cx, cz, radius, y, color, segments=16):
    for index in range(segments):
        mid = 2 * math.pi * (index + 0.5) / segments
        x = cx + radius * math.sin(mid)
        z = cz - radius * math.cos(mid)
        length = 2 * radius * math.tan(math.pi / segments)
        cube(
            f"{name}_{index}",
            (x, y + 0.5, z),
            (length, 1.0, 0.08),
            color,
            yaw_degrees=-math.degrees(mid),
        )


def spiral_stairs(name, cx, cz, radius, y0, y1, color, steps, width):
    rise = (y1 - y0) / steps
    for index in range(steps):
        angle = 2 * math.pi * index / steps
        x = cx + radius * math.sin(angle)
        z = cz - radius * math.cos(angle)
        length = 2 * math.pi * radius / steps + 0.05
        cube(
            f"{name}_{index}",
            (x, y0 + rise * (index + 1) - 0.1, z),
            (length, 0.2, width),
            color,
            yaw_degrees=-math.degrees(angle),
        )


def recess(name, x0, x1, z0, z1, bottom, top, color, cross_x=(), cross_z=()):
    low_x, high_x = min(x0, x1), max(x0, x1)
    low_z, high_z = min(z0, z1), max(z0, z1)
    lining = top - 1.0
    wall(f"{name}_west", "z", low_x - WALL / 2, low_z, high_z, bottom, lining, color, cross_z)
    wall(f"{name}_east", "z", high_x + WALL / 2, low_z, high_z, bottom, lining, color, cross_z)
    wall(f"{name}_south", "x", high_z + WALL / 2, low_x, high_x, bottom, lining, color, cross_x)
    wall(f"{name}_north", "x", low_z - WALL / 2, low_x, high_x, bottom, lining, color, cross_x)


def lamp(position, color, power, distance):
    light = bpy.data.lights.new(name="lamp", type="POINT")
    light.color = tuple(channel**2.2 for channel in color)
    light.energy = power
    light.use_custom_distance = True
    light.cutoff_distance = distance
    obj = bpy.data.objects.new("lamp", light)
    obj.location = to_blender(position)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def empty(name, position, direction=None):
    obj = bpy.data.objects.new(name, None)
    obj.location = to_blender(position)
    obj.empty_display_type = "SINGLE_ARROW"
    obj.empty_display_size = 1.5
    if direction is None:
        obj.rotation_euler = (math.radians(-90.0), 0.0, 0.0)
    else:
        obj.rotation_mode = "QUATERNION"
        obj.rotation_quaternion = Vector(to_blender(direction)).to_track_quat("Z", "Y")
    bpy.context.scene.collection.objects.link(obj)
    return obj


def wall(name, along, line, start, end, bottom, top, color, holes=()):
    low, high = min(start, end), max(start, end)
    segments = []
    cursor = low
    for hole_start, hole_end, hole_bottom, hole_top in sorted(holes):
        hole_start, hole_end = min(hole_start, hole_end), max(hole_start, hole_end)
        hole_start, hole_end = max(hole_start, low), min(hole_end, high)
        if hole_end <= hole_start:
            continue
        if hole_start > cursor:
            segments.append((cursor, hole_start, bottom, top))
        if hole_bottom > bottom + 0.001:
            segments.append((hole_start, hole_end, bottom, hole_bottom))
        if hole_top < top - 0.001:
            segments.append((hole_start, hole_end, hole_top, top))
        cursor = hole_end
    if cursor < high:
        segments.append((cursor, high, bottom, top))
    for index, (s, e, b, t) in enumerate(segments):
        if e - s <= 0.001 or t - b <= 0.001:
            continue
        if along == "x":
            center = ((s + e) / 2, (b + t) / 2, line)
            size = (e - s, t - b, WALL)
        else:
            center = (line, (b + t) / 2, (s + e) / 2)
            size = (WALL, t - b, e - s)
        cube(f"{name}_{index}", center, size, color)


def room(name, x0, x1, z0, z1, floor_y, height, color, holes=None, ceiling=True, floor=True, base=None):
    holes = holes or {}
    low_x, high_x = min(x0, x1), max(x0, x1)
    low_z, high_z = min(z0, z1), max(z0, z1)
    top = floor_y + height
    bottom = floor_y if base is None else base
    wall(f"{name}_west", "z", low_x - WALL / 2, low_z, high_z, bottom, top, color, holes.get("west", ()))
    wall(f"{name}_east", "z", high_x + WALL / 2, low_z, high_z, bottom, top, color, holes.get("east", ()))
    wall(f"{name}_south", "x", high_z + WALL / 2, low_x, high_x, bottom, top, color, holes.get("south", ()))
    wall(f"{name}_north", "x", low_z - WALL / 2, low_x, high_x, bottom, top, color, holes.get("north", ()))
    if floor:
        floor_slabs(
            f"{name}_floor",
            low_x,
            high_x,
            low_z,
            high_z,
            floor_y - 1.0,
            floor_y,
            holes.get("floor", ()),
            DARK_CONCRETE,
        )
    if ceiling:
        slab(f"{name}_ceiling", low_x, high_x, top, top + 1.0, low_z, high_z, DARK_CONCRETE)


def round_room(name, cx, cz, radius, floor_y, height, color, holes=None, segments=16, floor_holes=()):
    holes = holes or {}
    top = floor_y + height
    corners = []
    for index in range(segments):
        angle = 2 * math.pi * index / segments
        corners.append((cx + radius * math.sin(angle), cz - radius * math.cos(angle)))
    length = 2 * radius * math.tan(math.pi / segments)
    for index in range(segments):
        x0, z0 = corners[index]
        x1, z1 = corners[(index + 1) % segments]
        mx, mz = (x0 + x1) / 2, (z0 + z1) / 2
        yaw = math.degrees(math.atan2(-(z1 - z0), x1 - x0))
        bottom, hole_top = holes.get(index, (None, None))
        pieces = []
        if bottom is None:
            pieces.append((floor_y, top))
        else:
            if bottom > floor_y + 0.001:
                pieces.append((floor_y, bottom))
            if hole_top < top - 0.001:
                pieces.append((hole_top, top))
        for piece_index, (y0, y1) in enumerate(pieces):
            cube(
                f"{name}_{index}_{piece_index}",
                (mx, (y0 + y1) / 2, mz),
                (length + 0.2, y1 - y0, WALL),
                color,
                yaw_degrees=yaw,
            )
    floor_slabs(
        f"{name}_floor",
        cx - radius,
        cx + radius,
        cz - radius,
        cz + radius,
        floor_y - 1.0,
        floor_y + 0.004,
        floor_holes,
        DARK_CONCRETE,
    )
    slab(
        f"{name}_ceiling",
        cx - radius,
        cx + radius,
        top + 0.004,
        top + 1.0,
        cz - radius,
        cz + radius,
        DARK_CONCRETE,
    )


def stairs(name, along, line, start, end, base_y, steps, rise, width, color):
    span = (end - start) / steps
    for index in range(steps):
        a = start + span * index
        b = a + span
        top = base_y + rise * (index + 1)
        if along == "x":
            slab(f"{name}_{index}", min(a, b), max(a, b), base_y, top, line - width / 2, line + width / 2, color)
        else:
            slab(f"{name}_{index}", line - width / 2, line + width / 2, base_y, top, min(a, b), max(a, b), color)


def rail(name, along, line, start, end, y, color):
    low, high = min(start, end), max(start, end)
    for index, height in enumerate((0.5, 1.0)):
        if along == "x":
            cube(f"{name}_bar_{index}", ((low + high) / 2, y + height, line), (high - low, 0.08, 0.08), color)
        else:
            cube(f"{name}_bar_{index}", (line, y + height, (low + high) / 2), (0.08, 0.08, high - low), color)
    posts = max(2, int((high - low) / 3.0) + 1)
    for index in range(posts):
        t = low + (high - low) * index / (posts - 1)
        if along == "x":
            cube(f"{name}_post_{index}", (t, y + 0.5, line), (0.08, 1.0, 0.08), color)
        else:
            cube(f"{name}_post_{index}", (line, y + 0.5, t), (0.08, 1.0, 0.08), color)


def reception():
    room("reception", -8, 8, 0, -16, 0, 8, SOOT, holes={"north": [(-3, 3, 0, 5)]})
    slab("reception_ledge", -2, 2, 0, 4, -4, 0, STEEL)
    slab("reception_pipe", -1.8, 1.8, 6, 8, -1.0, -0.4, PIPE)
    cube("reception_pipe_dark", (0, 7, -0.6), (2.6, 1.8, 0.2), DARK_CONCRETE)
    zone("checkpoint_reception", (0, 0, -12), (4, 4, 4))
    cube("belt_intro", (0, 0.25, -9), (3, 0.5, 10), CONVEYOR)
    cube("belt_intro_rail", (-1.75, 0.4, -9), (0.5, 0.8, 10), HAZARD)
    cube("belt_intro_rail", (1.75, 0.4, -9), (0.5, 0.8, 10), HAZARD)
    zone("trigger_arrival", (0, 2, -3), (8, 4, 4))
    empty("player_start", (0, 4, -2))
    lamp((-5, 6.5, -5), SODIUM, 1500.0, 16.0)
    lamp((5, 6.5, -11), SODIUM, 1500.0, 16.0)


def sorting():
    room(
        "sorting",
        -24,
        24,
        -16,
        -64,
        0,
        14,
        SOOT,
        holes={
            "south": [(-3, 3, 0, 5)],
            "north": [(-2, 2, 8, 12)],
            "east": [(-48, -40, 0, 8)],
            "west": [(-52, -44, 8, 11)],
            "floor": [(-14, 14, -45, -35)],
        },
    )
    cube("belt_sort_1", (0, 0.25, -24), (40, 0.5, 3), CONVEYOR)
    cube("belt_sort_2", (0, 0.25, -32), (40, 0.5, 3), CONVEYOR)
    cube("belt_sort_3", (6, 0.25, -48), (28, 0.5, 3), CONVEYOR)
    slab("sorting_pit_floor", -14, 14, -5, -4, -45, -35, DARK_CONCRETE)
    recess("sorting_pit", -14, 14, -45, -35, -4, 0, SOOT)
    for index in range(8):
        x0 = -6 - index * 1.0
        slab(f"sorting_pit_ramp_{index}", x0 - 1.0, x0, -4, -4 + (index + 1) * 0.5, -45, -35, DARK_CONCRETE)
    for x, z, size in (
        (-18, -19, (2.0, 2.4, 2.0)),
        (-10, -28, (2.5, 2.4, 2.5)),
        (6, -20, (3.0, 3.0, 2.0)),
        (16, -27, (2.0, 2.0, 3.0)),
        (19, -20, (2.0, 3.0, 2.0)),
        (-4, -54, (3.0, 2.0, 2.0)),
        (12, -56, (2.5, 3.5, 2.5)),
        (-19, -60, (2.0, 2.0, 2.0)),
    ):
        cube("sorting_crate", (x, size[1] / 2, z), size, RUST)
    zone("checkpoint_sorting", (0, 0, -30), (4, 4, 4))
    for x in (-16, -8, 8, 16):
        empty("enemy", (x, 0, -56))
    for x, z in ((-20, -30), (20, -30), (-20, -58), (20, -58)):
        lamp((x, 11, z), SODIUM, 4000.0, 26.0)
    lamp((22, 7, -44), EMERGENCY, 500.0, 9.0)
    slab("sorting_balcony", -24, -18, 7.5, 8, -56, -44, STEEL)
    rail("sorting_balcony_rail", "z", -18, -56, -44, 8, STEEL)
    rail("sorting_balcony_rail", "x", -44, -24, -18, 8, STEEL)
    rail("sorting_balcony_rail", "x", -56, -24, -18, 8, STEEL)
    stairs("sorting_stairs", "x", -62, 14, -2, 0, 32, 0.25, 3.0, STEEL)
    cube("door_gate", (24, 4, -44), (1, 8, 8), STEEL)
    cube("sorting_gate_stripe", (23.4, 8.5, -44), (0.2, 1.0, 9), HAZARD)
    zone("trigger_gate_hint", (21, 2, -44), (4, 4, 12))
    zone("trigger_gate", (22, 2, -44), (3, 4, 8))


def catwalks():
    room(
        "catwalks",
        -14,
        14,
        -64,
        -108,
        -10,
        28,
        BRICK,
        holes={"south": [(-2, 2, 8, 12)], "west": [(-88, -82, -2, 2)]},
    )
    slab("hazard_molten", -14, 14, -10, -8, -108, -64, MOLTEN)
    slab("catwalks_island", -9, 9, -10, -2, -100, -70, SLAG)
    slab("catwalks_bridge_west", -14, -9, -2.5, -2, -88, -84, SLAG)
    slab("catwalks_hint", -13.5, -9.5, -2, -1.9, -86.6, -85.4, HAZARD)
    slab("catwalks_hint", -8, 6, -2, -1.9, -86.6, -85.4, HAZARD)
    lamp((-11, 2, -86), EMERGENCY, 700.0, 12.0)
    slab("catwalks_walk_1", -1.5, 1.5, 7.7, 8, -76, -63.4, STEEL)
    zone("checkpoint_catwalk", (0, 8, -70), (4, 4, 4))
    slab("catwalks_door_landing", -2.5, 2.5, 7.7, 8, -64.6, -63.4, STEEL)
    for index, z in enumerate((-78, -82, -86, -90)):
        cube("breakable_catwalk", (0, 7.85, z), (3, 0.3, 4), STEEL)
    slab("catwalks_walk_2", -1.5, 1.5, 7.7, 8, -108, -92, STEEL)
    rail("catwalks_rail", "z", -1.6, -108, -64, 8, STEEL)
    rail("catwalks_rail", "z", 1.6, -108, -64, 8, STEEL)
    cube("catwalks_sealed_door", (0, 11, -107.5), (3, 6, 0.6), STEEL)
    for index, (x, z) in enumerate(((-10, -70), (10, -84), (-10, -98))):
        cube(f"catwalks_ladle_{index}", (x, 3, z), (6, 6, 6), BRICK)
    for x, z in ((-6, -72), (6, -80), (-6, -96)):
        lamp((x, -4, z), ORANGE_GLOW, 700.0, 12.0)
    for x, z in ((-10, -68), (10, -68), (-10, -100), (10, -100)):
        lamp((x, 15, z), SODIUM, 1500.0, 20.0)
    zone("trigger_collapse", (0, 9.5, -84), (3, 3, 4))


def tunnels():
    room(
        "tunnel_a",
        -46,
        -14,
        -84,
        -88,
        -2,
        3.5,
        SOOT,
        holes={"east": [(-88, -84, -2, 1.5)], "south": [(-46, -42, -2, 1.5)]},
    )
    room(
        "tunnel_b",
        -46,
        -42,
        -60,
        -84,
        -2,
        3.5,
        SOOT,
        holes={"north": [(-46, -42, -2, 1.5)], "south": [(-46, -42, -2, 1.5)]},
    )
    zone("checkpoint_tunnels", (-16, 0, -86), (4, 4, 4))
    for index, z in enumerate((-76, -72)):
        cube(f"tunnel_pipe_{index}", (-44, 0.55, z), (4, 1.9, 1.0), PIPE)
    cube("tunnel_sign", (-44, 0.9, -79), (0.4, 1.8, 1.0), HAZARD)
    zone("trigger_ambush", (-29, 0, -86), (2, 4, 4))
    for position in ((-22, -2, -86), (-20, -2, -86), (-44, -2, -70), (-44, -2, -68)):
        empty("ambush", position)
    for z in (-64, -70, -76, -82):
        lamp((-45.5, 1.0, z), EMERGENCY, 500.0, 9.0)
    for x in (-40, -32, -24, -18):
        lamp((x, 1.0, -87), SODIUM, 700.0, 10.0)
    lamp((-45.5, 1.0, -60), SODIUM, 700.0, 10.0)
    lamp((-45.5, 1.0, -84), SODIUM, 700.0, 10.0)


def foreman():
    room(
        "foreman",
        -58,
        -42,
        -48,
        -60,
        -2,
        4,
        SOOT,
        holes={"north": [(-46, -42, -2, 1.5)], "east": [(-58, -54, -2, 2)]},
    )
    cube("foreman_desk", (-55.5, -1.5, -54), (1.5, 1.0, 3.0), RUST)
    cube("foreman_chair", (-53.5, -1.2, -54), (1.0, 1.6, 1.0), RUST)
    cube("key_master", (-54.3, -0.75, -53), (0.5, 0.5, 0.5), KEY_GOLD)
    zone("trigger_pass", (-55, 0.5, -53), (4, 4, 4))
    for position in ((-46, -2, -54), (-44, -2, -57)):
        empty("enemy", position)
    lamp((-50, 1.8, -54), SODIUM, 700.0, 10.0)


def shaft():
    room(
        "shaft",
        -42,
        -36,
        -58,
        -50,
        -2,
        16,
        DARK_CONCRETE,
        holes={"west": [(-58, -54, -2, 2)], "east": [(-56, -52, 8, 11)]},
    )
    slab("shaft_grate", -41, -37, -2.2, -2, -57, -51, STEEL)
    cube("pad_shaft", (-39, -1.5, -54), (3, 1, 3), CONVEYOR)
    zone("checkpoint_shaft", (-39, 0, -54), (5, 4, 5))
    zone("trigger_shaft_hint", (-39, -2, -48), (5, 4, 4))
    slab("shaft_pad_marker", -41.5, -36.5, -2.0, -1.98, -57.5, -50.5, HAZARD)
    lamp((-39, 13, -54), FLUORESCENT, 1500.0, 18.0)
    lamp((-34, 9.5, -54), SODIUM, 700.0, 10.0)


def compressor():
    room(
        "compressor",
        -36,
        -24,
        -58,
        -40,
        8,
        5,
        RUST,
        holes={"west": [(-56, -52, 8, 11)], "east": [(-52, -44, 8, 11)]},
    )
    cube("door_vent", (-24, 9.5, -48), (0.6, 3, 8), STEEL)
    cube("compressor_tank", (-33, 10.5, -50), (3, 4, 3), STEEL)
    cube("compressor_pipe", (-30, 10.5, -55), (1, 1, 5), PIPE)
    cube("switch_vent", (-30, 10, -57), (1, 1, 1), VALVE_RED)
    empty("enemy", (-28, 8, -46))
    lamp((-30, 12, -49), SODIUM, 700.0, 10.0)
    lamp((-27, 12, -42), EMERGENCY, 300.0, 7.0)


def molds():
    room(
        "molds",
        24,
        44,
        -40,
        -56,
        0,
        10,
        SOOT,
        holes={"west": [(-48, -40, 0, 8)], "east": [(-52, -44, 0, 8)]},
    )
    slab("molds_antresol", 24, 30, 5.5, 6, -56, -52, STEEL)
    slab("molds_antresol", 38, 44, 5.5, 6, -56, -52, STEEL)
    stairs("molds_stairs", "x", -50, 40, 26, 0, 12, 0.5, 2.0, STEEL)
    cube("breakable_shelf", (34, 5.75, -54), (8, 0.5, 4), STEEL)
    for x, z, size in (
        (28, -44, (3.0, 2.5, 2.0)),
        (32, -50, (2.0, 3.0, 2.0)),
        (40, -44, (2.0, 2.0, 3.0)),
        (42, -50, (2.0, 3.0, 2.0)),
        (28, -52, (2.5, 2.0, 2.5)),
    ):
        cube("molds_rack", (x, size[1] / 2, z), size, STEEL)
    for position in ((30, 0, -48), (40, 0, -47)):
        empty("enemy", position)
    zone("checkpoint_molds", (26, 2, -46), (4, 4, 10))
    zone("trigger_molds", (30, 2, -48), (4, 4, 8))
    lamp((34, 8.5, -46), SODIUM, 1500.0, 16.0)
    lamp((34, 8.5, -52), SODIUM, 1500.0, 16.0)
    lamp((28, 4, -54), SODIUM, 700.0, 10.0)


def presses():
    room(
        "presses",
        44,
        84,
        -44,
        -52,
        0,
        9,
        SOOT,
        holes={"west": [(-52, -44, 0, 8)], "east": [(-50, -46, 0, 6)]},
    )
    zone("checkpoint_presses", (46, 2, -48), (4, 4, 8))
    zone("trigger_presses", (49, 2, -48), (4, 4, 6))
    for index, x in enumerate((50, 57.5, 65, 72.5, 80)):
        cube(f"crusher_{index + 1}", (x, 7.5, -48), (3, 3, 8), STEEL)
        cube(f"presses_ram_{index}", (x, 8.6, -48), (1.2, 0.8, 8), STEEL)
        lamp((x, 8.2, -45), EMERGENCY, 300.0, 7.0)
    lamp((64, 8.5, -50), SODIUM, 1500.0, 18.0)


def furnace():
    round_room(
        "furnace",
        98,
        -48,
        14,
        0,
        20,
        BRICK,
        holes={3: (0, 6), 4: (0, 6), 5: (0, 6), 11: (0, 6), 12: (0, 6), 13: (0, 6)},
        floor_holes=[(88, 92, -62, -34), (104, 108, -62, -34)],
    )
    cube("furnace_body", (98, 6, -48), (8, 12, 8), BRICK)
    cube("furnace_mouth", (98, 1.5, -43.7), (4, 3, 0.4), ORANGE_GLOW)
    slab("hazard_channel", 88, 92, -2, -0.5, -62, -34, MOLTEN)
    slab("hazard_channel", 104, 108, -2, -0.5, -62, -34, MOLTEN)
    recess("furnace_channel_west", 88, 92, -62, -34, -2, 0, BRICK)
    recess("furnace_channel_east", 104, 108, -62, -34, -2, 0, BRICK)
    slab("furnace_bridge_1", 88, 92, 0, 0.3, -53, -51, STEEL)
    slab("furnace_bridge_2", 104, 108, 0, 0.3, -53, -51, STEEL)
    slab("furnace_bridge_3", 88, 92, 0, 0.3, -45, -43, STEEL)
    slab("furnace_bridge_4", 104, 108, 0, 0.3, -45, -43, STEEL)
    cube("pad_furnace", (98, 0.5, -56), (3, 1, 3), CONVEYOR)
    ring_catwalk("furnace_ring", 98, -48, 10, 14, 9.5, 10, STEEL)
    ring_rail("furnace_ring_rail", 98, -48, 10, 10, STEEL)
    cube("switch_furnace", (98, 10.5, -36.2), (1, 1, 1), VALVE_RED)
    cube("door_furnace", (111.6, 3, -48), (0.6, 6, 4), STEEL)
    zone("trigger_furnace", (86, 2, -48), (4, 4, 8))
    zone("trigger_furnace_hint", (109, 2, -48), (5, 4, 8))
    for position in ((95, 0, -56), (104, 0, -40)):
        empty("enemy", position)
    for x in (88, 104):
        lamp((x, 4, -58), ORANGE_GLOW, 700.0, 12.0)
        lamp((x, 4, -38), ORANGE_GLOW, 700.0, 12.0)
    lamp((98, 16, -48), SODIUM, 4000.0, 26.0)


def casting():
    room(
        "casting",
        112,
        172,
        -18,
        -78,
        0,
        20,
        SOOT,
        holes={
            "west": [(-50, -46, 0, 6)],
            "north": [(138, 146, 0, 6)],
            "floor": [(138, 142, -74, -22), (120, 168, -50, -46)],
        },
    )
    slab("hazard_channel", 138, 142, -3, -1, -74, -22, MOLTEN)
    slab("hazard_channel", 120, 168, -3, -1, -50, -46, MOLTEN)
    recess("casting_channel_z", 138, 142, -74, -22, -3, 0, BRICK, cross_z=[(-50, -46, -3, 0)])
    recess("casting_channel_x", 120, 168, -50, -46, -3, 0, BRICK, cross_x=[(138, 142, -3, 0)])
    for index, (x, z) in enumerate(
        (
            (140, -30),
            (140, -38),
            (140, -56),
            (140, -64),
            (124, -48),
            (132, -48),
            (150, -48),
            (158, -48),
        )
    ):
        slab(f"casting_bridge_{index}", x - 1, x + 1, -0.3, 0, z - 1, z + 1, STEEL)
    slab("casting_ring_west", 112, 115, 7.7, 8, -78, -18, STEEL)
    slab("casting_ring_east", 169, 172, 7.7, 8, -78, -18, STEEL)
    slab("casting_ring_north", 112, 172, 7.7, 8, -78, -75, STEEL)
    slab("casting_ring_south", 112, 172, 7.7, 8, -21, -18, STEEL)
    rail("casting_ring_rail", "z", 115, -78, -18, 8, STEEL)
    rail("casting_ring_rail", "z", 169, -78, -18, 8, STEEL)
    rail("casting_ring_rail", "x", -75, 112, 172, 8, STEEL)
    rail("casting_ring_rail", "x", -21, 112, 172, 8, STEEL)
    for index, (x, z) in enumerate(((116, -72), (168, -72), (116, -24), (168, -24))):
        cube(f"pad_casting_{index + 1}", (x, 0.5, z), (3, 1, 3), CONVEYOR)
    cube("tube_arena_a", (120, 1.5, -70), (2, 3, 2), PIPE)
    empty("tube_arena_a_exit", (166, 8.5, -22), (0, 0, -1))
    cube("tube_arena_b", (164, 1.5, -26), (2, 3, 2), PIPE)
    empty("tube_arena_b_exit", (118, 8.5, -74), (0, 0, 1))
    cube("arena_casting_machine", (168, 2.5, -62), (4, 5, 6), RUST)
    empty("arena_casting_machine_spawn", (162, 0, -61))
    for index, (x, z) in enumerate(((124, -30), (160, -30), (124, -66), (160, -66))):
        cube(f"casting_column_{index}", (x, 10, z), (3, 20, 3), BRICK)
    for x, z in ((122, -25), (150, -22), (122, -70), (150, -72)):
        empty("enemy", (x, 0, z))
    zone("arena_casting_trigger", (116, 3, -48), (6, 6, 10))
    zone("arena_casting_door", (112, 3, -48), (0.6, 6, 4))
    zone("arena_casting_exit", (142, 3, -78), (8, 6, 0.6))
    zone("checkpoint_casting", (114, 2, -48), (3, 4, 8))
    zone("trigger_casting_hint", (116, 2, -44), (5, 4, 4))
    for position in ((116, 0, -70), (167, 0, -72), (116, 0, -26), (167, 0, -26)):
        empty("arena_casting_wave_1", position)
    for position in ((120, 0, -72), (164, 0, -72), (140, 0, -74), (150, 0, -60), (130, 0, -30), (160, 0, -40)):
        empty("arena_casting_wave_2", position)
    for position in ((118, 0, -24), (166, 0, -24), (146, 0, -26), (150, 0, -70), (122, 0, -60), (162, 0, -44)):
        empty("arena_casting_wave_3", position)
    for x, z in ((120, -30), (150, -30), (120, -66), (150, -66), (146, -22), (146, -74)):
        lamp((x, 16, z), SODIUM, 4000.0, 26.0)
    for x in (124, 156):
        lamp((x, 4, -48), ORANGE_GLOW, 1000.0, 14.0)


def pouring():
    room(
        "pouring",
        130,
        154,
        -78,
        -122,
        0,
        16,
        SOOT,
        holes={"south": [(138, 146, 0, 6)], "north": [(138, 146, 0, 6)], "floor": [(130, 154, -112, -94)]},
        base=-2.0,
    )
    slab("pouring_river_bed", 130, 154, -2.5, -2, -112, -94, SLAG)
    slab("hazard_river", 130, 154, -2, -0.5, -112, -94, MOLTEN)
    wall("pouring_river_south", "x", -93.5, 130, 154, -2, 0, SLAG)
    wall("pouring_river_north", "x", -112.5, 130, 154, -2, 0, SLAG)
    cube("hazard_river.001", (142, 7, -97), (2, 14, 2), MOLTEN)
    cube("pouring_ladle", (142, 12, -94), (6, 3, 4), BRICK)
    cube("switch_valve", (130.6, 6, -88), (1, 1, 1), VALVE_RED)
    zone("trigger_pour_hint", (142, 2, -82), (10, 4, 6))
    zone("checkpoint_pour", (142, 2, -79.5), (10, 4, 3))
    for position in ((136, 0, -118), (140, 0, -116), (146, 0, -118), (150, 0, -116)):
        empty("pour", position)
    for z in (-88, -100, -114):
        lamp((142, 13, z), SODIUM, 1500.0, 20.0)
    for z in (-98, -108):
        lamp((138, 4, z), ORANGE_GLOW, 700.0, 12.0)
        lamp((146, 4, z), ORANGE_GLOW, 700.0, 12.0)


def crossing():
    room(
        "crossing",
        126,
        158,
        -122,
        -154,
        0,
        24,
        SOOT,
        holes={"south": [(138, 146, 0, 6)], "north": [(138, 146, 0, 6)], "floor": [(126, 158, -146, -134)]},
        base=-10.0,
    )
    slab("crossing_bed", 126, 158, -10.5, -10, -146, -134, SLAG)
    slab("hazard_canyon", 126, 158, -10, -8, -146, -134, MOLTEN)
    wall("crossing_canyon_south", "x", -133.5, 126, 158, -10, 0, SOOT)
    wall("crossing_canyon_north", "x", -146.5, 126, 158, -10, 0, SOOT)
    slab("crossing_bridge_main", 138, 142, -0.3, 0, -146, -134, STEEL)
    rail("crossing_bridge_rail", "z", 138, -146, -134, 0, STEEL)
    rail("crossing_bridge_rail", "z", 142, -146, -134, 0, STEEL)
    cube("breakable_bridge", (151.5, 0, -140), (3, 0.4, 12), STEEL)

    zone("trigger_crossing", (140, 1, -140), (5, 4, 4))
    zone("checkpoint_crossing", (140, 2, -128), (8, 4, 4))
    for position in ((132, 0, -130), (136, 0, -130), (150, 0, -128), (154, 0, -128)):
        empty("bridge", position)
    for z in (-136, -144):
        lamp((134, -6, z), EMERGENCY, 700.0, 10.0)
        lamp((148, -6, z), EMERGENCY, 700.0, 10.0)
    lamp((140, 18, -128), SODIUM, 4000.0, 26.0)
    lamp((140, 18, -148), SODIUM, 4000.0, 26.0)


def tank():
    round_room("tank", 142, -174, 16, 0, 24, DARK_CONCRETE, holes={0: (11, 15), 8: (0, 6)})
    room("tank_link", 138, 146, -154, -158, 0, 6, DARK_CONCRETE, holes={"south": [(138, 146, 0, 6)], "north": [(138, 146, 0, 6)]})
    stairs("tank_stairs", "z", 135, -160, -188, 0, 48, 0.25, 3.0, STEEL)
    slab("tank_top", 133, 147, 11.5, 12, -190, -186, STEEL)
    cube("hazard_steam", (136, 1.25, -168), (6, 2.5, 6), MOLTEN)
    cube("hazard_steam", (150, 1.25, -180), (6, 2.5, 6), MOLTEN)
    zone("checkpoint_tank", (142, 2, -160), (8, 4, 4))
    zone("trigger_tank", (142, 2, -165), (8, 4, 6))
    for position in ((130, 0.0, -166), (154, 0.0, -170), (128, 0.0, -180)):
        empty("enemy", position)
    for y in (6, 12, 18):
        lamp((142, y, -174), FLUORESCENT, 1500.0, 18.0)


def crane():
    room(
        "crane",
        130,
        154,
        -190,
        -240,
        0,
        40,
        SOOT,
        holes={"south": [(138, 146, 11, 15)]},
    )
    slab("crane_landing", 143, 149, 11.5, 12, -197, -189.5, STEEL)
    zone("checkpoint_crane", (146, 13, -194), (4, 4, 4))
    slab("crane_platform_1", 143, 149, 14.0, 14.5, -207, -201, STEEL)
    cube("pad_crane", (143.5, 15, -206.5), (2, 1, 2), CONVEYOR)
    slab("crane_belt_platform", 133, 139, 16.5, 17, -215, -199, STEEL)
    cube("belt_crane", (136, 17.25, -207), (6, 0.5, 16), CONVEYOR)
    slab("crane_platform_3", 134, 140, 19.0, 19.5, -225, -219, STEEL)
    slab("crane_platform_4", 144, 150, 21.5, 22, -225, -219, STEEL)
    slab("crane_top", 132, 152, 23.7, 24, -240, -228, STEEL)
    rail("crane_top_rail", "z", 132, -240, -228, 24, STEEL)
    rail("crane_top_rail", "z", 152, -240, -228, 24, STEEL)
    rail("crane_top_rail", "x", -228, 132, 152, 24, STEEL)
    cube("tube_crane", (136, 1.5, -230), (2, 3, 2), PIPE)
    empty("tube_crane_exit", (146, 12, -194), (1, 0, 0))
    room(
        "crane_exit",
        136,
        144,
        -239,
        -233,
        24,
        5,
        STEEL,
        holes={"south": [(136, 144, 24, 28)], "north": [(-100, 100, 24, 29)]},
        floor=False,
    )
    zone("exit", (140, 25.5, -236), (4, 3, 4))
    cube("crane_exit_pipe", (140, 27, -240), (2, 2, 2), PIPE)
    zone("arena_crane_top_trigger", (140, 25, -229.5), (12, 4, 5))
    zone("arena_crane_top_door", (140, 25, -233), (8, 4, 0.6))
    for position in ((136, 24, -238), (146, 24, -236), (140, 24, -231), (134, 24, -239), (148, 24, -239)):
        empty("arena_crane_top_wave_1", position)
    for position in ((136, 24, -234), (146, 24, -239), (140, 24, -236), (134, 24, -234), (148, 24, -233)):
        empty("arena_crane_top_wave_2", position)
    zone("trigger_crane_top", (140, 25, -230), (10, 4, 3))
    for index, (x, y, z) in enumerate(((146, 13, -198), (146, 16, -206), (137, 18, -212), (140, 21, -224), (142, 8, -196))):
        lamp((x, min(39, y), z), SODIUM, 1500.0, 18.0)
    lamp((140, 28, -237), FLUORESCENT, 1500.0, 12.0)

def check_overlaps():
    skipped = ("arena_", "trigger_", "checkpoint_", "exit", "tube_")
    boxes = []
    for obj in bpy.data.objects:
        if obj.type != "MESH" or obj.name.startswith(skipped):
            continue
        corners = [obj.matrix_world @ Vector(corner) for corner in obj.bound_box]
        xs = [point.x for point in corners]
        zs = [point.z for point in corners]
        ys = [point.y for point in corners]
        boxes.append((obj.name, (min(xs), min(zs), min(ys)), (max(xs), max(zs), max(ys))))
    volume = []
    flat = []
    for first in range(len(boxes)):
        name_a, low_a, high_a = boxes[first]
        height_a = high_a[2] - low_a[2]
        for second in range(first + 1, len(boxes)):
            name_b, low_b, high_b = boxes[second]
            height_b = high_b[2] - low_b[2]
            overlap = [
                min(high_a[axis], high_b[axis]) - max(low_a[axis], low_b[axis])
                for axis in range(3)
            ]
            if all(value > 1e-3 for value in overlap):
                volume.append((name_a, name_b, round(min(overlap), 3)))
            flat_pair = (
                overlap[0] > 1e-3
                and overlap[1] > 1e-3
                and abs(high_a[2] - high_b[2]) < 1e-3
                and height_a < 1.3
                and height_b < 1.3
            )
            if flat_pair:
                flat.append((name_a, name_b, round(overlap[0] * overlap[1], 3)))
    print(f"OVERLAPS volume={len(volume)} flat={len(flat)}")
    for entry in volume:
        print("VOLUME", entry)
    for entry in flat:
        print("FLAT", entry)

def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    reception()
    sorting()
    catwalks()
    tunnels()
    foreman()
    shaft()
    compressor()
    molds()
    presses()
    furnace()
    casting()
    pouring()
    crossing()
    tank()
    crane()
    check_overlaps()
    os.makedirs(os.path.dirname(BLEND_PATH), exist_ok=True)
    os.makedirs(os.path.dirname(GLB_PATH), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=BLEND_PATH)
    bpy.ops.export_scene.gltf(
        filepath=GLB_PATH,
        export_format="GLB",
        use_selection=False,
        export_yup=True,
        export_apply=False,
        export_cameras=False,
        export_lights=True,
    )


main()
