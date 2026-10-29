import math
import os

import bpy

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BLEND_PATH = os.path.join(ROOT, "art", "levels", "utilization.blend")
GLB_PATH = os.path.join(ROOT, "assets", "levels", "utilization.glb")

CONCRETE = (0.42, 0.40, 0.38)
DARK_CONCRETE = (0.30, 0.28, 0.27)
RUST = (0.55, 0.32, 0.18)
HAZARD = (0.85, 0.65, 0.10)
PIPE = (0.35, 0.40, 0.42)
CONVEYOR = (0.18, 0.18, 0.20)
PRESS = (0.50, 0.22, 0.16)
MACHINE = (0.55, 0.35, 0.18)
SODIUM = (1.0, 0.62, 0.28)
FLUORESCENT = (0.75, 0.88, 1.0)
EMERGENCY = (1.0, 0.12, 0.08)
TEAL_GLOW = (0.3, 1.0, 0.9)
TUBE = (0.20, 0.55, 0.55)
ZONE = (0.9, 0.1, 0.9)

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
    linear = tuple(channel ** 2.2 for channel in color)
    result = bpy.data.materials.new(name=f"color_{len(materials)}")
    result.diffuse_color = (*linear, 1.0)
    if result.node_tree is None:
        result.use_nodes = True
    principled = result.node_tree.nodes.get("Principled BSDF")
    principled.inputs["Base Color"].default_value = (*linear, 1.0)
    materials[color] = result
    return result


def cube(name, center, size, color, rotation_x_degrees=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=to_blender(center))
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = size_to_blender(size)
    obj.rotation_euler = (math.radians(rotation_x_degrees), 0.0, 0.0)
    obj.data.materials.append(material(color))
    return obj


def zone(name, center, size):
    obj = cube(name, center, size, ZONE)
    obj.display_type = "WIRE"
    return obj


def lamp(position, color, power, distance):
    light = bpy.data.lights.new(name="lamp", type="POINT")
    light.color = tuple(channel ** 2.2 for channel in color)
    light.energy = power
    light.use_custom_distance = True
    light.cutoff_distance = distance
    obj = bpy.data.objects.new("lamp", light)
    obj.location = to_blender(position)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def empty(name, position):
    obj = bpy.data.objects.new(name, None)
    obj.location = to_blender(position)
    obj.empty_display_type = "SINGLE_ARROW"
    obj.empty_display_size = 1.5
    obj.rotation_euler = (math.radians(-90.0), 0.0, 0.0)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def room_walls(prefix, x_min, x_max, z_max, z_min, height, color, gaps):
    thickness = 1.0
    center_x = (x_min + x_max) / 2
    center_z = (z_max + z_min) / 2
    width = x_max - x_min
    depth = z_max - z_min
    cube(f"{prefix}_wall_west", (x_min - 0.5, height / 2, center_z), (thickness, height, depth + 2), color)
    cube(f"{prefix}_wall_east", (x_max + 0.5, height / 2, center_z), (thickness, height, depth + 2), color)
    for side, z in (("south", z_max + 0.5), ("north", z_min - 0.5)):
        gap = gaps.get(side)
        if gap == "open":
            continue
        if gap is None:
            cube(f"{prefix}_wall_{side}", (center_x, height / 2, z), (width, height, thickness), color)
            continue
        gap_left, gap_right, gap_height = gap
        left_width = gap_left - x_min
        right_width = x_max - gap_right
        if left_width > 0:
            cube(f"{prefix}_wall_{side}", (x_min + left_width / 2, height / 2, z), (left_width, height, thickness), color)
        if right_width > 0:
            cube(f"{prefix}_wall_{side}", (gap_right + right_width / 2, height / 2, z), (right_width, height, thickness), color)
        lintel_height = height - gap_height
        cube(
            f"{prefix}_wall_{side}",
            ((gap_left + gap_right) / 2, gap_height + lintel_height / 2, z),
            (gap_right - gap_left, lintel_height, thickness),
            color,
        )
    cube(f"{prefix}_floor", (center_x, -0.5, center_z), (width + 2, 1.0, depth + 2), DARK_CONCRETE)
    cube(f"{prefix}_ceiling", (center_x, height + 0.5, center_z), (width + 2, 1.0, depth + 2), DARK_CONCRETE)


def disposal_room():
    room_walls("disposal", -10.0, 10.0, 0.0, -24.0, 10.0, CONCRETE, {"north": (-9.5, -4.5, 5.0)})
    cube("conveyor", (0.0, 0.5, -11.0), (3.0, 1.0, 20.0), CONVEYOR)
    cube("conveyor_rail", (-1.75, 0.75, -11.0), (0.5, 1.5, 20.0), HAZARD)
    cube("conveyor_rail", (1.75, 0.75, -11.0), (0.5, 1.5, 20.0), HAZARD)
    cube("press", (0.0, 5.0, -21.5), (7.0, 10.0, 5.0), PRESS)
    cube("workbench", (-7.0, 0.5, -6.0), (3.0, 1.0, 1.5), RUST)
    cube("crate", (7.0, 1.0, -4.0), (2.0, 2.0, 2.0), RUST)
    cube("crate", (7.5, 0.75, -9.0), (1.5, 1.5, 1.5), RUST)
    empty("player_start", (0.0, 1.0, -3.0))
    lamp((-5.0, 7.5, -6.0), SODIUM, 1500.0, 16.0)
    lamp((5.0, 7.5, -6.0), SODIUM, 1500.0, 16.0)
    lamp((-5.0, 7.5, -14.0), SODIUM, 1500.0, 16.0)
    lamp((5.0, 7.5, -14.0), SODIUM, 1500.0, 16.0)
    lamp((6.5, 4.0, -19.0), EMERGENCY, 300.0, 7.0)
    cube("target", (6.0, 4.0, -16.0), (1.0, 1.0, 1.0), HAZARD)


def corridor():
    room_walls("corridor", -9.5, -4.5, -24.0, -60.0, 6.0, CONCRETE, {"south": "open", "north": "open"})
    cube("pipe_low", (-9.0, 1.0, -42.0), (0.8, 0.8, 36.0), PIPE)
    cube("pipe_high", (-9.0, 4.8, -42.0), (0.8, 0.8, 36.0), PIPE)
    cube("pipe_high", (-5.0, 5.2, -42.0), (0.6, 0.6, 36.0), PIPE)
    cube("pipe_crossing", (-7.0, 2.0, -38.0), (5.0, 1.2, 0.8), PIPE)
    cube("pipe_crossing", (-7.0, 4.6, -50.0), (5.0, 0.8, 0.8), PIPE)
    empty("enemy", (-7.0, 0.0, -48.0))
    empty("enemy", (-6.0, 0.0, -53.0))
    for z in (-28.0, -36.0, -44.0, -52.0, -58.0):
        lamp((-7.0, 4.8, z), FLUORESCENT, 500.0, 9.0)


def assembly_hall():
    room_walls("hall", -25.0, 25.0, -60.0, -110.0, 16.0, CONCRETE, {"south": (-9.5, -4.5, 6.0)})
    for x, z in ((-12.0, -78.0), (12.0, -78.0), (-12.0, -94.0), (12.0, -94.0)):
        cube("pillar", (x, 8.0, z), (2.0, 16.0, 2.0), RUST)
    cube("platform", (0.0, 2.5, -86.0), (10.0, 1.0, 6.0), HAZARD)
    cube("mezzanine", (-21.0, 4.0, -85.0), (8.0, 1.0, 30.0), RUST)
    cube("ramp", (-21.0, 2.0, -66.6), (4.0, 0.5, 8.4), RUST, rotation_x_degrees=-28.0)
    cube("hall_conveyor", (0.0, 0.25, -97.0), (3.0, 0.5, 14.0), CONVEYOR)
    cube("pneumatic_tube", (22.0, 3.0, -107.0), (3.0, 6.0, 3.0), TUBE)
    zone("exit", (22.0, 2.0, -104.0), (4.0, 4.0, 3.0))
    for x, z in ((-14.0, -70.0), (14.0, -70.0), (-14.0, -86.0), (14.0, -86.0), (-14.0, -102.0), (14.0, -102.0)):
        lamp((x, 11.5, z), SODIUM, 4000.0, 26.0)
    lamp((0.0, 6.5, -102.0), EMERGENCY, 700.0, 10.0)
    lamp((22.0, 4.0, -104.0), TEAL_GLOW, 800.0, 9.0)

    zone("arena_hall_trigger", (-7.0, 3.0, -67.0), (10.0, 6.0, 6.0))
    zone("arena_hall_door", (-7.0, 3.0, -60.5), (5.0, 6.0, 1.0))
    zone("arena_hall_machine", (0.0, 2.5, -106.5), (6.0, 5.0, 3.0))
    empty("arena_hall_machine_spawn", (0.0, 0.0, -101.0))
    for position in ((-15.0, 0.0, -100.0), (15.0, 0.0, -100.0), (0.0, 0.0, -90.0)):
        empty("arena_hall_wave_1", position)
    for position in ((-18.0, 0.0, -75.0), (18.0, 0.0, -75.0), (-10.0, 0.0, -105.0), (10.0, 0.0, -105.0)):
        empty("arena_hall_wave_2", position)


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    disposal_room()
    corridor()
    assembly_hall()
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
