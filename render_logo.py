"""
Silicon Logo — Static render of the glass trefoil knot.
Renders a single high-res PNG from the same trefoil as silicon_logo_final.py.

Usage:
  /Applications/Blender.app/Contents/MacOS/Blender --background --python render_logo.py

Output: silicon_logo.png (1024x1024, transparent background)
"""

import bpy
import bmesh
import math

RESOLUTION = 1024
SAMPLES = 256
OUTPUT = "/Users/rani/code/Rust/silicon/silicon_logo.png"

# ── Clean scene ──────────────────────────────────────────────────────────────
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete()
for block in bpy.data.materials:
    bpy.data.materials.remove(block)
for block in bpy.data.meshes:
    bpy.data.meshes.remove(block)

# ── Glass material ───────────────────────────────────────────────────────────
mat = bpy.data.materials.new("SiliconGlass")
mat.use_nodes = True
nodes = mat.node_tree.nodes
links = mat.node_tree.links
nodes.clear()

output_node = nodes.new('ShaderNodeOutputMaterial')
mix = nodes.new('ShaderNodeMixShader')
mix.inputs['Fac'].default_value = 0.1

glass = nodes.new('ShaderNodeBsdfGlass')
glass.inputs['Color'].default_value = (0.93, 0.95, 0.98, 1.0)
glass.inputs['Roughness'].default_value = 0.05
glass.inputs['IOR'].default_value = 1.45

emit = nodes.new('ShaderNodeEmission')
emit.inputs['Color'].default_value = (0.85, 0.88, 1.0, 1.0)
emit.inputs['Strength'].default_value = 0.3

links.new(glass.outputs['BSDF'], mix.inputs[1])
links.new(emit.outputs['Emission'], mix.inputs[2])
links.new(mix.outputs['Shader'], output_node.inputs['Surface'])

# ── Trefoil knot ─────────────────────────────────────────────────────────────
VERT_COUNT = 384

def trefoil_coords(p=2, q=3, scale=0.45):
    coords = []
    for i in range(VERT_COUNT):
        t = 2 * math.pi * i / VERT_COUNT
        r = math.cos(q * t) + 2
        x = r * math.cos(p * t) * scale
        y = r * math.sin(p * t) * scale
        z = -math.sin(q * t) * scale
        coords.append((x, y, z))
    return coords

base_coords = trefoil_coords()
mesh_data = bpy.data.meshes.new("Trefoil")
bm = bmesh.new()
for co in base_coords:
    bm.verts.new(co)
bm.verts.ensure_lookup_table()
for i in range(VERT_COUNT):
    bm.edges.new((bm.verts[i], bm.verts[(i + 1) % VERT_COUNT]))
bm.to_mesh(mesh_data)
bm.free()

obj = bpy.data.objects.new("SiliconTrefoil", mesh_data)
bpy.context.collection.objects.link(obj)
bpy.context.view_layer.objects.active = obj
obj.select_set(True)

# Skin modifier for tube volume
mod = obj.modifiers.new("Skin", 'SKIN')
for v in obj.data.skin_vertices[0].data:
    v.radius = (0.14, 0.14)

# Subdivision for smoothness
mod2 = obj.modifiers.new("Subsurf", 'SUBSURF')
mod2.levels = 2
mod2.render_levels = 3

obj.data.materials.append(mat)

# ── Lighting ─────────────────────────────────────────────────────────────────
bpy.ops.object.light_add(type='AREA', location=(0, -4, 4))
key = bpy.context.active_object
key.data.energy = 600
key.data.size = 6
key.rotation_euler = (math.radians(45), 0, 0)

bpy.ops.object.light_add(type='AREA', location=(4, 2, 2))
fill = bpy.context.active_object
fill.data.energy = 250
fill.data.size = 4
fill.rotation_euler = (math.radians(-20), math.radians(30), 0)

bpy.ops.object.light_add(type='AREA', location=(-3, 3, 3))
rim = bpy.context.active_object
rim.data.energy = 350
rim.data.size = 4
rim.rotation_euler = (math.radians(-40), math.radians(-20), 0)

# ── Camera ───────────────────────────────────────────────────────────────────
bpy.ops.object.camera_add(location=(0, -5.5, 1.0))
cam = bpy.context.active_object
cam.rotation_euler = (math.radians(80), 0, 0)
cam.data.lens = 70
bpy.context.scene.camera = cam

# ── Render settings ──────────────────────────────────────────────────────────
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = SAMPLES
scene.render.resolution_x = RESOLUTION
scene.render.resolution_y = RESOLUTION
scene.render.film_transparent = True
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGBA'
scene.render.filepath = OUTPUT

scene.cycles.caustics_reflective = True
scene.cycles.caustics_refractive = True

# Use GPU if available
prefs = bpy.context.preferences.addons.get('cycles')
if prefs:
    prefs.preferences.compute_device_type = 'METAL'
    scene.cycles.device = 'GPU'

# ── Render ───────────────────────────────────────────────────────────────────
bpy.ops.render.render(write_still=True)
print(f"\n✅ Rendered to {OUTPUT}")
