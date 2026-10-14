import bpy
import numpy
import sys
import json
import copy
import hashlib
import shutil
from pathlib import Path
from mathutils import Matrix
import argparse
repository_directory=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(repository_directory/'tools'))
sys.path.insert(0,str(repository_directory/'tools/blender'))
from common.output_policy import validate_output_path
from glb_util import read_glb,write_glb
parser=argparse.ArgumentParser()
parser.add_argument('--output-directory',type=Path,required=True)
parser.add_argument('--baseline-directory',type=Path,default=repository_directory/'game/assets/models/vehicles/f1-2030')
parser.add_argument('--source-file',type=Path,default=repository_directory/'game/assets/models/vehicles/f1-2030/source/f1_2030.blend')
arguments=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
output_directory=validate_output_path(repository_directory,arguments.output_directory,'preview').path
output_directory.mkdir(parents=True,exist_ok=True)
asset_directory=arguments.baseline_directory.resolve()
source_path=arguments.source_file.resolve()
source_digest=hashlib.sha256(source_path.read_bytes()).hexdigest()
manifest=json.loads((asset_directory/'manifest.json').read_text())
document,buffer=read_glb(asset_directory/'f1_2030_v10_chassis.glb')
original_document=copy.deepcopy(document);original_buffer=bytes(buffer)
bpy.ops.wm.open_mainfile(filepath=str(source_path))
scene=bpy.context.scene
bodywork_objects=[scene_object for scene_object in scene.objects if scene_object.type=='MESH' and scene_object.name.startswith('Bodywork')]
conversion=Matrix.Translation((0,0,manifest['authored_alignment']['chassis_vertical_offset']))@Matrix.Rotation(-1.5707963267948966,4,'Z')
def position_key(position):return tuple(round(float(value),4) for value in position)
def accessor_values(index):
 record=document['accessors'][index];view=document['bufferViews'][record['bufferView']];width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[record['type']];dtype={5126:'<f4',5125:'<u4',5123:'<u2',5121:'u1'}[record['componentType']]
 return numpy.ndarray((record['count'],width),dtype=dtype,buffer=buffer,offset=view.get('byteOffset',0)+record.get('byteOffset',0),strides=(view.get('byteStride',numpy.dtype(dtype).itemsize*width),numpy.dtype(dtype).itemsize)).copy()
def append_accessor(values,accessor_type,component_type=5126):
 dtype='<f4' if component_type==5126 else '<u4';values=numpy.asarray(values,dtype=dtype);encoded=values.tobytes();buffer.extend(b'\0'*((-len(buffer))%4));offset=len(buffer);buffer.extend(encoded)
 view_index=len(document['bufferViews']);document['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':len(encoded)})
 record={'bufferView':view_index,'componentType':component_type,'count':len(values),'type':accessor_type}
 if accessor_type=='VEC3':record['min']=values.min(axis=0).tolist();record['max']=values.max(axis=0).tolist()
 index=len(document['accessors']);document['accessors'].append(record);return index
records=[]
for scene_object in bodywork_objects:
 bpy.ops.object.select_all(action='DESELECT');scene_object.hide_set(False);scene_object.select_set(True);bpy.context.view_layer.objects.active=scene_object
 original_mesh=scene_object.data
 clean_mesh=bpy.data.meshes.new(scene_object.name+'SponsorExportSurface')
 clean_mesh.from_pydata([tuple(vertex.co) for vertex in original_mesh.vertices],[],[tuple(polygon.vertices) for polygon in original_mesh.polygons])
 for material in original_mesh.materials:clean_mesh.materials.append(material)
 for original_polygon,polygon in zip(original_mesh.polygons,clean_mesh.polygons):polygon.material_index=original_polygon.material_index;polygon.use_smooth=original_polygon.use_smooth
 for original_coordinates in original_mesh.uv_layers:
  copied_coordinates=clean_mesh.uv_layers.new(name=original_coordinates.name)
  for original_coordinate,copied_coordinate in zip(original_coordinates.data,copied_coordinates.data):copied_coordinate.uv=original_coordinate.uv
 scene_object.data=clean_mesh
 print('PREPARING',scene_object.name,flush=True)
 coordinates=scene_object.data.uv_layers.new(name='UpdatedSponsorExportCoordinates');scene_object.data.uv_layers.active=coordinates;coordinates.active_render=True
 print('PROJECTING',scene_object.name,flush=True)
 bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT');bpy.ops.uv.smart_project(angle_limit=1.1519173063,island_margin=.008);bpy.ops.object.mode_set(mode='OBJECT')
 coordinates=scene_object.data.uv_layers['UpdatedSponsorExportCoordinates']
 image=bpy.data.images.new(scene_object.name+'UpdatedSponsorAtlas',width=2048,height=2048,alpha=True);image.colorspace_settings.name='sRGB'
 from rasterize_formula_one_2030_sponsor_colors import rasterize_bodywork
 parameters,normal_pixels=rasterize_bodywork(scene_object,image)
 additional_textures={}
 for texture_role,pixels in [('parameters',parameters),('normal',normal_pixels)]:
  extra_image=bpy.data.images.new(scene_object.name+texture_role,width=2048,height=2048,alpha=True);extra_image.colorspace_settings.name='Non-Color';extra_image.pixels.foreach_set(pixels.ravel())
  extra_path=output_directory/(scene_object.name+'_'+texture_role+'.png');validate_output_path(repository_directory,extra_path,'preview');extra_image.filepath_raw=str(extra_path);extra_image.file_format='PNG';extra_image.save()
  encoded=extra_path.read_bytes();buffer.extend(b'\0'*((-len(buffer))%4));offset=len(buffer);buffer.extend(encoded)
  extra_view=len(document['bufferViews']);document['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':len(encoded)})
  extra_image_index=len(document['images']);document['images'].append({'bufferView':extra_view,'mimeType':'image/png','name':extra_image.name})
  extra_texture_index=len(document['textures']);document['textures'].append({'source':extra_image_index,'sampler':0});additional_textures[texture_role]=extra_texture_index
 image_path=output_directory/(scene_object.name+'_updated_sponsors.png');validate_output_path(repository_directory,image_path,'preview');image.filepath_raw=str(image_path);image.file_format='PNG';image.save()
 scene_object.data.calc_loop_triangles();triangle_coordinates={}
 for triangle in scene_object.data.loop_triangles:
  positions=[conversion@scene_object.matrix_world@scene_object.data.vertices[index].co for index in triangle.vertices]
  converted=[position_key((position.x,position.z,-position.y)) for position in positions]
  triangle_coordinates[tuple(sorted(converted))]={position:tuple((coordinates.data[loop_index].uv.x,1-coordinates.data[loop_index].uv.y)) for position,loop_index in zip(converted,triangle.loops)}
 polygon_coordinate_records=[];polygons_by_position={}
 for polygon in scene_object.data.polygons:
  polygon_coordinates={}
  for loop_index in polygon.loop_indices:
   position=conversion@scene_object.matrix_world@scene_object.data.vertices[scene_object.data.loops[loop_index].vertex_index].co
   polygon_coordinates[position_key((position.x,position.z,-position.y))]=(coordinates.data[loop_index].uv.x,1-coordinates.data[loop_index].uv.y)
  polygon_index=len(polygon_coordinate_records);polygon_coordinate_records.append(polygon_coordinates)
  for position in polygon_coordinates:polygons_by_position.setdefault(position,set()).add(polygon_index)
 node=next(node for node in document['nodes'] if node.get('name')==scene_object.name and 'mesh' in node)
 encoded=image_path.read_bytes();buffer.extend(b'\0'*((-len(buffer))%4));offset=len(buffer);buffer.extend(encoded)
 image_view_index=len(document['bufferViews']);document['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':len(encoded)})
 image_index=len(document['images']);document['images'].append({'bufferView':image_view_index,'mimeType':'image/png','name':image.name})
 texture_index=len(document['textures']);document['textures'].append({'source':image_index,'sampler':0})
 material_mapping={}
 for primitive in document['meshes'][node['mesh']]['primitives']:
  attributes={name:accessor_values(index) for name,index in primitive['attributes'].items()};indices=accessor_values(primitive['indices']).ravel();positions=attributes['POSITION'];old_indices=[];new_coordinates=[];new_indices=[];vertex_mapping={}
  for triangle_offset in range(0,len(indices),3):
   triangle_indices=indices[triangle_offset:triangle_offset+3];keys=[position_key(positions[index]) for index in triangle_indices];key=tuple(sorted(keys))
   if key not in triangle_coordinates:
    candidates=set.intersection(*(polygons_by_position.get(position,set()) for position in keys))
    if not candidates:raise RuntimeError('Source/runtime triangle mismatch '+scene_object.name+' '+str(key))
    triangle_coordinates[key]=polygon_coordinate_records[min(candidates)]
   for original_index,position in zip(triangle_indices,keys):
    coordinate=triangle_coordinates[key][position];vertex_key=(int(original_index),tuple(round(value,7) for value in coordinate))
    if vertex_key not in vertex_mapping:
     vertex_mapping[vertex_key]=len(old_indices);old_indices.append(int(original_index));new_coordinates.append(coordinate)
    new_indices.append(vertex_mapping[vertex_key])
  for name,values in attributes.items():
   original_accessor=document['accessors'][primitive['attributes'][name]];primitive['attributes'][name]=append_accessor(values[old_indices],original_accessor['type'])
  primitive['attributes']['TEXCOORD_1']=append_accessor(new_coordinates,'VEC2');primitive['indices']=append_accessor(numpy.array(new_indices).reshape(-1,1),'SCALAR',5125)
  original_material_index=primitive['material']
  if original_material_index not in material_mapping:
   material=copy.deepcopy(document['materials'][original_material_index]);material['name']=scene_object.name+'UpdatedSponsorsRuntimeMaterial';material['pbrMetallicRoughness']['baseColorTexture']={'index':texture_index,'texCoord':1};material['pbrMetallicRoughness']['metallicRoughnessTexture']={'index':additional_textures['parameters'],'texCoord':1};material['normalTexture']={'index':additional_textures['normal'],'texCoord':1};material['occlusionTexture']={'index':additional_textures['parameters'],'texCoord':1};new_material_index=len(document['materials']);document['materials'].append(material);material_mapping[original_material_index]=new_material_index
  primitive['material']=material_mapping[original_material_index]
 records.append({'object':scene_object.name,'texture':str(image_path),'texture_sha256':hashlib.sha256(encoded).hexdigest(),'texture_resolution':[2048,2048],'triangles':sum(document['accessors'][primitive['indices']]['count']//3 for primitive in document['meshes'][node['mesh']]['primitives'])})
 print('BAKED_AND_EXPORTED',scene_object.name,flush=True)
document['buffers'][0]['byteLength']=len(buffer)
assert bytes(buffer[:len(original_buffer)])==original_buffer
chassis_path=output_directory/'f1_2030_v10_chassis.glb';validate_output_path(repository_directory,chassis_path,'preview');write_glb(chassis_path,document,buffer)
for key,asset in manifest['geometry_assets'].items():
 destination=output_directory/asset['path'];validate_output_path(repository_directory,destination,'preview')
 if key!='chassis':shutil.copyfile(asset_directory/asset['path'],destination)
 asset['sha256']=hashlib.sha256(destination.read_bytes()).hexdigest().upper();asset['size_bytes']=destination.stat().st_size
manifest['current_chassis_source_sha256']=source_digest
record={'source_file':str(source_path),'source_digest':source_digest,'baseline_chassis_digest':hashlib.sha256((asset_directory/'f1_2030_v10_chassis.glb').read_bytes()).hexdigest(),'bodywork_textures':records,'original_position_normal_tangent_and_texture_coordinate_values_preserved':True,'unrelated_embedded_textures_and_meshes_preserved':True,'wheel_exports_byte_identical':True,'files':manifest['geometry_assets']}
manifest['sponsor_layout_export']=record
for name,value in [('manifest.json',manifest),('sponsor_layout_export_report.json',record)]:
 destination=output_directory/name;validate_output_path(repository_directory,destination,'preview');destination.write_text(json.dumps(value,indent=2))
assert hashlib.sha256(source_path.read_bytes()).hexdigest()==source_digest
print('EXPORT_COMPLETED',flush=True)
