import os

import bpy
from mathutils import Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BLEND_PATH = os.path.join(ROOT, "art", "levels", "cooling.blend")
GLB_PATH = os.path.join(ROOT, "assets", "levels", "cooling.glb")
KIT_PATH = os.path.join(ROOT, "art", "decor", "decor_kit.blend")

TINT = (0.9, 0.9, 0.9)
FLOOR_T = 1.0
WALL_T = 1.0

COOL_BLUE = (0.35, 0.55, 1.0)
WARM_SODIUM = (1.0, 0.62, 0.28)
WHITE = (0.85, 0.9, 1.0)
ICE_BLUE = (0.55, 0.8, 1.0)

materials = {}
kit_objects = {}


def material(name, color=TINT):
    if name not in materials:
        made = bpy.data.materials.new(name)
        made.use_nodes = True
        made.diffuse_color = (*color, 1.0)
        made.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (*color, 1.0)
        materials[name] = made
    return materials[name]


def to_blender(position):
    x, up, depth = position
    return Vector((x, -depth, up))


def solid(name, center, size, mat):
    x, up, depth = center
    sx, sy, sz = size
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(x, -depth, up))
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = (sx, sz, sy)
    bpy.ops.object.transform_apply(scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material(mat))
    return obj


def wall_x(name, x0, x1, z, y0, y1, mat):
    return solid(name, ((x0 + x1) / 2.0, (y0 + y1) / 2.0, z),
                 (x1 - x0, y1 - y0, WALL_T), mat)


def wall_z(name, z0, z1, x, y0, y1, mat):
    return solid(name, (x, (y0 + y1) / 2.0, (z0 + z1) / 2.0),
                 (WALL_T, y1 - y0, z1 - z0), mat)


def room(name, x0, z0, x1, z1, floor_y, height, floor_mat, wall_mat, ceil_mat=None, openings=()):
    """openings: (side, center, width, door_height); side n=z0 s=z1 w=x0 e=x1;
    center — координата середины проёма вдоль стены."""
    ceil_mat = ceil_mat or wall_mat
    solid(f"{name}_floor", ((x0 + x1) / 2.0, floor_y - FLOOR_T / 2.0, (z0 + z1) / 2.0),
          (x1 - x0, FLOOR_T, z1 - z0), floor_mat)
    solid(f"{name}_ceil", ((x0 + x1) / 2.0, floor_y + height + FLOOR_T / 2.0, (z0 + z1) / 2.0),
          (x1 - x0, FLOOR_T, z1 - z0), ceil_mat)
    for side, axis0, axis1, fixed in (
        ("n", x0, x1, z0),
        ("s", x0, x1, z1),
        ("w", z0 + WALL_T / 2.0, z1 - WALL_T / 2.0, x0),
        ("e", z0 + WALL_T / 2.0, z1 - WALL_T / 2.0, x1),
    ):
        spans = [(axis0, axis1, floor_y + height, False)]
        for oside, ocenter, owidth, oheight in openings:
            if oside != side:
                continue
            spans = split_span(spans, ocenter - owidth / 2.0, ocenter + owidth / 2.0,
                               floor_y + oheight)
        for index, (span0, span1, top, cut) in enumerate(spans):
            if span1 - span0 < 0.05:
                continue
            if cut and top >= floor_y + height - 0.05:
                continue
            if top >= floor_y + height - 0.05:
                full(name, side, index, span0, span1, fixed, floor_y, floor_y + height, wall_mat)
            else:
                full(name, side, index, span0, span1, fixed, top, floor_y + height, wall_mat)


def full(name, side, index, span0, span1, fixed, y0, y1, mat):
    if side in ("n", "s"):
        wall_x(f"{name}_{side}{index}", span0, span1, fixed, y0, y1, mat)
    else:
        wall_z(f"{name}_{side}{index}", span0, span1, fixed, y0, y1, mat)


def split_span(spans, cut0, cut1, lintel_bottom):
    result = []
    for span0, span1, top, cut in spans:
        if cut1 <= span0 or cut0 >= span1:
            result.append((span0, span1, top, cut))
            continue
        if cut0 > span0:
            result.append((span0, cut0, top, cut))
        result.append((cut0, cut1, min(top, lintel_bottom), True))
        if cut1 < span1:
            result.append((cut1, span1, top, cut))
    return result


def corridor(name, ax, az, bx, bz, floor_y, width, height, floor_mat, wall_mat):
    length_x = abs(bx - ax)
    length_z = abs(bz - az)
    if length_x >= length_z:
        x0, x1 = min(ax, bx), max(ax, bx)
        zc = (az + bz) / 2.0
        solid(f"{name}_floor", ((x0 + x1) / 2.0, floor_y - FLOOR_T / 2.0, zc),
              (x1 - x0, FLOOR_T, width), floor_mat)
        solid(f"{name}_ceil", ((x0 + x1) / 2.0, floor_y + height + FLOOR_T / 2.0, zc),
              (x1 - x0 - WALL_T, FLOOR_T, width), wall_mat)
        wall_z(f"{name}_north", x0 + WALL_T / 2.0, x1 - WALL_T / 2.0, zc - width / 2.0,
               floor_y, floor_y + height, wall_mat)
        wall_z(f"{name}_south", x0 + WALL_T / 2.0, x1 - WALL_T / 2.0, zc + width / 2.0,
               floor_y, floor_y + height, wall_mat)
    else:
        z0, z1 = min(az, bz), max(az, bz)
        xc = (ax + bx) / 2.0
        solid(f"{name}_floor", (xc, floor_y - FLOOR_T / 2.0, (z0 + z1) / 2.0),
              (width, FLOOR_T, z1 - z0), floor_mat)
        solid(f"{name}_ceil", (xc, floor_y + height + FLOOR_T / 2.0, (z0 + z1) / 2.0),
              (width, FLOOR_T, z1 - z0 - WALL_T), wall_mat)
        wall_x(f"{name}_west", z0 + WALL_T / 2.0, z1 - WALL_T / 2.0, xc - width / 2.0,
               floor_y, floor_y + height, wall_mat)
        wall_x(f"{name}_east", z0 + WALL_T / 2.0, z1 - WALL_T / 2.0, xc + width / 2.0,
               floor_y, floor_y + height, wall_mat)


def lamp(name, position, color, power):
    light_data = bpy.data.lights.new(name, "POINT")
    light_data.color = color
    light_data.energy = power
    light_data.shadow_soft_size = 0.5
    light_object = bpy.data.objects.new(name, light_data)
    bpy.context.collection.objects.link(light_object)
    light_object.location = to_blender(position)


def deco(kind, name, position, rotation=0.0, scale=1.0):
    if kind not in kit_objects:
        bpy.ops.wm.append(filename=kind, directory=os.path.join(KIT_PATH, "Object"))
        kit_objects[kind] = bpy.data.objects[kind]
        kit_objects[kind].hide_render = True
    source = kit_objects[kind]
    copy = source.copy()
    copy.data = source.data.copy()
    copy.hide_render = False
    copy.name = f"deco_{name}"
    copy.location = to_blender(position)
    copy.rotation_euler = (0.0, 0.0, -rotation)
    copy.scale = (scale, scale, scale)
    bpy.context.collection.objects.link(copy)
    return copy


def marker(name, position, rotation=0.0):
    empty = bpy.data.objects.new(name, None)
    empty.empty_display_type = "SINGLE_ARROW"
    bpy.context.collection.objects.link(empty)
    empty.location = to_blender(position)
    empty.rotation_euler = (0.0, 0.0, -rotation)
    return empty


def zone(name, center, size, mat="warning", color=(0.9, 0.1, 0.9)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=to_blender(center))
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = (size[0], size[2], size[1])
    bpy.ops.object.transform_apply(scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material(mat, color))


def water(name, center, size):
    solid(name, center, size, "water")


def act_one():
    room("arrival", -3, -3, 3, 3, 0.0, 4.0, "concrete_dark", "concrete_dark",
         openings=(("e", 0.0, 3.0, 3.0),))
    marker("player_start", (-1.5, 0.0, 0.0), 0.0)
    zone("tube_arrival_visual", (0.0, 3.6, 0.0), (2.0, 1.0, 2.0), "metal_brushed", (0.3, 0.4, 0.5))
    deco("icicle", "arrival_1", (-2.2, 3.4, -2.2))
    deco("icicle", "arrival_2", (2.0, 3.4, -1.8))
    lamp("lamp_arrival", (0.0, 3.4, 0.0), COOL_BLUE, 300)

    corridor("cond", 3.0, 0.0, 16.0, 0.0, 0.0, 3.0, 3.0, "tile_dirty", "tile_dirty")
    zone("hazard_steam_a", (6.0, 0.4, -0.9), (1.2, 0.8, 1.2))
    zone("hazard_steam_b", (12.0, 0.4, 0.9), (1.2, 0.8, 1.2))
    deco("pipe", "cond_1", (8.0, 2.8, -1.1), 1.5708)
    deco("icicle", "cond_2", (4.0, 2.8, 1.0))
    lamp("lamp_cond", (8.0, 2.6, 0.0), COOL_BLUE, 120)

    room("pump", 16, -4, 34, 4, 0.0, 8.0, "tile_blue", "concrete",
         openings=(("w", 0.0, 3.0, 3.0), ("n", 25.0, 3.0, 3.0), ("e", 0.0, 3.0, 3.0)))
    for index in range(3):
        deco("column", f"pump_col_{index}", (20.0 + index * 4.0, 0.0, -1.0 + index * 2.0))
    deco("barrel", "pump_barrel_1", (31.0, 0.0, 2.0))
    deco("barrel", "pump_barrel_2", (30.0, 0.0, 2.6))
    deco("crate", "pump_crate", (17.5, 0.0, -2.5))
    marker("enemy", (26.0, 0.0, 0.0))
    marker("enemy", (30.0, 0.0, -2.0))
    lamp("lamp_pump_a", (21.0, 6.5, 0.0), COOL_BLUE, 900)
    lamp("lamp_pump_b", (29.0, 6.5, 0.0), WARM_SODIUM, 700)

    corridor("ring_a", 25.0, -4.0, 25.0, -25.0, 0.0, 3.0, 3.0, "tile_dirty", "tile_dirty")
    lamp("lamp_ring_a", (25.0, 2.6, -16.0), COOL_BLUE, 150)
    deco("pipe", "ring_a_pipe", (25.0, 2.7, -20.0), 0.0)

    room("j1", 22, -31, 28, -25, 0.0, 3.0, "concrete_dark", "concrete_dark",
         openings=(("s", 25.0, 3.0, 3.0), ("e", -28.0, 2.0, 2.0)))
    deco("rubble", "j1_rubble", (23.5, 0.0, -29.5))
    lamp("lamp_j1", (25.0, 2.6, -28.0), COOL_BLUE, 120)

    corridor("ring_b", 28.0, -28.0, 45.0, -28.0, 0.0, 2.0, 2.0, "concrete_dark", "concrete_dark")
    lamp("lamp_ring_b", (36.0, 1.8, -28.0), COOL_BLUE, 100)

    room("viewpoint", 45, -31, 51, -25, 0.0, 5.0, "concrete_dark", "concrete_dark",
         openings=(("w", -28.0, 2.0, 2.0), ("e", -28.0, 2.0, 2.0)))
    water("view_pool", (48.0, -0.75, -28.0), (4.0, 0.8, 3.8))
    deco("rock", "view_rock", (46.2, 0.0, -29.6))
    lamp("lamp_view", (48.0, 4.0, -28.0), ICE_BLUE, 400)

    corridor("bridge", 51.0, -28.0, 61.0, -28.0, 0.0, 2.0, 2.0, "floor_plate", "concrete_dark")
    water("bridge_pool_n", (56.0, -0.7, -30.4), (7.0, 1.0, 2.6))
    water("bridge_pool_s", (56.0, -0.7, -25.6), (7.0, 1.0, 2.6))
    lamp("lamp_bridge", (56.0, 1.8, -28.0), WARM_SODIUM, 300)

    room("vent", 61, -31, 69, -25, 0.0, 5.0, "metal_brushed", "concrete_dark",
         openings=(("w", -28.0, 2.0, 2.0), ("e", -28.0, 3.0, 4.0)))
    zone("switch_vent", (68.0, 1.6, -28.0), (0.8, 0.8, 0.8), "metal_panel", (0.9, 0.5, 0.1))
    deco("pipe", "vent_pipe", (64.0, 4.4, -30.2))
    lamp("lamp_vent", (65.0, 4.0, -28.0), WARM_SODIUM, 400)

    corridor("gate", 69.0, -28.0, 75.0, -28.0, 0.0, 4.0, 3.0, "concrete_dark", "concrete_dark")
    zone("door_gate", (75.0, 2.0, -28.0), (1.0, 4.0, 3.0), "metal_panel", (0.5, 0.1, 0.05))

    room("arena_settling", 75, -36, 97, -20, 0.0, 10.0, "tile_blue", "concrete",
         openings=(("w", -28.0, 3.0, 4.0), ("e", -28.0, 4.0, 4.0)))
    zone("arena_settling_trigger", (79.0, 1.0, -28.0), (2.0, 2.0, 2.0))
    for index in range(3):
        solid(f"settling_column_{index}", (82.0 + index * 4.0, 5.0, -33.0), (2.0, 10.0, 2.0),
              "concrete_dark")
        solid(f"settling_column_b_{index}", (82.0 + index * 4.0, 5.0, -23.0), (2.0, 10.0, 2.0),
              "concrete_dark")
    zone("arena_settling_exit", (97.0, 2.0, -28.0), (1.0, 4.0, 4.0), "metal_panel", (0.5, 0.1, 0.05))
    zone("arena_settling_machine", (93.0, 1.5, -28.0), (4.0, 3.0, 4.0), "rust", (0.6, 0.4, 0.2))
    for wave in (1, 2):
        for index in range(4):
            marker(f"arena_settling_wave_{wave}",
                   (78.0 + index * 5.0, 0.0, -24.0 - (index % 2) * 8.0))
    lamp("lamp_settling_a", (81.0, 8.5, -28.0), COOL_BLUE, 1200)
    lamp("lamp_settling_b", (92.0, 8.5, -28.0), WARM_SODIUM, 900)

    corridor("after_settling", 97.0, -28.0, 103.0, -28.0, 0.0, 3.0, 3.0, "concrete_dark",
             "concrete_dark")
    room("j2", 103, -31, 109, -25, 0.0, 3.0, "concrete_dark", "concrete_dark",
         openings=(("w", -28.0, 3.0, 3.0), ("n", 106.0, 3.0, 3.0)))
    zone("checkpoint_junction", (106.0, 1.0, -28.0), (2.0, 2.0, 2.0))
    lamp("lamp_j2", (106.0, 2.6, -28.0), COOL_BLUE, 200)

    corridor("north", 106.0, -31.0, 106.0, -44.0, 0.0, 3.0, 3.0, "tile_dirty", "tile_dirty")
    lamp("lamp_north", (106.0, 2.6, -39.0), COOL_BLUE, 150)

    room("lab", 100, -50, 112, -44, 0.0, 4.0, "lab_tile", "lab_panel",
         openings=(("s", 106.0, 3.0, 3.0), ("w", -47.0, 3.0, 3.0)))
    zone("key_cooling", (102.5, 1.2, -47.0), (0.7, 0.7, 0.7), "lab_stripe", (1.0, 0.85, 0.2))
    solid("clock_stuck", (111.2, 2.6, -47.0), (0.3, 0.8, 0.8), "warning")
    zone("trigger_clock", (109.0, 1.0, -47.0), (2.0, 2.0, 2.0))
    deco("statue", "lab_statue", (106.0, 0.0, -45.5), 0.3)
    lamp("lamp_lab", (106.0, 3.4, -47.0), WHITE, 700)


def act_two():
    corridor("flooded", 85.0, -47.0, 100.0, -47.0, 0.0, 3.0, 3.0, "tile_dirty", "tile_dirty")
    water("flooded_side_a", (92.0, -0.5, -49.2), (14.0, 0.7, 1.4))
    water("flooded_side_b", (92.0, -0.5, -44.8), (14.0, 0.7, 1.4))
    zone("hazard_steam_c", (95.0, 0.4, -47.0), (1.2, 0.8, 1.2))
    lamp("lamp_flooded", (92.0, 2.6, -47.0), COOL_BLUE, 200)

    room("tower", 75, -50, 85, -40, 0.0, 16.0, "concrete_dark", "concrete_dark",
         openings=(("e", -47.0, 3.0, 3.0), ("n", 80.0, 10.0, 16.0)))
    wall_x("tower_n_west", 75.0, 78.0, -50.0, 0.0, 16.0, "concrete_dark")
    wall_x("tower_n_east", 82.0, 85.0, -50.0, 0.0, 16.0, "concrete_dark")
    wall_x("tower_n_below", 78.0, 82.0, -50.0, 0.0, 13.0, "concrete_dark")
    for index, (x, z, y, power) in enumerate((
        (80.0, -44.0, 0.0, 25.0),
        (78.0, -47.5, 4.0, 27.0),
        (82.0, -48.5, 8.0, 28.0),
    )):
        zone(f"pad_tower_{index}", (x, y + 0.1, z), (3.0, 0.4, 3.0), "metal_brushed", (0.2, 0.5, 0.6))
    solid("tower_landing", (80.0, 12.5, -47.4), (8.0, 1.0, 4.0), "floor_plate")
    zone("checkpoint_tower", (80.0, 14.0, -48.0), (2.0, 2.0, 2.0))
    lamp("lamp_tower", (80.0, 11.0, -45.0), ICE_BLUE, 500)
    deco("pipe", "tower_pipe", (76.0, 14.5, -41.0))

    corridor("tower_link", 80.0, -50.5, 80.0, -55.0, 13.0, 4.0, 4.0, "floor_plate",
             "concrete_dark")
    lamp("lamp_tower_link", (80.0, 16.6, -60.0), ICE_BLUE, 200)

    room("reservoir", 60, -75, 84, -55, 13.0, 10.0, "floor_plate", "concrete",
         openings=(("w", -65.0, 4.0, 4.0), ("s", 72.0, 24.0, 10.0)))
    wall_x("reservoir_s_west", 60.0, 78.0, -55.0, 13.0, 23.0, "concrete")
    wall_x("reservoir_s_east", 82.0, 84.0, -55.0, 13.0, 23.0, "concrete")
    wall_x("reservoir_s_above", 78.0, 82.0, -55.0, 17.0, 23.0, "concrete")
    solid("reservoir_spine", (72.0, 18.0, -65.0), (24.0, 1.0, 3.0), "floor_plate")
    solid("reservoir_cross_a", (63.0, 18.0, -68.75), (3.0, 1.0, 4.5), "floor_plate")
    solid("reservoir_cross_b", (81.0, 18.0, -61.25), (3.0, 1.0, 4.5), "floor_plate")
    water("reservoir_pool", (72.0, 12.8, -65.0), (22.0, 0.8, 16.0))
    zone("arena_reservoir_trigger", (78.0, 14.0, -65.0), (2.0, 2.0, 2.0))
    zone("arena_reservoir_exit", (60.0, 15.0, -65.0), (1.0, 4.0, 4.0), "metal_panel", (0.5, 0.1, 0.05))
    zone("arena_reservoir_machine", (77.0, 14.5, -71.0), (4.0, 3.0, 4.0), "rust", (0.6, 0.4, 0.2))
    for wave in (1, 2, 3):
        for index in range(4):
            marker(f"arena_reservoir_wave_{wave}",
                   (64.0 + index * 5.0, 13.5, -60.0 - (index % 2) * 10.0))
    lamp("lamp_reservoir_a", (67.0, 21.5, -65.0), ICE_BLUE, 1100)
    lamp("lamp_reservoir_b", (77.0, 21.5, -65.0), WARM_SODIUM, 900)

    corridor("after_reservoir", 55.0, -65.0, 60.0, -65.0, 13.0, 3.0, 3.0, "concrete_dark",
             "concrete_dark")

    room("filters", 44, -70, 55, -60, 13.0, 7.0, "metal_brushed", "concrete_dark",
         openings=(("e", -65.0, 3.0, 3.0), ("w", -65.0, 3.0, 3.0)))
    for index in range(3):
        zone(f"crusher_filter_{index}", (47.0 + index * 3.0, 18.5, -65.0), (2.5, 3.0, 2.5),
             "metal_panel", (0.4, 0.42, 0.45))
    deco("pipe", "filters_pipe", (53.0, 19.2, -68.5))
    marker("enemy", (46.0, 13.0, -63.0))
    marker("enemy", (52.0, 13.0, -67.0))
    lamp("lamp_filters", (49.5, 18.5, -65.0), WHITE, 900)

    corridor("after_filters", 38.0, -65.0, 44.0, -65.0, 13.0, 3.0, 3.0, "concrete_dark",
             "concrete_dark")

    room("storage", 32, -70, 38, -60, 13.0, 6.0, "concrete_dark", "concrete_dark",
         openings=(("e", -65.0, 3.0, 3.0), ("w", -65.0, 3.0, 3.0)))
    for index in range(4):
        deco("crate", f"storage_crate_{index}",
             (33.5 + (index % 2) * 2.0, 13.0, -68.0 + (index // 2) * 3.0))
    deco("barrel", "storage_barrel_1", (36.5, 13.0, -62.0))
    deco("pallet", "storage_pallet", (33.5, 13.0, -62.0))
    zone("breakable_storage", (35.0, 14.4, -65.0), (5.0, 0.6, 2.5), "floor_plate", (0.5, 0.4, 0.3))
    zone("trigger_break_storage", (36.5, 14.0, -65.0), (1.5, 2.0, 2.5))
    lamp("lamp_storage", (35.0, 17.5, -65.0), WARM_SODIUM, 600)

    corridor("after_storage", 26.0, -65.0, 32.0, -65.0, 13.0, 3.0, 3.0, "concrete_dark",
             "concrete_dark")


def act_three():
    room("cryo", 20, -70, 26, -60, 13.0, 6.0, "tile_blue", "concrete_blue",
         openings=(("e", -65.0, 3.0, 3.0), ("w", -65.0, 3.0, 3.0)))
    for index in range(3):
        solid(f"cryo_cell_{index}", (22.0, 15.0, -68.5 + index * 2.8), (3.0, 4.0, 2.0), "ice")
    zone("trigger_cryo_ambush", (22.5, 14.0, -65.0), (2.0, 2.0, 2.0))
    marker("cryo_ambush", (24.0, 13.0, -65.0))
    marker("cryo_ambush", (24.0, 13.0, -63.0))
    marker("cryo_ambush", (24.0, 13.0, -67.0))
    lamp("lamp_cryo", (23.0, 17.5, -65.0), ICE_BLUE, 800)

    corridor("ice_link", 14.0, -65.0, 20.0, -65.0, 13.0, 3.0, 3.0, "ice", "concrete_blue")

    room("ice_pocket", 6, -70, 14, -60, 13.0, 4.0, "ice", "concrete_blue",
         openings=(("e", -65.0, 3.0, 3.0), ("w", -65.0, 2.0, 2.0)))
    zone("checkpoint_ice", (10.0, 14.0, -65.0), (2.0, 2.0, 2.0))
    for index in range(4):
        deco("icicle", f"ice_icicle_{index}", (7.5 + index * 2.0, 16.6, -68.0 + (index % 2) * 5.0))
    deco("rock", "ice_rock", (12.0, 13.0, -62.5), 0.0, 1.3)
    lamp("lamp_ice", (10.0, 16.4, -65.0), ICE_BLUE, 250)

    corridor("span_a", 0.0, -65.0, 6.0, -65.0, 13.0, 2.0, 2.0, "floor_plate", "concrete_dark")
    corridor("span_b", -13.0, -65.0, -7.0, -65.0, 13.0, 2.0, 2.0, "floor_plate", "concrete_dark")
    zone("breakable_span", (-3.5, 12.7, -65.0), (7.0, 0.6, 2.0), "floor_plate", (0.5, 0.4, 0.3))
    lamp("lamp_span", (-3.5, 14.8, -65.0), WARM_SODIUM, 200)

    room("towers_final", -45, -75, -13, -55, 13.0, 12.0, "floor_plate", "concrete",
         openings=(("e", -65.0, 2.0, 2.0), ("n", -29.0, 4.0, 4.0)))
    solid("towers_ring_w", (-31.0, 20.0, -71.0), (30.0, 1.0, 3.0), "floor_plate")
    solid("towers_ring_n", (-42.0, 20.0, -60.0), (3.0, 1.0, 19.0), "floor_plate")
    solid("towers_ring_s", (-20.0, 20.0, -60.0), (3.0, 1.0, 19.0), "floor_plate")
    zone("pad_towers", (-16.5, 14.0, -65.0), (3.0, 0.4, 3.0), "metal_brushed", (0.2, 0.5, 0.6))
    zone("arena_towers_trigger", (-18.0, 14.0, -65.0), (2.0, 2.0, 2.0))
    zone("arena_towers_exit", (-29.0, 15.0, -75.0), (4.0, 4.0, 1.0), "metal_panel", (0.5, 0.1, 0.05))
    zone("arena_towers_machine", (-40.0, 14.5, -70.0), (4.0, 3.0, 4.0), "rust", (0.6, 0.4, 0.2))
    for wave in (1, 2, 3):
        for index in range(4):
            marker(f"arena_towers_wave_{wave}",
                   (-38.0 + index * 6.0, 13.5, -60.0 - (index % 2) * 10.0))
    for index in range(3):
        deco("column", f"towers_col_{index}", (-26.0 + index * 6.0, 13.0, -65.0))
    deco("statue", "towers_statue", (-40.0, 13.0, -60.0), 0.9)
    lamp("lamp_towers_a", (-24.0, 23.5, -65.0), ICE_BLUE, 1200)
    lamp("lamp_towers_b", (-38.0, 23.5, -65.0), WARM_SODIUM, 1000)

    corridor("to_keylock", -29.0, -75.0, -29.0, -81.0, 13.0, 3.0, 3.0, "metal_brushed",
             "concrete_dark")

    room("keylock", -32, -84, -26, -81, 13.0, 4.0, "metal_brushed", "concrete_dark",
         openings=(("s", -29.0, 3.0, 3.0), ("n", -29.0, 3.0, 3.0)))
    zone("door_keylock", (-29.0, 14.5, -84.0), (3.0, 3.0, 1.0), "metal_panel", (0.6, 0.5, 0.1))
    zone("trigger_keylock", (-29.0, 14.0, -82.5), (2.0, 2.0, 2.0))
    lamp("lamp_keylock", (-29.0, 16.0, -82.5), WHITE, 300)

    corridor("to_exit", -29.0, -84.0, -29.0, -90.0, 13.0, 3.0, 3.0, "concrete_dark",
             "concrete_dark")

    room("exit_room", -32, -93, -26, -90, 13.0, 4.0, "concrete_dark", "concrete_dark",
         openings=(("s", -29.0, 3.0, 3.0),))
    zone("exit", (-29.0, 14.5, -91.5), (2.0, 2.0, 2.0))
    zone("checkpoint_exit", (-29.0, 14.0, -92.5), (2.0, 2.0, 2.0))
    deco("icicle", "exit_icicle", (-27.0, 16.6, -92.0))
    lamp("lamp_exit", (-29.0, 16.0, -91.5), COOL_BLUE, 300)
def check_player_path():
    for required in ("player_start", "exit"):
        assert required in bpy.data.objects, f"missing {required}"


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    act_one()
    act_two()
    act_three()
    check_player_path()
    for obj in list(kit_objects.values()):
        bpy.data.objects.remove(obj, do_unlink=True)
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
