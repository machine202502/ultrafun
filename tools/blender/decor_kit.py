import os
import random

import bpy
from mathutils import Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BLEND_PATH = os.path.join(ROOT, "art", "decor", "decor_kit.blend")
GLB_PATH = os.path.join(ROOT, "assets", "models", "decor_kit.glb")
SAMPLE_PATH = os.path.join(ROOT, "assets", "models", "decor_sample.glb")

rng = random.Random(1681)

CONCRETE = (0.30, 0.29, 0.27)
GRANITE = (0.22, 0.22, 0.24)
MANNEQUIN = (0.62, 0.55, 0.42)
WOOD = (0.28, 0.17, 0.08)
RUST = (0.35, 0.18, 0.08)
STEEL = (0.28, 0.30, 0.32)
ICE = (0.65, 0.75, 0.85)

materials = {}


def material(name, color):
    if name not in materials:
        made = bpy.data.materials.new(name)
        made.use_nodes = True
        made.diffuse_color = (*color, 1.0)
        made.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (*color, 1.0)
        materials[name] = made
    return materials[name]


def shade_flat(object):
    for face in object.data.polygons:
        face.use_smooth = False


def jitter(object, amount):
    for vertex in object.data.vertices:
        vertex.co += Vector((rng.uniform(-amount, amount) for _ in range(3)))


def place(name, object, material_name):
    object.name = name
    object.data.materials.clear()
    object.data.materials.append(material(material_name, globals()[material_name]))
    shade_flat(object)
    return object


def cube(name, size, location, material_name, jitter_amount=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    object = bpy.context.active_object
    object.scale = size
    bpy.ops.object.transform_apply(scale=True)
    if jitter_amount:
        jitter(object, jitter_amount)
    return place(name, object, material_name)


def prism(name, vertices, radius, depth, location, material_name):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=location)
    return place(name, bpy.context.active_object, material_name)


def joined(name, parts):
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    result = bpy.context.active_object
    result.name = name
    return result


def rock():
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=1, radius=0.55, location=(0.0, 0.0, 0.35))
    object = bpy.context.active_object
    jitter(object, 0.16)
    return place("rock", object, "GRANITE")


def column():
    base = prism("column_base", 8, 0.55, 0.25, (0.0, 0.0, 0.125), "CONCRETE")
    shaft = prism("column_shaft", 8, 0.38, 3.4, (0.0, 0.0, 0.25 + 1.7), "CONCRETE")
    cap = prism("column_cap", 8, 0.55, 0.3, (0.0, 0.0, 3.65 + 0.15), "CONCRETE")
    return joined("column", [base, shaft, cap])


def statue():
    parts = [
        cube("statue_pedestal", (1.4, 1.4, 0.9), (0.0, 0.0, 0.45), "CONCRETE"),
        cube("statue_legs", (0.55, 0.4, 0.8), (0.0, 0.0, 1.3), "MANNEQUIN"),
        cube("statue_torso", (0.75, 0.45, 1.0), (0.0, 0.0, 2.2), "MANNEQUIN"),
        cube("statue_head", (0.4, 0.4, 0.45), (0.0, 0.0, 2.95), "MANNEQUIN"),
        cube("statue_arm_left", (0.22, 0.22, 0.85), (-0.55, 0.0, 2.2), "MANNEQUIN"),
    ]
    raised = cube("statue_arm_right", (0.22, 0.22, 0.9), (0.5, 0.0, 2.55), "MANNEQUIN")
    raised.rotation_euler = (0.0, 0.0, 0.5)
    bpy.ops.object.transform_apply(rotation=True)
    parts.append(raised)
    return joined("statue", parts)


def crate():
    cube("crate", (0.85, 0.85, 0.85), (0.0, 0.0, 0.43), "WOOD", jitter_amount=0.02)
    return bpy.data.objects["crate"]


def barrel():
    prism("barrel", 8, 0.36, 0.95, (0.0, 0.0, 0.48), "RUST")
    return bpy.data.objects["barrel"]


def pipe():
    prism("pipe", 6, 0.16, 4.0, (0.0, 0.0, 2.0), "STEEL")
    return bpy.data.objects["pipe"]


def pallet():
    base = cube("pallet_base", (1.2, 0.8, 0.1), (0.0, 0.0, 0.05), "WOOD")
    slats = cube("pallet_slats", (1.2, 0.8, 0.06), (0.0, 0.0, 0.13), "WOOD")
    return joined("pallet", [base, slats])


def rubble():
    parts = []
    for index in range(5):
        bpy.ops.mesh.primitive_ico_sphere_add(
            subdivisions=0,
            radius=rng.uniform(0.12, 0.28),
            location=(rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5), 0.15),
        )
        object = bpy.context.active_object
        jitter(object, 0.06)
        parts.append(place(f"rubble_{index}", object, "GRANITE"))
    return joined("rubble", parts)


def icicle():
    bpy.ops.mesh.primitive_cone_add(vertices=6, radius1=0.16, radius2=0.0, depth=0.8, location=(0.0, 0.0, 0.4))
    object = bpy.context.active_object
    object.rotation_euler = (3.14159265, 0.0, 0.0)
    return place("icicle", object, "ICE")


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    rock()
    column()
    statue()
    crate()
    barrel()
    pipe()
    pallet()
    rubble()
    icicle()
    os.makedirs(os.path.dirname(BLEND_PATH), exist_ok=True)
    os.makedirs(os.path.dirname(GLB_PATH), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=BLEND_PATH)
    bpy.ops.export_scene.gltf(
        filepath=GLB_PATH,
        export_format="GLB",
        use_selection=False,
        export_yup=True,
        export_apply=True,
        export_cameras=False,
        export_lights=False,
    )
    bpy.ops.wm.read_factory_settings(use_empty=True)
    materials.clear()
    sample = rock()
    sample.name = "deco_rock_sample"
    sample.location = (0.0, 0.0, 0.0)
    sample.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=SAMPLE_PATH,
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=True,
        export_cameras=False,
        export_lights=False,
    )


main()
