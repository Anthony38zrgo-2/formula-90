import bpy
import hashlib
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path
from mathutils import Vector

project_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(project_directory / 'tools'))
from common.output_policy import validate_output_path
source_path = project_directory / 'game/assets/models/vehicles/f1-2030/source/f1_2030.blend'
output_directory = validate_output_path(project_directory,'scratch/vehicle_finish_and_studio','preview').path
output_directory.mkdir(parents=True,exist_ok=True)
backup_directory = validate_output_path(project_directory, output_directory / 'backup', 'preview').path
texture_directory = source_path.parent / 'textures/baked_physical_materials'
assert Path(bpy.data.filepath).resolve()==source_path.resolve()
assert not bpy.data.scenes.get('FinalVehicleStudio'),'Studio already exists; review before applying finishes again'
backup_directory.mkdir(parents=True,exist_ok=True)
source_digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
backup_path = backup_directory / ('before_finish_and_studio_'+source_digest[:12]+'.blend')
shutil.copy2(source_path,backup_path)
shutil.copytree(texture_directory,backup_directory / 'baked_physical_materials',dirs_exist_ok=True)
report = {'branch':subprocess.check_output(['git','branch','--show-current'],cwd=project_directory,text=True).strip(),'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=project_directory,text=True).strip(),'status_before':subprocess.check_output(['git','status','--short'],cwd=project_directory,text=True),'source_before_sha256':source_digest,'backup':str(backup_path)}

def surface_digest(scene_object):
    digest = hashlib.sha256()
    digest.update(struct.pack('<16d',*(value for row in scene_object.matrix_world for value in row)))
    for vertex in scene_object.data.vertices:
        digest.update(struct.pack('<3f',*vertex.co))
    for polygon in scene_object.data.polygons:
        digest.update(struct.pack('<'+'I'*len(polygon.vertices),*polygon.vertices))
        digest.update(struct.pack('<I',polygon.material_index))
    for coordinates in scene_object.data.uv_layers:
        for coordinate in coordinates.data:
            digest.update(struct.pack('<2f',*coordinate.uv))
    return digest.hexdigest()

vehicle_objects = list(bpy.context.scene.objects)
protected_surfaces = {scene_object.name:surface_digest(scene_object) for scene_object in vehicle_objects if scene_object.type=='MESH'}
manifest_path = texture_directory / 'baking_manifest.json'
manifest = json.loads(manifest_path.read_text())
authoring_scene = bpy.context.scene
authoring_scene.name = 'VehicleAuthoring'
authoring_scene.render.engine = 'CYCLES'
authoring_scene.cycles.samples = 16
authoring_scene.render.bake.margin = 12
authoring_scene.render.bake.use_selected_to_active = False
authoring_scene.render.bake.use_clear = True
report['finish_variations'] = []
for family_name,material_name,color_variation,roughness_variation,pattern_scale in (('red_lacquer','BakedPhysicalRedLacquer',0.012,0.016,1400),('satin_carbon','BakedPhysicalSatinCarbon',0.04,0.05,450)):
    material = bpy.data.materials[material_name]
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    shader = next(node for node in nodes if node.type=='BSDF_PRINCIPLED')
    output = next(node for node in nodes if node.type=='OUTPUT_MATERIAL')
    color_source = shader.inputs['Base Color'].links[0].from_socket
    roughness_source = shader.inputs['Roughness'].links[0].from_socket
    separation = roughness_source.node
    group = next(group for group in manifest['groups'] if group['family']==family_name)
    vertices = []
    faces = []
    coordinates = []
    for scene_object in vehicle_objects:
        if scene_object.type!='MESH' or material not in list(scene_object.data.materials):
            continue
        offset = len(vertices)
        vertices.extend(scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices)
        for polygon in scene_object.data.polygons:
            if scene_object.data.materials[polygon.material_index]!=material:
                continue
            faces.append(tuple(offset+index for index in polygon.vertices))
            coordinates.extend(scene_object.data.uv_layers['BakedPhysicalMaterialCoordinates'].data[index].uv.copy() for index in polygon.loop_indices)
    mesh = bpy.data.meshes.new('FinishVariationBakeSurface')
    mesh.from_pydata(vertices,[],faces)
    mesh.materials.append(material)
    texture_coordinates = mesh.uv_layers.new(name='BakedPhysicalMaterialCoordinates')
    texture_coordinates.active_render = True
    for index,coordinate in enumerate(coordinates):
        texture_coordinates.data[index].uv = coordinate
    temporary_object = bpy.data.objects.new('FinishVariationBakeSurface',mesh)
    authoring_scene.collection.objects.link(temporary_object)
    bpy.ops.object.select_all(action='DESELECT')
    temporary_object.select_set(True)
    bpy.context.view_layer.objects.active = temporary_object
    previous_nodes = set(nodes)
    position = nodes.new('ShaderNodeNewGeometry')
    broad_noise = nodes.new('ShaderNodeTexNoise')
    broad_noise.inputs['Scale'].default_value = 6
    broad_noise.inputs['Detail'].default_value = 2
    links.new(position.outputs['Position'],broad_noise.inputs['Vector'])
    fine_noise = nodes.new('ShaderNodeTexNoise')
    fine_noise.inputs['Scale'].default_value = pattern_scale
    fine_noise.inputs['Detail'].default_value = 2
    links.new(position.outputs['Position'],fine_noise.inputs['Vector'])
    color_factor = nodes.new('ShaderNodeMath')
    color_factor.operation = 'MULTIPLY_ADD'
    links.new(broad_noise.outputs['Fac'],color_factor.inputs[0])
    color_factor.inputs[1].default_value = color_variation
    color_factor.inputs[2].default_value = 1-color_variation*0.5
    color_multiply = nodes.new('ShaderNodeMixRGB')
    color_multiply.blend_type = 'MULTIPLY'
    color_multiply.inputs[0].default_value = 1
    links.new(color_source,color_multiply.inputs[1])
    links.new(color_factor.outputs[0],color_multiply.inputs[2])
    roughness_offset = nodes.new('ShaderNodeMath')
    roughness_offset.operation = 'MULTIPLY_ADD'
    links.new(fine_noise.outputs['Fac'],roughness_offset.inputs[0])
    roughness_offset.inputs[1].default_value = roughness_variation
    roughness_offset.inputs[2].default_value = -roughness_variation*0.5
    roughness_sum = nodes.new('ShaderNodeMath')
    roughness_sum.operation = 'ADD'
    roughness_sum.use_clamp = True
    links.new(roughness_source,roughness_sum.inputs[0])
    links.new(roughness_offset.outputs[0],roughness_sum.inputs[1])
    combined = nodes.new('ShaderNodeCombineColor')
    links.new(separation.outputs['Red'],combined.inputs['Red'])
    links.new(roughness_sum.outputs[0],combined.inputs['Green'])
    links.new(separation.outputs['Blue'],combined.inputs['Blue'])
    emission = nodes.new('ShaderNodeEmission')
    destination = nodes.new('ShaderNodeTexImage')
    nodes.active = destination
    links.new(emission.outputs[0],output.inputs['Surface'])
    generated_images = []
    for output_name,socket in (('base_color',color_multiply.outputs[0]),('occlusion_roughness_metallic',combined.outputs[0])):
        generated_image = bpy.data.images.new('FinishVariationDestination',width=group['resolution'],height=group['resolution'],alpha=False)
        generated_image.colorspace_settings.name = 'sRGB' if output_name=='base_color' else 'Non-Color'
        destination.image = generated_image
        links.new(socket,emission.inputs['Color'])
        bpy.ops.object.bake(type='EMIT',uv_layer='BakedPhysicalMaterialCoordinates')
        generated_images.append((output_name,generated_image))
        print('FINISH_VARIATION_BAKED='+family_name+'_'+output_name,flush=True)
    links.new(shader.outputs[0],output.inputs['Surface'])
    for output_name,generated_image in generated_images:
        destination_path = validate_output_path(project_directory,texture_directory / (family_name+'_'+output_name+'.png'),'promote').path
        generated_image.filepath_raw = str(destination_path)
        generated_image.file_format = 'PNG'
        generated_image.save()
        bpy.data.images[family_name+'_'+output_name].reload()
        group['maps'][output_name]['sha256'] = hashlib.sha256(destination_path.read_bytes()).hexdigest()
    for node in list(nodes):
        if node not in previous_nodes:
            nodes.remove(node)
    for output_name,generated_image in generated_images:
        bpy.data.images.remove(generated_image)
    bpy.data.objects.remove(temporary_object,do_unlink=True)
    bpy.data.meshes.remove(mesh)
    shader.inputs['Coat Weight'].default_value = 0.42 if family_name=='red_lacquer' else 0.12
    shader.inputs['Coat Roughness'].default_value = 0.14 if family_name=='red_lacquer' else 0.28
    report['finish_variations'].append({'family':family_name,'color_peak_to_peak':color_variation,'roughness_peak_to_peak':roughness_variation,'microtexture_scale_per_meter':pattern_scale,'maps_rebaked':['base_color','occlusion_roughness_metallic'],'normal_map_preserved':True})

studio_scene = bpy.data.scenes.new('FinalVehicleStudio')
for scene_object in vehicle_objects:
    studio_scene.collection.objects.link(scene_object)
studio_collection = bpy.data.collections.new('StudioLightingAndCameras')
studio_scene.collection.children.link(studio_collection)
world = bpy.data.worlds.new('NeutralStudioEnvironment')
world.use_nodes = True
world.node_tree.nodes['Background'].inputs['Color'].default_value = (0.16,0.17,0.19,1)
world.node_tree.nodes['Background'].inputs['Strength'].default_value = 0.25
studio_scene.world = world
floor_mesh = bpy.data.meshes.new('StudioGroundSurface')
floor_mesh.from_pydata([(-15,-15,-0.463),(15,-15,-0.463),(15,15,-0.463),(-15,15,-0.463)],[],[(0,1,2,3)])
floor_object = bpy.data.objects.new('StudioGround',floor_mesh)
studio_collection.objects.link(floor_object)
floor_material = bpy.data.materials.new('NeutralMatteStudioGround')
floor_material.use_nodes = True
floor_shader = floor_material.node_tree.nodes.get('Principled BSDF')
floor_shader.inputs['Base Color'].default_value = (0.055,0.06,0.07,1)
floor_shader.inputs['Roughness'].default_value = 0.72
floor_mesh.materials.append(floor_material)
light_records = []
for name,position,power,width,height in (('StudioKeySoftbox',(-3.5,-4.5,5.5),1100,5,3),('StudioFillSoftbox',(-1,4,3.5),650,4,3),('StudioRearContourSoftbox',(4,-1.5,4),1000,3,2),('StudioOverheadStrip',(0,0,6),700,6,1)):
    light = bpy.data.lights.new(name,'AREA')
    light.energy = power
    light.shape = 'RECTANGLE'
    light.size = width
    light.size_y = height
    scene_object = bpy.data.objects.new(name,light)
    studio_collection.objects.link(scene_object)
    scene_object.location = position
    scene_object.rotation_euler = (Vector((0,0,0.1))-scene_object.location).to_track_quat('-Z','Y').to_euler()
    light_records.append({'name':name,'position':position,'power_watts':power,'dimensions_meters':[width,height]})
camera_records = []
for name,position,target,scale in (('StudioFrontThreeQuarter',(-4.8,-6,3.2),(0,0,0.05),6.7),('StudioRearThreeQuarter',(4.8,-6,2.8),(0,0,0.05),6.7),('StudioSide',(0,-8,0.5),(0,0,0.05),6.4),('StudioTop',(0,0,8),(0,0,0),6.4)):
    camera = bpy.data.cameras.new(name)
    camera.type = 'ORTHO'
    camera.ortho_scale = scale
    scene_object = bpy.data.objects.new(name,camera)
    studio_collection.objects.link(scene_object)
    scene_object.location = position
    scene_object.rotation_euler = (Vector(target)-scene_object.location).to_track_quat('-Z','Y').to_euler()
    camera_records.append({'name':name,'position':position,'target':target,'orthographic_scale':scale})
studio_scene.camera = bpy.data.objects['StudioFrontThreeQuarter']
studio_scene.render.engine = 'CYCLES'
studio_scene.cycles.device = 'CPU'
studio_scene.cycles.samples = 512
studio_scene.cycles.use_adaptive_sampling = True
studio_scene.cycles.adaptive_threshold = 0.005
studio_scene.cycles.adaptive_min_samples = 32
studio_scene.cycles.use_denoising = True
studio_scene.cycles.denoiser = 'OPENIMAGEDENOISE'
studio_scene.cycles.max_bounces = 10
studio_scene.cycles.diffuse_bounces = 4
studio_scene.cycles.glossy_bounces = 6
studio_scene.cycles.transmission_bounces = 8
studio_scene.cycles.transparent_max_bounces = 8
studio_scene.cycles.sample_clamp_indirect = 3
studio_scene.render.resolution_x = 3840
studio_scene.render.resolution_y = 2160
studio_scene.render.resolution_percentage = 100
studio_scene.render.image_settings.file_format = 'PNG'
studio_scene.render.image_settings.color_mode = 'RGBA'
studio_scene.render.image_settings.color_depth = '16'
studio_scene.render.film_transparent = False
studio_scene.view_settings.view_transform = 'AgX'
studio_scene.view_settings.look = 'AgX - Medium High Contrast'
studio_scene.view_settings.exposure = -0.3
studio_scene.render.filepath = str(output_directory / 'final_vehicle_render.png')
preset = {'scene':studio_scene.name,'engine':'CYCLES','device':'CPU','samples':512,'adaptive_threshold':0.005,'denoiser':'OPENIMAGEDENOISE','resolution':[3840,2160],'format':'PNG','color_depth':16,'view_transform':'AgX','look':'AgX - Medium High Contrast','exposure':-0.3,'lights':light_records,'cameras':camera_records,'active_camera':studio_scene.camera.name}
studio_scene['FinalRenderPreset'] = json.dumps(preset)
text = bpy.data.texts.new('FinalVehicleRenderPreset.json')
text.write(json.dumps(preset,indent=2))
for name,digest in protected_surfaces.items():
    assert surface_digest(bpy.data.objects[name])==digest,name
manifest['subtle_finish_variations'] = report['finish_variations']
validate_output_path(project_directory,manifest_path,'promote').path.write_text(json.dumps(manifest,indent=2),encoding='utf-8')
bpy.context.window.scene = studio_scene
bpy.context.preferences.filepaths.save_version = 0
assert hashlib.sha256(source_path.read_bytes()).hexdigest()==source_digest
bpy.ops.wm.save_as_mainfile(filepath=str(validate_output_path(project_directory,source_path,'promote').path))
bpy.ops.wm.open_mainfile(filepath=str(source_path))
for name,digest in protected_surfaces.items():
    assert surface_digest(bpy.data.objects[name])==digest,name
studio_scene = bpy.data.scenes['FinalVehicleStudio']
assert studio_scene.camera.name=='StudioFrontThreeQuarter'
assert studio_scene.cycles.samples==512 and studio_scene.render.resolution_x==3840
assert len([scene_object for scene_object in studio_scene.objects if scene_object.type=='LIGHT'])==4
assert not any(scene_object.name.startswith('Studio') for scene_object in bpy.data.scenes['VehicleAuthoring'].objects)
for group in manifest['groups']:
    for output_name,record in group['maps'].items():
        texture_image = bpy.data.images[group['family']+'_'+output_name]
        assert tuple(texture_image.size)==(group['resolution'],group['resolution']),texture_image.name
        assert hashlib.sha256(Path(record['path']).read_bytes()).hexdigest()==record['sha256']
report.update({'source_after_sha256':hashlib.sha256(source_path.read_bytes()).hexdigest(),'protected_meshes':len(protected_surfaces),'geometry_and_texture_coordinates_unchanged':True,'studio_separate_from_authoring':True,'saved_source_reopened_and_verified':True,'render_preset':preset})
validate_output_path(project_directory,output_directory / 'finish_and_studio_report.json','preview').path.write_text(json.dumps(report,indent=2),encoding='utf-8')
print('FINISHES_AND_STUDIO_SAVED_AND_VERIFIED',flush=True)
studio_scene.cycles.samples = 64
studio_scene.cycles.adaptive_threshold = 0.02
studio_scene.render.resolution_x = 1600
studio_scene.render.resolution_y = 900
studio_scene.render.image_settings.file_format = 'JPEG'
studio_scene.render.image_settings.color_mode = 'RGB'
studio_scene.render.image_settings.quality = 97
studio_scene.render.filepath = str(output_directory / 'studio_finish_review.jpg')
bpy.ops.render.render(write_still=True,scene=studio_scene.name)
