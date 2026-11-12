import bpy
import json
import math
import os
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODELS = ROOT / "assets/models"
TEXTURES = ROOT / "assets/textures"
ART = ROOT / "art/models"
NAMES = ["white", "steel", "dark", "ceramic", "rubber", "hazard", "visor", "coil", "orange", "bronze", "grunt_body", "grunt_head"]
COLORS = [(255, 255, 255), (119, 139, 147), (38, 49, 58), (211, 218, 198), (32, 30, 35), (238, 178, 38), (43, 212, 231), (243, 97, 36), (188, 82, 36), (141, 108, 69), (146, 156, 128), (188, 194, 161)]


def png(path, width, height, data):
    def chunk(kind, payload):
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff)
    rows = b"".join(b"\0" + data[y * width * 4:(y + 1) * width * 4] for y in range(height))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b""))


def atlas():
    pixels = bytearray([255] * (512 * 512 * 4))
    for index, (name, color) in enumerate(zip(NAMES, COLORS)):
        for y in range(64):
            for x in range(64):
                noise = ((x * 73 + y * 151 + x * y * 3) % 17 - 8) / 255
                edge = min(x, y, 63 - x, 63 - y)
                shade = 1.0 if name == "white" else 0.78 + noise
                if name != "white":
                    if edge < 2:
                        shade *= 0.45
                    elif edge < 4:
                        shade *= 1.2
                    if y in (12, 48) and name not in ("rubber", "visor", "coil"):
                        shade *= 0.7
                    if name == "rubber" and y % 6 < 2:
                        shade *= 0.5
                    if name == "hazard" and (x + y) % 24 < 12:
                        shade *= 0.12
                    if name == "visor":
                        shade = 0.55 + 0.4 * (1 - abs(y - 25) / 40)
                        if y % 8 == 0:
                            shade *= 0.6
                    if name == "coil" and x % 8 < 3:
                        shade *= 0.25
                    if name in ("grunt_body", "grunt_head") and 23 < x < 41 and 22 < y < 36:
                        shade *= 0.4
                p = (((index // 8) * 64 + y) * 512 + (index % 8) * 64 + x) * 4
                pixels[p:p + 4] = bytes([min(255, max(0, int(c * shade))) for c in color] + [255])
    png(TEXTURES / "combat.png", 512, 512, pixels)
    tiles = ",\n".join(f'        (name: "{name}", col: {i % 8}, row: {i // 8}, meters: 1.0)' for i, name in enumerate(NAMES))
    (TEXTURES / "combat.ron").write_text('(image: "combat.png", tile_size: 64, tiles: [\n' + tiles + '\n])\n', encoding="utf-8")


def box(size, center=(0, 0, 0), bevel=0):
    sx, sy, sz = [v / 2 for v in size]
    vertices = []
    faces = []
    for axis in range(3):
        a, b = (axis + 1) % 3, (axis + 2) % 3
        for sign in (-1, 1):
            start = len(vertices)
            for u, v in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
                p = [0, 0, 0]
                p[axis] = sign * [sx, sy, sz][axis]
                p[a] = u * [sx, sy, sz][a]
                p[b] = v * [sx, sy, sz][b]
                vertices.append(tuple(p[i] + center[i] for i in range(3)))
            order = (0, 1, 2, 3) if sign == 1 else (3, 2, 1, 0)
            faces.extend([(start + order[0], start + order[1], start + order[2]), (start + order[0], start + order[2], start + order[3])])
    return vertices, faces


def cylinder(radius, length, center=(0, 0, 0), axis=2, sides=8, end_radius=None):
    end_radius = radius if end_radius is None else end_radius
    vertices = []
    a, b = (axis + 1) % 3, (axis + 2) % 3
    for end, r in [(-1, radius), (1, end_radius)]:
        for i in range(sides):
            p = list(center)
            p[axis] += end * length / 2
            p[a] += math.cos(i * math.tau / sides) * r
            p[b] += math.sin(i * math.tau / sides) * r
            vertices.append(tuple(p))
    faces = []
    for i in range(sides):
        j = (i + 1) % sides
        faces.extend([(i, j, sides + j), (i, sides + j, sides + i)])
    for i in range(1, sides - 1):
        faces.extend([(0, i + 1, i), (sides, sides + i, sides + i + 1)])
    return vertices, faces


class Builder:
    def __init__(self):
        self.nodes = []
        self.meshes = []
        self.accessors = []
        self.views = []
        self.data = bytearray()

    def accessor(self, values, count, kind, component=5126):
        while len(self.data) % 4:
            self.data.append(0)
        offset = len(self.data)
        code = "I" if component == 5125 else "f"
        self.data.extend(struct.pack("<" + code * len(values), *values))
        self.views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(self.data) - offset})
        item = {"bufferView": len(self.views) - 1, "componentType": component, "count": count, "type": kind}
        if kind == "VEC3":
            item["min"] = [min(values[i::3]) for i in range(3)]
            item["max"] = [max(values[i::3]) for i in range(3)]
        self.accessors.append(item)
        return len(self.accessors) - 1

    def node(self, name, position=(0, 0, 0), parent=None, shape=None, material="steel"):
        node = {"name": name, "translation": list(position)}
        if shape:
            points, faces = shape
            positions, normals, uv = [], [], []
            for face in faces:
                p, q, r = [points[i] for i in face]
                u = [q[i] - p[i] for i in range(3)]
                v = [r[i] - p[i] for i in range(3)]
                n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
                length = math.sqrt(sum(x * x for x in n)) or 1
                n = [x / length for x in n]
                axis = max(range(3), key=lambda i: abs(n[i]))
                a, b = (axis + 1) % 3, (axis + 2) % 3
                mins = [min(point[i] for point in points) for i in range(3)]
                spans = [max(point[i] for point in points) - mins[i] for i in range(3)]
                for index in face:
                    point = points[index]
                    positions.extend(point)
                    normals.extend(n)
                    uv.extend([0.02 + 0.96 * (point[i] - mins[i]) / max(spans[i], 0.001) for i in (a, b)])
            count = len(positions) // 3
            primitive = {"attributes": {"POSITION": self.accessor(positions, count, "VEC3"), "NORMAL": self.accessor(normals, count, "VEC3"), "TEXCOORD_0": self.accessor(uv, count, "VEC2")}, "indices": self.accessor(list(range(count)), count, "SCALAR", 5125), "material": NAMES.index(material)}
            node["mesh"] = len(self.meshes)
            self.meshes.append({"primitives": [primitive]})
        index = len(self.nodes)
        self.nodes.append(node)
        if parent is not None:
            self.nodes[parent].setdefault("children", []).append(index)
        return index

    def save(self, name):
        children = {i for node in self.nodes for i in node.get("children", [])}
        materials = []
        for material in NAMES:
            item = {"name": material, "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0, "roughnessFactor": 1}}
            if material in ("visor", "coil"):
                item["emissiveFactor"] = [1, 1, 1]
            materials.append(item)
        doc = {"asset": {"version": "2.0", "generator": "1681 original deterministic models"}, "scene": 0, "scenes": [{"nodes": [i for i in range(len(self.nodes)) if i not in children]}], "nodes": self.nodes, "meshes": self.meshes, "materials": materials, "buffers": [{"byteLength": len(self.data)}], "bufferViews": self.views, "accessors": self.accessors}
        text = json.dumps(doc, separators=(",", ":")).encode()
        text += b" " * ((-len(text)) % 4)
        data = bytes(self.data) + b"\0" * ((-len(self.data)) % 4)
        payload = struct.pack("<II", len(text), 0x4e4f534a) + text + struct.pack("<II", len(data), 0x004e4942) + data
        path = MODELS / f"{name}.glb"
        path.write_bytes(struct.pack("<III", 0x46546c67, 2, len(payload) + 12) + payload)
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=str(path))
        bpy.ops.wm.save_as_mainfile(filepath=str(ART / f"{name}.blend"))
        print(f"{name}: {sum(a['count'] for a in self.accessors if a['type'] == 'SCALAR') // 3} triangles, {len(self.nodes)} nodes")


def grunt():
    m = Builder()
    root = m.node("root")
    hips = m.node("pelvis", (0, 0.94, 0), root, box((0.42, 0.25, 0.28)), "dark")
    body = m.node("torso", (0, 0.3, 0), hips, cylinder(0.29, 0.53, axis=1, sides=6, end_radius=0.36), "grunt_body")
    m.node("chest", (0, 0.06, -0.25), body, box((0.32, 0.19, 0.07)), "hazard")
    m.node("spine", (0, 0.1, 0.27), body, cylinder(0.08, 0.45, axis=1), "dark")
    head = m.node("head", (0, 0.48, 0), body, cylinder(0.2, 0.32, axis=1, sides=6, end_radius=0.16), "grunt_head")
    m.node("visor", (0, 0.04, -0.18), head, box((0.26, 0.075, 0.035)), "visor")
    for side, x, phase in [("left", -1, 0), ("right", 1, 1)]:
        arm = m.node(f"{side}_arm", (x * 0.35, 0.16, 0), body)
        m.node(f"{side}_shoulder", parent=arm, shape=cylinder(0.12, 0.13, axis=0), material="bronze")
        m.node(f"{side}_upper_arm", (0, -0.23, 0), arm, cylinder(0.09, 0.37, axis=1, sides=6), "ceramic")
        fore = m.node(f"{side}_forearm", (0, -0.44, 0), arm)
        m.node(f"{side}_elbow", parent=fore, shape=cylinder(0.1, 0.12, axis=0), material="dark")
        m.node(f"{side}_lower_arm", (0, -0.18, -0.02), fore, cylinder(0.105, 0.3, axis=1, sides=6), "steel")
        m.node(f"{side}_fist", (0, -0.38, -0.02), fore, box((0.19, 0.17, 0.2)), "rubber")
        leg = m.node(f"{side}_leg", (x * 0.15, -0.09, 0), hips)
        m.node(f"{side}_thigh", (0, -0.2, 0), leg, cylinder(0.12, 0.32, axis=1, sides=6), "grunt_body")
        shin = m.node(f"{side}_shin", (0, -0.43, 0), leg)
        m.node(f"{side}_knee", parent=shin, shape=cylinder(0.1, 0.15, axis=0), material="bronze")
        m.node(f"{side}_lower_leg", (0, -0.19, 0), shin, cylinder(0.09, 0.31, axis=1, sides=6), "steel")
        m.node(f"{side}_foot", (0, -0.36, -0.065), shin, box((0.19, 0.12, 0.36)), "rubber")
    m.save("grunt")


def weapon(name):
    m = Builder()
    root = m.node("root")
    m.node("grip", (0, -0.06, 0.015), root, cylinder(0.052, 0.19, axis=1, sides=6), "rubber")
    arm = m.node("hand", (0.02, -0.13, 0.02), root, box((0.12, 0.09, 0.15)), "orange")
    m.node("forearm", (0.02, -0.13, 0.22), root, cylinder(0.07, 0.32, axis=2, sides=6), "dark")
    m.node("cuff", (0.02, -0.13, 0.1), root, cylinder(0.08, 0.06, axis=2), "hazard")
    if name == "rivet_gun":
        m.node("housing", (0, 0.07, -0.12), root, box((0.115, 0.14, 0.31)), "steel")
        m.node("barrel", (0, 0.06, -0.35), root, cylinder(0.047, 0.22, axis=2), "dark")
        m.node("bolt", (0, 0.15, -0.12), root, box((0.09, 0.04, 0.24)), "bronze")
        m.node("chamber", (0, 0.05, 0.04), root, cylinder(0.09, 0.11, axis=2), "orange")
        m.node("sight", (0, 0.18, -0.17), root, box((0.025, 0.025, 0.15)), "ceramic")
        m.node("muzzle", (0, 0.06, -0.48), root)
        m.node("charge", (0.06, 0.09, -0.12), root, box((0.01, 0.025, 0.16)), "coil")
    elif name == "sandblaster":
        m.node("housing", (0, 0.065, -0.15), root, box((0.23, 0.18, 0.38)), "orange")
        m.node("nozzle", (0, 0.065, -0.43), root, cylinder(0.11, 0.24, axis=2, sides=8, end_radius=0.07), "steel")
        m.node("muzzle", (0, 0.065, -0.57), root)
        m.node("tank", (-0.1, -0.04, -0.13), root, cylinder(0.085, 0.31, axis=2), "ceramic")
        m.node("bolt", (0, 0.18, -0.18), root, box((0.11, 0.04, 0.22)), "dark")
        m.node("pressure", (0.1, 0.07, -0.18), root, cylinder(0.055, 0.03, axis=0), "visor")
        m.node("guard", (0, -0.025, -0.36), root, box((0.27, 0.04, 0.055)), "hazard")
    else:
        m.node("housing", (0, 0.06, -0.12), root, box((0.18, 0.15, 0.34)), "ceramic")
        core = m.node("core", (0, 0.09, -0.31), root, cylinder(0.06, 0.24, axis=2), "coil")
        for side in (-1, 1):
            m.node(f"rail_{side}", (side * 0.085, 0.1, -0.38), root, box((0.05, 0.09, 0.35)), "dark")
            m.node(f"rail_tip_{side}", (side * 0.085, 0.1, -0.57), root, box((0.06, 0.12, 0.07)), "bronze")
        m.node("muzzle", (0, 0.1, -0.59), root)
        m.node("bolt", (0, 0.16, -0.08), root, box((0.11, 0.045, 0.14)), "steel")
    m.save(name)


for directory in (MODELS, TEXTURES, ART):
    directory.mkdir(parents=True, exist_ok=True)
atlas()
grunt()
for name in ("rivet_gun", "sandblaster", "plasma_cutter"):
    weapon(name)
