"""Generate humanoid glTF/GLB models from a body fixture.

The models are assembled from Blender primitives (no armature or rig).
Gem-D and Gem-K have hand-authored builders modelled on the k-dlooks
reference imagery, whose proportions already encode each founder's build,
so only their height is read from the fixture. Every other character uses
the generic humanoid, which also scales by the fixture's build.

Hair, eye and skin colours must be names in COLOR_LOOKUP; an unknown name
is an error rather than a silent grey.

Run headlessly:
    blender -b --python tools/blender/generate_human_model.py -- \
        --character gem-d \
        --input fixtures/human/body_gem_d.json \
        --output apps/mk_ui/assets/models/GemD.glb \
        --skin-tone olive
"""

from __future__ import annotations

import argparse
import json
import math
import pathlib
import sys

import bpy

BUILD_MULTIPLIERS = {
    "slender": 0.85,
    "athletic": 1.15,
    "average": 1.0,
    "stocky": 1.3,
    "heavy": 1.4,
}
DEFAULT_BUILD_MULTIPLIER = 1.0

COLOR_LOOKUP = {
    "brown": (0.30, 0.18, 0.09),
    "black": (0.03, 0.03, 0.03),
    "blond": (0.80, 0.65, 0.35),
    "blonde": (0.80, 0.65, 0.35),
    "auburn": (0.45, 0.17, 0.08),
    "amber": (0.65, 0.42, 0.12),
    "red": (0.55, 0.20, 0.10),
    "gray": (0.60, 0.60, 0.60),
    "grey": (0.60, 0.60, 0.60),
    "white": (0.90, 0.90, 0.90),
    "blue": (0.20, 0.35, 0.65),
    "hazel": (0.45, 0.35, 0.15),
    "green": (0.20, 0.45, 0.25),
    "olive": (0.55, 0.42, 0.30),
    "fair": (0.85, 0.70, 0.60),
    "tan": (0.70, 0.50, 0.35),
    "dark": (0.30, 0.20, 0.13),
}


def resolve_color(name: str) -> tuple[float, float, float]:
    key = (name or "").strip().lower()
    if key not in COLOR_LOOKUP:
        known = ", ".join(sorted(COLOR_LOOKUP))
        raise SystemExit(f"generate_human_model: unknown colour {name!r}; known colours: {known}")
    return COLOR_LOOKUP[key]


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for block in bpy.data.meshes:
        if block.users == 0:
            bpy.data.meshes.remove(block)
    for block in bpy.data.materials:
        if block.users == 0:
            bpy.data.materials.remove(block)


def make_material(
    name: str,
    rgb: tuple[float, float, float],
    roughness: float = 0.55,
    metallic: float = 0.0,
    transmission: float = 0.0,
):
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf is None:
        raise RuntimeError(f"Missing Principled BSDF node for material {name}")
    bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if "Transmission Weight" in bsdf.inputs:
        bsdf.inputs["Transmission Weight"].default_value = transmission
    elif "Transmission" in bsdf.inputs:
        bsdf.inputs["Transmission"].default_value = transmission
    return mat


def add_cylinder(
    name: str,
    radius: float,
    length: float,
    location: tuple[float, float, float],
    material,
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
    scale: tuple[float, float, float] = (1.0, 1.0, 1.0),
):
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius, depth=length, location=location, rotation=rotation
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material)
    return obj


def add_sphere(
    name: str,
    radius: float,
    location: tuple[float, float, float],
    material,
    scale: tuple[float, float, float] = (1.0, 1.0, 1.0),
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
):
    bpy.ops.mesh.primitive_uv_sphere_add(
        radius=radius, location=location, rotation=rotation
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material)
    return obj


def add_cube(
    name: str,
    size: float,
    location: tuple[float, float, float],
    material,
    scale: tuple[float, float, float] = (1.0, 1.0, 1.0),
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
):
    bpy.ops.mesh.primitive_cube_add(
        size=size, location=location, rotation=rotation
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material)
    return obj


def add_torus(
    name: str,
    major_radius: float,
    minor_radius: float,
    location: tuple[float, float, float],
    material,
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
    scale: tuple[float, float, float] = (1.0, 1.0, 1.0),
):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major_radius,
        minor_radius=minor_radius,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material)
    return obj


def add_cone(
    name: str,
    radius1: float,
    radius2: float,
    depth: float,
    location: tuple[float, float, float],
    material,
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
    scale: tuple[float, float, float] = (1.0, 1.0, 1.0),
):
    bpy.ops.mesh.primitive_cone_add(
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.clear()
    obj.data.materials.append(material)
    return obj


def join_objects(objects, name: str):
    bpy.ops.object.select_all(action="DESELECT")
    valid_objs = [o for o in objects if o is not None]
    if not valid_objs:
        return None
    for obj in valid_objs:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = valid_objs[0]
    bpy.ops.object.join()
    joined = bpy.context.active_object
    joined.name = name
    return joined


# ---------------------------------------------------------------------------
# Gem-D Character Model Synthesis
# ---------------------------------------------------------------------------


def build_gem_d_model(
    height_cm: float,
    skin_mat,
    hair_mat,
    eye_mat,
    gold_mat,
    black_fabric_mat,
    black_rubber_mat,
    red_rubber_mat,
    sunglasses_mat,
):
    scale = height_cm / 178.0
    parts = []

    # 1. Head & Facial Features
    head_z = 1.65 * scale
    head_r = 0.105 * scale
    parts.append(
        add_sphere(
            "Head",
            head_r,
            (0.0, 0.0, head_z),
            skin_mat,
            scale=(0.94, 1.0, 1.06),
        )
    )

    # Ears
    for side in [-1.0, 1.0]:
        parts.append(
            add_sphere(
                f"Ear_{side}",
                head_r * 0.28,
                (side * head_r * 0.95, 0.0, head_z),
                skin_mat,
                scale=(0.3, 0.6, 0.8),
            )
        )

    # Nose
    parts.append(
        add_cone(
            "Nose",
            0.015 * scale,
            0.002 * scale,
            0.035 * scale,
            (0.0, -head_r * 0.98, head_z - 0.01 * scale),
            skin_mat,
            rotation=(math.radians(90), 0.0, 0.0),
        )
    )

    # Hair (Short brown hair styled on top with tapered sides)
    parts.append(
        add_sphere(
            "HairCap",
            head_r * 1.02,
            (0.0, 0.01 * scale, head_z + 0.02 * scale),
            hair_mat,
            scale=(0.96, 1.02, 0.92),
        )
    )
    # Textured hair volume on top
    parts.append(
        add_sphere(
            "HairTopVolume",
            head_r * 0.75,
            (0.0, -0.01 * scale, head_z + head_r * 0.65),
            hair_mat,
            scale=(0.95, 1.15, 0.55),
        )
    )

    # Sunglasses (Black frame, dark polarized lenses, gold temple hinges)
    lens_w = 0.036 * scale
    lens_h = 0.022 * scale
    lens_d = 0.008 * scale
    lens_y = -head_r * 0.98
    lens_z = head_z + 0.025 * scale

    for side in [-1.0, 1.0]:
        # Lens
        parts.append(
            add_cube(
                f"Lens_{side}",
                1.0,
                (side * 0.042 * scale, lens_y, lens_z),
                sunglasses_mat,
                scale=(lens_w, lens_d, lens_h),
            )
        )
        # Frame border
        parts.append(
            add_cube(
                f"Frame_{side}",
                1.0,
                (side * 0.042 * scale, lens_y + 0.002 * scale, lens_z),
                black_fabric_mat,
                scale=(
                    lens_w + 0.006 * scale,
                    lens_d * 0.5,
                    lens_h + 0.006 * scale,
                ),
            )
        )
        # Gold Temple Hinge
        parts.append(
            add_cylinder(
                f"GoldTemple_{side}",
                0.0035 * scale,
                0.09 * scale,
                (side * (head_r * 0.92), 0.02 * scale, lens_z),
                gold_mat,
                rotation=(math.radians(90), 0.0, 0.0),
            )
        )

    # Bridge between lenses
    parts.append(
        add_cube(
            "SunglassesBridge",
            1.0,
            (0.0, lens_y, lens_z + 0.005 * scale),
            black_fabric_mat,
            scale=(0.016 * scale, lens_d * 0.8, 0.006 * scale),
        )
    )

    # 2. Neck & Muscular Torso
    neck_z = 1.49 * scale
    neck_h = 0.10 * scale
    parts.append(
        add_cylinder(
            "Neck",
            0.052 * scale,
            neck_h,
            (0.0, 0.0, neck_z),
            skin_mat,
        )
    )

    # Torso (Athletic V-Taper)
    torso_z = 1.18 * scale
    torso_h = 0.54 * scale
    torso_w = 0.18 * scale
    torso_d = 0.11 * scale
    parts.append(
        add_cylinder(
            "Torso",
            1.0,
            torso_h,
            (0.0, 0.0, torso_z),
            black_fabric_mat,
            scale=(torso_w, torso_d, 1.0),
        )
    )

    # Shoulders (Skin under sleeveless tank)
    for side in [-1.0, 1.0]:
        parts.append(
            add_sphere(
                f"Shoulder_{side}",
                0.062 * scale,
                (side * 0.19 * scale, 0.0, 1.42 * scale),
                skin_mat,
            )
        )

    # 3. Gold Versace Emblem & Lettering on Tank Top
    medallion_y = -torso_d * 1.02
    medallion_z = 1.34 * scale
    # Outer gold ring
    parts.append(
        add_torus(
            "MedallionRing",
            0.056 * scale,
            0.005 * scale,
            (0.0, medallion_y, medallion_z),
            gold_mat,
            rotation=(math.radians(90), 0.0, 0.0),
        )
    )
    # Inner gold medallion relief disc
    parts.append(
        add_cylinder(
            "MedallionDisc",
            0.052 * scale,
            0.004 * scale,
            (0.0, medallion_y, medallion_z),
            gold_mat,
            rotation=(math.radians(90), 0.0, 0.0),
        )
    )
    # Gold "VERSACE" text accent bar
    parts.append(
        add_cube(
            "GoldTextBar",
            1.0,
            (0.0, medallion_y, 1.25 * scale),
            gold_mat,
            scale=(0.065 * scale, 0.004 * scale, 0.012 * scale),
        )
    )

    # 4. Arms & Hands
    for side in [-1.0, 1.0]:
        # Upper arm (Bicep/Tricep)
        parts.append(
            add_cylinder(
                f"UpperArm_{side}",
                0.048 * scale,
                0.28 * scale,
                (side * 0.22 * scale, 0.0, 1.28 * scale),
                skin_mat,
            )
        )
        # Forearm
        parts.append(
            add_cylinder(
                f"Forearm_{side}",
                0.042 * scale,
                0.26 * scale,
                (side * 0.23 * scale, 0.0, 1.02 * scale),
                skin_mat,
            )
        )
        # Hand
        parts.append(
            add_cube(
                f"Hand_{side}",
                1.0,
                (side * 0.235 * scale, 0.0, 0.85 * scale),
                skin_mat,
                scale=(0.028 * scale, 0.045 * scale, 0.075 * scale),
            )
        )

    # 5. Black Athletic Shorts with Gold Greek Key Waistband
    waist_z = 0.91 * scale
    waist_h = 0.06 * scale
    # Gold Greek key waistband
    parts.append(
        add_cylinder(
            "GoldWaistband",
            1.0,
            waist_h,
            (0.0, 0.0, waist_z),
            gold_mat,
            scale=(0.17 * scale, 0.115 * scale, 1.0),
        )
    )
    # Shorts legs
    shorts_z = 0.74 * scale
    shorts_h = 0.30 * scale
    for side in [-1.0, 1.0]:
        parts.append(
            add_cylinder(
                f"ShortsLeg_{side}",
                0.095 * scale,
                shorts_h,
                (side * 0.088 * scale, 0.0, shorts_z),
                black_fabric_mat,
            )
        )
        # Gold side stripe on shorts
        parts.append(
            add_cube(
                f"ShortsStripe_{side}",
                1.0,
                (side * 0.185 * scale, 0.0, shorts_z),
                gold_mat,
                scale=(0.005 * scale, 0.02 * scale, shorts_h * 0.9),
            )
        )

    # Gold round medallion on lower left leg hem
    parts.append(
        add_cylinder(
            "ShortsMedallion",
            0.022 * scale,
            0.003 * scale,
            (0.09 * scale, -0.095 * scale, 0.64 * scale),
            gold_mat,
            rotation=(math.radians(90), 0.0, 0.0),
        )
    )

    # 6. Bare Legs & Wellington Rubber Boots
    for side in [-1.0, 1.0]:
        # Bare leg (Thigh & Knee above boots)
        parts.append(
            add_cylinder(
                f"LegSkin_{side}",
                0.065 * scale,
                0.35 * scale,
                (side * 0.085 * scale, 0.0, 0.52 * scale),
                skin_mat,
            )
        )

        # Wellington Boot Shaft (Black rubber)
        boot_shaft_z = 0.22 * scale
        boot_shaft_h = 0.32 * scale
        parts.append(
            add_cylinder(
                f"BootShaft_{side}",
                0.068 * scale,
                boot_shaft_h,
                (side * 0.085 * scale, 0.0, boot_shaft_z),
                black_rubber_mat,
            )
        )

        # Red top collar trim on boot
        parts.append(
            add_cylinder(
                f"BootRedRim_{side}",
                0.071 * scale,
                0.025 * scale,
                (side * 0.085 * scale, 0.0, 0.37 * scale),
                red_rubber_mat,
            )
        )

        # Boot foot base
        parts.append(
            add_cube(
                f"BootFoot_{side}",
                1.0,
                (side * 0.085 * scale, -0.04 * scale, 0.038 * scale),
                black_rubber_mat,
                scale=(0.09 * scale, 0.22 * scale, 0.075 * scale),
            )
        )

        # Red reinforced toe cap
        parts.append(
            add_sphere(
                f"BootRedToe_{side}",
                0.046 * scale,
                (side * 0.085 * scale, -0.115 * scale, 0.036 * scale),
                red_rubber_mat,
                scale=(0.95, 1.2, 0.75),
            )
        )

    human = join_objects(parts, "GemD_Model")
    human.location = (0.0, 0.0, 0.0)
    bpy.ops.object.shade_smooth()
    return human


# ---------------------------------------------------------------------------
# Gem-K Character Model Synthesis
# ---------------------------------------------------------------------------


def build_gem_k_model(
    height_cm: float,
    skin_mat,
    hair_mat,
    eye_mat,
    white_fabric_mat,
    navy_logo_mat,
    combat_boot_mat,
    sole_mat,
):
    scale = height_cm / 165.0
    parts = []

    # 1. Head & Facial Features
    head_z = 1.52 * scale
    head_r = 0.095 * scale
    parts.append(
        add_sphere(
            "Head",
            head_r,
            (0.0, 0.0, head_z),
            skin_mat,
            scale=(0.90, 0.98, 1.02),
        )
    )

    # Ears
    for side in [-1.0, 1.0]:
        parts.append(
            add_sphere(
                f"Ear_{side}",
                head_r * 0.25,
                (side * head_r * 0.92, 0.0, head_z),
                skin_mat,
                scale=(0.25, 0.5, 0.75),
            )
        )

    # Nose
    parts.append(
        add_cone(
            "Nose",
            0.012 * scale,
            0.002 * scale,
            0.028 * scale,
            (0.0, -head_r * 0.96, head_z - 0.008 * scale),
            skin_mat,
            rotation=(math.radians(90), 0.0, 0.0),
        )
    )

    # Eyes (Brown)
    for side in [-1.0, 1.0]:
        parts.append(
            add_sphere(
                f"Eye_{side}",
                0.012 * scale,
                (side * 0.035 * scale, -head_r * 0.88, head_z + 0.015 * scale),
                eye_mat,
            )
        )

    # Long Dark Flowing Hair
    # Crown cap
    parts.append(
        add_sphere(
            "HairCrown",
            head_r * 1.03,
            (0.0, 0.01 * scale, head_z + 0.015 * scale),
            hair_mat,
            scale=(0.95, 1.02, 0.95),
        )
    )
    # Long flowing back cascade
    back_hair_z = 1.30 * scale
    back_hair_h = 0.44 * scale
    parts.append(
        add_cube(
            "HairBackCascade",
            1.0,
            (0.0, 0.065 * scale, back_hair_z),
            hair_mat,
            scale=(0.22 * scale, 0.06 * scale, back_hair_h),
            rotation=(math.radians(-5), 0.0, 0.0),
        )
    )
    # Front flowing locks over shoulders
    for side in [-1.0, 1.0]:
        parts.append(
            add_cylinder(
                f"HairFrontLock_{side}",
                0.024 * scale,
                0.32 * scale,
                (side * 0.085 * scale, -0.045 * scale, 1.36 * scale),
                hair_mat,
                rotation=(math.radians(10), side * math.radians(-5), 0.0),
            )
        )

    # 2. Neck & Upper Torso
    neck_z = 1.38 * scale
    neck_h = 0.09 * scale
    parts.append(
        add_cylinder(
            "Neck",
            0.044 * scale,
            neck_h,
            (0.0, 0.0, neck_z),
            skin_mat,
        )
    )

    # White Polo Crop Top
    top_z = 1.25 * scale
    top_h = 0.20 * scale
    top_w = 0.155 * scale
    top_d = 0.095 * scale
    parts.append(
        add_cylinder(
            "PoloCropTop",
            1.0,
            top_h,
            (0.0, 0.0, top_z),
            white_fabric_mat,
            scale=(top_w, top_d, 1.0),
        )
    )

    # Polo Collar
    parts.append(
        add_torus(
            "PoloCollar",
            0.055 * scale,
            0.008 * scale,
            (0.0, 0.0, 1.34 * scale),
            white_fabric_mat,
        )
    )

    # Navy Polo Logo on Left Chest
    parts.append(
        add_cube(
            "NavyLogo",
            1.0,
            (-0.062 * scale, -top_d * 1.02, 1.28 * scale),
            navy_logo_mat,
            scale=(0.012 * scale, 0.003 * scale, 0.016 * scale),
        )
    )

    # Shoulders & Short Polo Sleeves
    for side in [-1.0, 1.0]:
        # Sleeve cap
        parts.append(
            add_cylinder(
                f"Sleeve_{side}",
                0.044 * scale,
                0.11 * scale,
                (side * 0.17 * scale, 0.0, 1.29 * scale),
                white_fabric_mat,
            )
        )
        # Bare Forearm
        parts.append(
            add_cylinder(
                f"Forearm_{side}",
                0.035 * scale,
                0.26 * scale,
                (side * 0.185 * scale, 0.0, 1.05 * scale),
                skin_mat,
            )
        )
        # Hand
        parts.append(
            add_cube(
                f"Hand_{side}",
                1.0,
                (side * 0.19 * scale, 0.0, 0.88 * scale),
                skin_mat,
                scale=(0.024 * scale, 0.038 * scale, 0.065 * scale),
            )
        )

    # 3. Toned Athletic Midriff (Bare Skin)
    midriff_z = 1.08 * scale
    midriff_h = 0.15 * scale
    parts.append(
        add_cylinder(
            "TonedMidriff",
            1.0,
            midriff_h,
            (0.0, 0.0, midriff_z),
            skin_mat,
            scale=(0.14 * scale, 0.088 * scale, 1.0),
        )
    )

    # 4. White Athletic Tennis Mini-Skirt
    # Waistband
    skirt_waist_z = 0.98 * scale
    parts.append(
        add_cylinder(
            "SkirtWaistband",
            1.0,
            0.05 * scale,
            (0.0, 0.0, skirt_waist_z),
            white_fabric_mat,
            scale=(0.152 * scale, 0.098 * scale, 1.0),
        )
    )
    # Flared Skirt Body (A-line)
    skirt_z = 0.83 * scale
    skirt_h = 0.26 * scale
    parts.append(
        add_cone(
            "SkirtBody",
            0.195 * scale,
            0.155 * scale,
            skirt_h,
            (0.0, 0.0, skirt_z),
            white_fabric_mat,
            scale=(1.0, 0.78, 1.0),
        )
    )

    # Navy logo on skirt hem
    parts.append(
        add_cube(
            "SkirtNavyLogo",
            1.0,
            (-0.13 * scale, -0.105 * scale, 0.72 * scale),
            navy_logo_mat,
            scale=(0.01 * scale, 0.003 * scale, 0.014 * scale),
        )
    )

    # 5. Toned Legs & Black Combat Boots
    for side in [-1.0, 1.0]:
        # Bare leg from skirt hem to combat boots
        leg_z = 0.48 * scale
        leg_h = 0.48 * scale
        parts.append(
            add_cylinder(
                f"LegSkin_{side}",
                0.055 * scale,
                leg_h,
                (side * 0.08 * scale, 0.0, leg_z),
                skin_mat,
            )
        )

        # Combat Boot Shaft (Black leather ankle boot)
        boot_shaft_z = 0.14 * scale
        boot_shaft_h = 0.19 * scale
        parts.append(
            add_cylinder(
                f"CombatBootShaft_{side}",
                0.058 * scale,
                boot_shaft_h,
                (side * 0.08 * scale, 0.0, boot_shaft_z),
                combat_boot_mat,
            )
        )

        # Laced Front Tongue
        parts.append(
            add_cube(
                f"BootTongue_{side}",
                1.0,
                (side * 0.08 * scale, -0.048 * scale, boot_shaft_z),
                sole_mat,
                scale=(0.045 * scale, 0.012 * scale, boot_shaft_h * 0.85),
            )
        )

        # Combat Boot Foot & Rugged Lugged Sole
        parts.append(
            add_cube(
                f"CombatBootFoot_{side}",
                1.0,
                (side * 0.08 * scale, -0.035 * scale, 0.035 * scale),
                combat_boot_mat,
                scale=(0.085 * scale, 0.20 * scale, 0.065 * scale),
            )
        )

        # Rugged Sole Base
        parts.append(
            add_cube(
                f"CombatBootSole_{side}",
                1.0,
                (side * 0.08 * scale, -0.035 * scale, 0.012 * scale),
                sole_mat,
                scale=(0.092 * scale, 0.21 * scale, 0.024 * scale),
            )
        )

    human = join_objects(parts, "GemK_Model")
    human.location = (0.0, 0.0, 0.0)
    bpy.ops.object.shade_smooth()
    return human


# ---------------------------------------------------------------------------
# Generic Fallback Humanoid
# ---------------------------------------------------------------------------


def build_generic_humanoid(
    height_cm: float, build_multiplier: float, skin_mat, hair_mat, eye_mat
):
    height_m = height_cm / 100.0
    leg_length = height_m * 0.48
    torso_height = height_m * 0.30
    arm_length = height_m * 0.44
    head_radius = height_m * 0.065
    neck_height = height_m * 0.04

    torso_radius = 0.09 * build_multiplier
    leg_radius = 0.045 * build_multiplier
    arm_radius = 0.035 * build_multiplier
    neck_radius = leg_radius * 0.6
    shoulder_half_width = torso_radius + arm_radius + 0.02
    hip_offset = torso_radius * 0.75

    skin_parts = []
    skin_parts.append(
        add_cylinder(
            "LeftLeg",
            leg_radius,
            leg_length,
            (hip_offset, 0.0, leg_length / 2.0),
            skin_mat,
        )
    )
    skin_parts.append(
        add_cylinder(
            "RightLeg",
            leg_radius,
            leg_length,
            (-hip_offset, 0.0, leg_length / 2.0),
            skin_mat,
        )
    )
    skin_parts.append(
        add_cylinder(
            "Torso",
            torso_radius,
            torso_height,
            (0.0, 0.0, leg_length + torso_height / 2.0),
            skin_mat,
        )
    )
    skin_parts.append(
        add_cylinder(
            "LeftArm",
            arm_radius,
            arm_length,
            (
                shoulder_half_width,
                0.0,
                leg_length + torso_height - arm_length / 2.0,
            ),
            skin_mat,
        )
    )
    skin_parts.append(
        add_cylinder(
            "RightArm",
            arm_radius,
            arm_length,
            (
                -shoulder_half_width,
                0.0,
                leg_length + torso_height - arm_length / 2.0,
            ),
            skin_mat,
        )
    )
    skin_parts.append(
        add_cylinder(
            "Neck",
            neck_radius,
            neck_height,
            (0.0, 0.0, leg_length + torso_height + neck_height / 2.0),
            skin_mat,
        )
    )

    head_z = leg_length + torso_height + neck_height + head_radius
    skin_parts.append(
        add_sphere("Head", head_radius, (0.0, 0.0, head_z), skin_mat)
    )

    hair = add_sphere(
        "HairCap",
        head_radius * 1.02,
        (0.0, 0.0, head_z + head_radius * 0.10),
        hair_mat,
        scale=(1.0, 1.0, 0.9),
    )
    hair.rotation_euler[0] = math.radians(6.0)

    eye_radius = head_radius * 0.12
    eye_y = -head_radius * 0.82
    eye_z = head_z + head_radius * 0.05
    left_eye = add_sphere(
        "LeftEye", eye_radius, (head_radius * 0.35, eye_y, eye_z), eye_mat
    )
    right_eye = add_sphere(
        "RightEye", eye_radius, (-head_radius * 0.35, eye_y, eye_z), eye_mat
    )

    human = join_objects(skin_parts + [hair, left_eye, right_eye], "HumanModel")
    human.location = (0.0, 0.0, 0.0)
    bpy.ops.object.shade_smooth()
    return human


# ---------------------------------------------------------------------------
# CLI Entrypoint
# ---------------------------------------------------------------------------


def main() -> None:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--skin-tone", default="olive")
    parser.add_argument("--character", default="auto", choices=["auto", "gem-d", "gem-k", "generic"])
    args = parser.parse_args(argv)

    with open(args.input, "r", encoding="utf-8") as handle:
        fixture = json.load(handle)
    appearance = fixture.get("appearance", {})

    output_path = pathlib.Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    char_choice = args.character.lower()
    if char_choice == "auto":
        out_name = output_path.stem.lower()
        agent_id = fixture.get("agent_id", "").lower()
        if "gemd" in out_name or "gem-d" in out_name or "analytical" in agent_id:
            char_choice = "gem-d"
        elif "gemk" in out_name or "gem-k" in out_name or "intuitive" in agent_id:
            char_choice = "gem-k"
        else:
            char_choice = "generic"

    reset_scene()

    # Base materials
    skin_mat = make_material("Skin", resolve_color(args.skin_tone), roughness=0.55)
    hair_mat = make_material("Hair", resolve_color(appearance.get("hairColor", "")), roughness=0.75)
    eye_mat = make_material("Eyes", resolve_color(appearance.get("eyeColor", "")), roughness=0.2)

    height_val = float(appearance.get("height", 170.0))

    if char_choice == "gem-d":
        gold_mat = make_material("Gold_Accent", (0.92, 0.76, 0.22), roughness=0.25, metallic=0.95)
        black_fabric_mat = make_material("Black_Fabric", (0.03, 0.03, 0.03), roughness=0.75)
        black_rubber_mat = make_material("Black_Rubber", (0.04, 0.04, 0.04), roughness=0.5)
        red_rubber_mat = make_material("Red_Rubber", (0.85, 0.20, 0.12), roughness=0.45)
        sunglasses_mat = make_material("Sunglasses_Glass", (0.015, 0.015, 0.015), roughness=0.08, transmission=0.1)

        build_gem_d_model(
            height_val,
            skin_mat,
            hair_mat,
            eye_mat,
            gold_mat,
            black_fabric_mat,
            black_rubber_mat,
            red_rubber_mat,
            sunglasses_mat,
        )
    elif char_choice == "gem-k":
        white_fabric_mat = make_material("White_Fabric", (0.92, 0.92, 0.92), roughness=0.65)
        navy_logo_mat = make_material("Navy_Logo", (0.05, 0.08, 0.25), roughness=0.5)
        combat_boot_mat = make_material("Combat_Boot", (0.05, 0.05, 0.05), roughness=0.6)
        sole_mat = make_material("Boot_Sole", (0.02, 0.02, 0.02), roughness=0.85)

        build_gem_k_model(
            height_val,
            skin_mat,
            hair_mat,
            eye_mat,
            white_fabric_mat,
            navy_logo_mat,
            combat_boot_mat,
            sole_mat,
        )
    else:
        build_generic_humanoid(
            height_val,
            BUILD_MULTIPLIERS.get(appearance.get("build", "").lower(), DEFAULT_BUILD_MULTIPLIER),
            skin_mat,
            hair_mat,
            eye_mat,
        )

    bpy.ops.export_scene.gltf(
        filepath=str(output_path),
        export_format="GLB",
        export_yup=True,
        use_selection=False,
    )
    print(f"Exported {output_path}")


if __name__ == "__main__":
    main()

