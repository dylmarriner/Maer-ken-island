"""Generate Maer-Ken biome terrain meshes and deterministic planet materials."""

import argparse
import json
import math
import os
import sys

import bpy


BIOMES = {
    "deep_ocean": ((0.02, 0.08, 0.22), 0.15),
    "shallow_ocean": ((0.05, 0.20, 0.45), 0.18),
    "coastal_waters": ((0.15, 0.45, 0.70), 0.22),
    "reef_sea": ((0.12, 0.55, 0.52), 0.25),
    "desert": ((0.72, 0.54, 0.28), 0.72),
    "semi_desert": ((0.58, 0.46, 0.28), 0.68),
    "savanna": ((0.58, 0.62, 0.22), 0.75),
    "grassland": ((0.30, 0.56, 0.20), 0.82),
    "shrubland": ((0.24, 0.42, 0.18), 0.84),
    "tundra": ((0.45, 0.52, 0.48), 0.86),
    "tropical_rainforest": ((0.08, 0.38, 0.12), 0.88),
    "tropical_dry_forest": ((0.25, 0.46, 0.16), 0.86),
    "temperate_forest": ((0.16, 0.36, 0.14), 0.90),
    "boreal_forest": ((0.12, 0.28, 0.20), 0.90),
    "woodland": ((0.26, 0.42, 0.16), 0.86),
    "montane_forest": ((0.20, 0.30, 0.18), 0.92),
    "alpine": ((0.48, 0.50, 0.48), 0.94),
    "wetland": ((0.12, 0.34, 0.28), 0.60),
    "river": ((0.10, 0.40, 0.68), 0.16),
    "volcanic": ((0.16, 0.10, 0.08), 0.96),
    "ice_sheet": ((0.78, 0.88, 0.94), 0.35),
}

MATERIALS = {
    "ocean_deep": (0.02, 0.08, 0.22), "ocean_shallow": (0.05, 0.20, 0.45),
    "coast_sand": (0.70, 0.62, 0.42), "reef": (0.10, 0.55, 0.52),
    "desert_sand": (0.72, 0.54, 0.28), "soil": (0.28, 0.20, 0.12),
    "grass": (0.30, 0.56, 0.20), "forest": (0.12, 0.30, 0.12),
    "wetland": (0.12, 0.34, 0.28), "river": (0.10, 0.40, 0.68),
    "rock": (0.36, 0.36, 0.36), "ice": (0.78, 0.88, 0.94),
    "lava": (0.75, 0.08, 0.015),
}


def mat(name, color):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color, 1.0)
    return m


def cube(name, loc, scale, material):
    bpy.ops.mesh.primitive_cube_add(location=loc)
    o = bpy.context.object
    o.name = name
    o.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    o.data.materials.append(material)
    return o


def terrain_mesh(slug, color):
    material = mat("Biome " + slug, color)
    cube(slug + " base", (0, 0, -0.18), (1.5, 1.5, 0.18), material)
    # Low-poly deterministic relief; biome tiles are source/material previews,
    # while the live planet uses the simulation heightfield.
    for i in range(7):
        angle = i * math.tau / 7.0
        h = 0.12 + (i % 3) * 0.08
        cube(slug + " relief", (math.cos(angle) * 0.8, math.sin(angle) * 0.8, h), (0.25, 0.25, h), material)
    if "ocean" in slug or slug in ("coastal_waters", "reef_sea", "river"):
        cube(slug + " water", (0, 0, 0.12), (1.42, 1.42, 0.035), material)
    if slug == "ice_sheet":
        cube(slug + " cap", (0, 0, 0.28), (1.2, 1.2, 0.16), material)
    if slug == "volcanic":
        cone = cube(slug + " lava", (0, 0, 0.35), (0.35, 0.35, 0.35), mat("Volcanic lava", MATERIALS["lava"]))
        cone.rotation_euler[2] = math.radians(45)


# Height field shared by every map of a material, so the normal map is the
# true normal of the surface the base colour and roughness are shaded from.
HEIGHT_AMPLITUDE = 0.18
NORMAL_STRENGTH = 4.0


def _wave(x, y):
    return (math.sin(x * 0.31) + math.cos(y * 0.27) + math.sin((x + y) * 0.11)) / 3.0


def surface_height(x, y):
    return max(0.0, min(1.0, 0.5 + _wave(x, y) * HEIGHT_AMPLITUDE))


def surface_normal(x, y):
    # d(wave)/dx and d(wave)/dy, scaled like the height.
    dx = (0.31 * math.cos(x * 0.31) + 0.11 * math.cos((x + y) * 0.11)) / 3.0
    dy = (-0.27 * math.sin(y * 0.27) + 0.11 * math.cos((x + y) * 0.11)) / 3.0
    gx = dx * HEIGHT_AMPLITUDE * NORMAL_STRENGTH
    gy = dy * HEIGHT_AMPLITUDE * NORMAL_STRENGTH
    length = math.sqrt(gx * gx + gy * gy + 1.0)
    return (-gx / length, -gy / length, 1.0 / length)


def pixel_texture(path, color, roughness=0.6, mode="base"):
    size = 64
    image = bpy.data.images.new(os.path.basename(path), width=size, height=size)
    pixels = [0.0] * (size * size * 4)
    for y in range(size):
        for x in range(size):
            n = surface_height(x, y)
            i = (y * size + x) * 4
            if mode == "roughness":
                pixels[i:i + 4] = [0.0, roughness * (0.82 + n * 0.18), 0.0, 1.0]
            elif mode == "normal":
                # Tangent-space normal of the same height field, from its
                # analytic gradient, encoded as 0.5 + 0.5 * n.
                nx, ny, nz = surface_normal(x, y)
                pixels[i:i + 4] = [0.5 + 0.5 * nx, 0.5 + 0.5 * ny, 0.5 + 0.5 * nz, 1.0]
            elif mode == "emission":
                e = n if color[0] > color[2] else 0.0
                pixels[i:i + 4] = [color[0] * e, color[1] * e, color[2] * e, 1.0]
            else:
                pixels[i:i + 4] = [min(1.0, color[0] * (0.86 + n * 0.14)), min(1.0, color[1] * (0.86 + n * 0.14)), min(1.0, color[2] * (0.86 + n * 0.14)), 1.0]
    image.pixels = pixels
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()


def clear():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for collection in (bpy.data.meshes, bpy.data.curves, bpy.data.cameras, bpy.data.lights, bpy.data.images):
        for block in list(collection):
            if block.users == 0:
                collection.remove(block)


def generate(output):
    clear()
    terrain_dir = os.path.join(output, "terrain_biomes")
    material_dir = os.path.join(output, "planet_materials")
    os.makedirs(terrain_dir, exist_ok=True)
    os.makedirs(material_dir, exist_ok=True)
    manifest = {"manifest_id": "MAERKEN_TERRAIN_MATERIALS_V1", "terrain_meshes": [], "materials": []}
    for slug, (color, roughness) in BIOMES.items():
        bpy.ops.object.select_all(action="DESELECT")
        terrain_mesh(slug, color)
        objects = list(bpy.context.scene.objects)
        for o in objects:
            o.select_set(True)
        path = os.path.join(terrain_dir, slug + ".glb")
        bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True)
        manifest["terrain_meshes"].append({"id": slug, "path": os.path.relpath(path, output), "geometry_status": "procedural_biome_preview"})
        bpy.ops.object.select_all(action="SELECT")
        bpy.ops.object.delete(use_global=False)
    for slug, color in MATERIALS.items():
        files = {}
        for mode in ("basecolor", "roughness", "normal", "emission"):
            path = os.path.join(material_dir, slug + "_" + mode + ".png")
            pixel_texture(path, color, 0.6 if mode != "roughness" else 0.75, mode if mode != "basecolor" else "base")
            files[mode] = os.path.relpath(path, output)
        manifest["materials"].append({"id": slug, "textures": files, "material_status": "procedural_deterministic"})
    with open(os.path.join(output, "asset_manifest.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    args, _ = parser.parse_known_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    generate(os.path.abspath(args.output))
