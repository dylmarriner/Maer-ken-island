"""Generate deterministic low-poly GLB assets for the reconstructed Gem property.

These are Maer-Ken reconstructions from the in-repository property manifest,
not exports copied from the unavailable Gemini Universe source repository.
Run with Blender's Python interpreter:
  blender -b --python tools/assets/generate_gem_property_assets.py -- --output <dir>
"""

import argparse
import os
import sys

import bpy


ASSETS = {
    "buildings": {
        "homestead_house": "House",
        "equipment_shed": "Shed",
        "building_workshop": "Workshop",
        "secure_armoury": "Armoury",
        "computer_room": "ComputerRoom",
    },
    "vehicles": {
        "utility_truck": "Vehicle",
        "all_terrain_utility_vehicle": "Vehicle",
        "motorbike": "Vehicle",
        "utility_trailer": "Vehicle",
    },
    "building_equipment": {
        "heavy_workbench": "BuildingEquipment",
        "portable_generator": "BuildingEquipment",
        "welding_station": "BuildingEquipment",
        "power_tool_set": "BuildingEquipment",
        "hand_tool_set": "BuildingEquipment",
        "ladder_set": "BuildingEquipment",
        "material_hoist": "BuildingEquipment",
    },
    "armoury": {
        "secure_locker": "ArmouryItem",
        "protective_equipment_set": "ArmouryItem",
        "field_safety_kit": "ArmouryItem",
    },
    "computers": {
        "primary_workstation": "Computer",
        "portable_laptop": "Computer",
        "local_archive_server": "Computer",
        "network_equipment_rack": "Computer",
    },
    "household": {
        "household_storage_set": "HouseholdItem",
        "water_storage_tank": "HouseholdItem",
        "emergency_medical_kit": "HouseholdItem",
    },
}


def material(name, color, metallic=0.0, roughness=0.7):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.metallic = metallic
    mat.roughness = roughness
    return mat


MAT = {
    "body": material("MaerKen body", (0.18, 0.28, 0.34), 0.25),
    "structure": material("MaerKen structure", (0.34, 0.42, 0.46), 0.1),
    "dark": material("MaerKen dark metal", (0.04, 0.06, 0.07), 0.7),
    "accent": material("MaerKen safety accent", (0.85, 0.35, 0.06), 0.1),
    "glass": material("MaerKen glass", (0.08, 0.28, 0.42), 0.35, 0.18),
    "medical": material("MaerKen medical", (0.72, 0.82, 0.78), 0.0),
}


def apply(obj, mat):
    obj.data.materials.append(mat)
    return obj


def cube(name, location, scale, mat="body", bevel=0.04):
    bpy.ops.mesh.primitive_cube_add(location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        modifier = obj.modifiers.new("edge bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 2
    return apply(obj, MAT[mat])


def cylinder(name, location, radius, depth, mat="body", vertices=16):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=location)
    return apply(bpy.context.object, MAT[mat])


def sphere(name, location, scale, mat="body"):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=1, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return apply(obj, MAT[mat])


def building(kind):
    cube(kind, (0, 0, 1.0), (1.5, 1.1, 1.0), "structure", 0.08)
    cube(kind + " roof", (0, 0, 2.15), (1.65, 1.25, 0.12), "dark")
    cube(kind + " door", (0, -1.12, 0.7), (0.32, 0.05, 0.7), "dark")
    cube(kind + " window", (-0.65, -1.13, 1.35), (0.38, 0.04, 0.28), "glass", 0.01)
    cube(kind + " window 2", (0.65, -1.13, 1.35), (0.38, 0.04, 0.28), "glass", 0.01)
    if kind in ("Equipment Shed", "Building Workshop"):
        cube(kind + " safety stripe", (0, -1.27, 0.22), (1.2, 0.03, 0.08), "accent", 0.01)


def asset_geometry(name, category):
    if category in ("House", "Shed", "Workshop", "Armoury", "ComputerRoom"):
        building(name.replace("_", " ").title())
    elif category == "Vehicle":
        cube(name, (0, 0, 0.45), (1.25, 0.62, 0.32), "body")
        for x in (-0.78, 0.78):
            for y in (-0.52, 0.52):
                cylinder(name + " wheel", (x, y, 0.32), 0.28, 0.18, "dark")
        cube(name + " cabin", (0.25, 0, 0.95), (0.55, 0.52, 0.35), "glass")
        if "motorbike" in name:
            cylinder(name + " frame", (0, 0, 0.8), 0.12, 1.4, "dark")
    elif category == "BuildingEquipment":
        cube(name, (0, 0, 0.45), (0.8, 0.45, 0.35), "dark")
        cube(name + " top", (0, 0, 0.9), (0.9, 0.52, 0.08), "accent")
        if "ladder" in name:
            for x in (-0.5, 0.5):
                cylinder(name + " rail", (x, 0, 1.25), 0.06, 2.0, "structure")
        if "hoist" in name:
            cylinder(name + " arm", (0, 0, 1.5), 0.08, 2.5, "structure")
    elif category == "ArmouryItem":
        cube(name, (0, 0, 0.5), (0.7, 0.35, 0.5), "dark")
        cube(name + " safety marking", (0, -0.37, 0.5), (0.22, 0.03, 0.22), "accent", 0.01)
        if "protective" in name:
            sphere(name + " helmet", (0, 0, 1.25), (0.35, 0.3, 0.25), "medical")
    elif category == "Computer":
        cube(name, (0, 0, 0.45), (0.62, 0.42, 0.08), "dark")
        cube(name + " display", (0, 0.05, 0.85), (0.48, 0.04, 0.35), "glass", 0.02)
        cube(name + " indicator", (0, -0.01, 0.88), (0.08, 0.02, 0.04), "accent", 0.01)
    else:
        cube(name, (0, 0, 0.4), (0.6, 0.45, 0.4), "medical" if "medical" in name else "body")
        if "tank" in name:
            cylinder(name + " tank", (0, 0, 0.9), 0.5, 1.2, "structure")


def generate(output_root):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.meshes, bpy.data.curves, bpy.data.cameras, bpy.data.lights):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)

    os.makedirs(output_root, exist_ok=True)
    manifest = []
    for category, entries in ASSETS.items():
        category_dir = os.path.join(output_root, category)
        os.makedirs(category_dir, exist_ok=True)
        for name, kind in entries.items():
            bpy.ops.object.select_all(action="DESELECT")
            asset_geometry(name, kind)
            for obj in bpy.context.scene.objects:
                obj.select_set(True)
            root = bpy.context.scene.objects[0]
            root["asset_id"] = "gem_property_" + name
            root["source"] = "maerken_reconstruction"
            root["category"] = category
            root["property_kind"] = kind
            path = os.path.join(category_dir, name + ".glb")
            bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True)
            manifest.append({"asset_id": "gem_property_" + name, "category": category, "kind": kind, "path": os.path.relpath(path, output_root)})
            bpy.ops.object.select_all(action="SELECT")
            bpy.ops.object.delete(use_global=False)

    with open(os.path.join(output_root, "asset_manifest.json"), "w", encoding="utf-8") as handle:
        import json
        json.dump({"manifest_id": "GEM_PROPERTY_RECONSTRUCTION_V1", "source": "maerken_reconstruction", "assets": manifest}, handle, indent=2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    args, _ = parser.parse_known_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    generate(os.path.abspath(args.output))
