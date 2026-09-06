"""Render the actual exported runtime assets; no source/GLB writes."""
import bpy,math,json,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/'game/assets/models/vehicles/f1-2026-2008'
REPORT=ROOT/'reports/2026-3500-jordan'
physics=json.loads((ROOT/'game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json').read_text())
halfbase=physics['geometry']['wheelbase']/2
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(ASSETS/'f1_2026_2008_chassis.glb'))
for axle,half,y in [('front',physics['geometry']['front_track']/2,halfbase),('rear',physics['geometry']['rear_track']/2,-halfbase)]:
    for side in [-1,1]:
        before=set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(ASSETS/f'f1_2026_2008_wheel_{axle}.glb'))
        for o in set(bpy.data.objects)-before:
            o.rotation_mode='XYZ'
            if side<0:
                o.location.x=-o.location.x; o.location.y=-o.location.y; o.rotation_euler.z=math.pi
            o.location+=Vector((side*half,y,0))
scene=bpy.context.scene; scene.render.engine='CYCLES'; scene.cycles.samples=24
scene.cycles.use_denoising=True; scene.cycles.device='CPU'
scene.world=bpy.data.worlds.new('StudioWorld'); scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.22,.25,.3,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.45
bpy.ops.mesh.primitive_plane_add(size=200,location=(0,0,-.33))
ground=bpy.context.object; m=bpy.data.materials.new('StudioGrey'); m.use_nodes=True
m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=(.14,.16,.18,1)
m.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.9; ground.data.materials.append(m)
for name,position,power,size in [('Key',(3,4,7),1800,5),('Fill',(-4,1,4),1100,5),('Rim',(1,-5,5),2000,4)]:
    light=bpy.data.objects.new(name,bpy.data.lights.new(name,'AREA')); scene.collection.objects.link(light)
    light.location=position; light.data.energy=power; light.data.shape='DISK'; light.data.size=size
    light.rotation_euler=(-light.location).to_track_quat('-Z','Y').to_euler()
camera=bpy.data.objects.new('ReviewCamera',bpy.data.cameras.new('ReviewCamera')); scene.collection.objects.link(camera); scene.camera=camera
camera.data.type='ORTHO'
scene.render.resolution_x=1500; scene.render.resolution_y=1050; scene.render.resolution_percentage=100
scene.view_settings.view_transform='Standard'; scene.view_settings.exposure=-1.3
for name,position,target,scale in [('front_three_quarter',(4,6,3.5),(0,.18,.08),5.65),
    ('rear_three_quarter',(-4,-6,3.3),(0,-.1,.07),5.65),('side',(7,0,1.7),(0,.2,.10),5.6),
    ('top',(0,.2,8),(0,.2,0),7.2)]:
    camera.location=position; camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale; scene.render.filepath=str(REPORT/(name+'.png'))
    bpy.ops.render.render(write_still=True)
