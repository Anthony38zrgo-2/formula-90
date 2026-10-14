import numpy
import math
texture_pixel_cache={}
from types import SimpleNamespace
compiled_material_cache={}
class CompiledInputs:
 def __init__(self,values):self.values=values;self.by_name={value.name:value for value in values}
 def __getitem__(self,key):return self.values[key] if isinstance(key,int) else self.by_name[key]
class CompiledNode:
 def as_pointer(self):return self.identity

def compiled_material_nodes(material):
 identity=material.as_pointer()
 if identity in compiled_material_cache:return compiled_material_cache[identity]
 compiled_nodes={};compiled_sockets={}
 for source_node in material.node_tree.nodes:
  node=CompiledNode();node.identity=source_node.as_pointer();node.type=source_node.type;node.operation=getattr(source_node,'operation',None);node.uv_map=getattr(source_node,'uv_map','');node.image=getattr(source_node,'image',None);node.extension=getattr(source_node,'extension','')
  if source_node.type=='VALTORGB':node.color_ramp=SimpleNamespace(elements=[SimpleNamespace(position=element.position,color=tuple(element.color)) for element in source_node.color_ramp.elements])
  compiled_nodes[node.identity]=node
  for source_socket in list(source_node.inputs)+list(source_node.outputs):
   default=getattr(source_socket,'default_value',0)
   if not isinstance(default,(float,int)):default=tuple(default)
   socket=SimpleNamespace(node=node,name=source_socket.name,default_value=default,is_linked=False,links=[])
   compiled_sockets[source_socket.as_pointer()]=socket
  node.inputs=CompiledInputs([compiled_sockets[socket.as_pointer()] for socket in source_node.inputs])
 for source_node in material.node_tree.nodes:
  for source_socket in source_node.inputs:
   socket=compiled_sockets[source_socket.as_pointer()];socket.is_linked=source_socket.is_linked;socket.links=[SimpleNamespace(from_socket=compiled_sockets[link.from_socket.as_pointer()]) for link in source_socket.links]
 result=list(compiled_nodes.values());compiled_material_cache[identity]=result;return result
def decoded(values):return numpy.where(values<=.04045,values/12.92,((values+.055)/1.055)**2.4)
def encoded(values):return numpy.where(values<=.0031308,values*12.92,1.055*numpy.maximum(values,0)**(1/2.4)-.055)
def image_values(image):
 key=image.as_pointer()
 if key not in texture_pixel_cache:
  values=numpy.empty(image.size[0]*image.size[1]*4,dtype=numpy.float32);image.pixels.foreach_get(values);texture_pixel_cache[key]=values.reshape(image.size[1],image.size[0],4)
 return texture_pixel_cache[key]
def evaluate_colors(material,positions,coordinate_layers):
 nodes=compiled_material_nodes(material);cache={};sample_count=len(positions)
 def input_value(socket):
  if socket.is_linked:
   linked=output_value(socket.links[0].from_socket)
   if isinstance(socket.default_value,(float,int)) and linked.ndim==2:return numpy.sum(linked[:,:3]*numpy.array([.2126,.7152,.0722]),axis=1)
   return linked
  value=socket.default_value
  if isinstance(value,(float,int)):return numpy.full(sample_count,value,dtype=numpy.float32)
  return numpy.tile(numpy.array(value,dtype=numpy.float32),(sample_count,1))
 def output_value(socket):
  key=(socket.node.as_pointer(),socket.name)
  if key in cache:return cache[key]
  node=socket.node
  if node.type=='RGB':value=numpy.tile(numpy.array(socket.default_value),(sample_count,1))
  elif node.type=='UVMAP':value=coordinate_layers[node.uv_map]
  elif node.type=='TEX_COORD':value=positions
  elif node.type=='NEW_GEOMETRY':value=positions
  elif node.type=='SEPXYZ':value=input_value(node.inputs[0])[:,{'X':0,'Y':1,'Z':2}[socket.name]]
  elif node.type=='COMBXYZ':value=numpy.stack([input_value(node.inputs[index]) for index in range(3)],axis=1)
  elif node.type=='SEPARATE_COLOR':value=input_value(node.inputs[0])[:,{'Red':0,'Green':1,'Blue':2}[socket.name]]
  elif node.type=='VALTORGB':
   factor=input_value(node.inputs[0]);elements=list(node.color_ramp.elements);stops=[element.position for element in elements];colors=numpy.array([list(element.color) for element in elements]);value=numpy.stack([numpy.interp(factor,stops,colors[:,channel]) for channel in range(4)],axis=1)
  elif node.type=='MAPPING':
   vector=input_value(node.inputs['Vector']);scale=input_value(node.inputs['Scale']);location=input_value(node.inputs['Location']);rotation=input_value(node.inputs['Rotation'])[0];first,second,third=rotation
   first_rotation=numpy.array([[1,0,0],[0,math.cos(first),-math.sin(first)],[0,math.sin(first),math.cos(first)]])
   second_rotation=numpy.array([[math.cos(second),0,math.sin(second)],[0,1,0],[-math.sin(second),0,math.cos(second)]])
   third_rotation=numpy.array([[math.cos(third),-math.sin(third),0],[math.sin(third),math.cos(third),0],[0,0,1]])
   if vector.shape[1]==2:vector=numpy.column_stack([vector,numpy.zeros(sample_count)])
   scaled=vector*scale;rotation_matrix=third_rotation@second_rotation@first_rotation
   value=numpy.stack([numpy.sum(scaled*rotation_matrix[axis],axis=1) for axis in range(3)],axis=1)+location
  elif node.type=='MATH':
   first=input_value(node.inputs[0]);second=input_value(node.inputs[1]);operation=node.operation
   if operation=='SUBTRACT':value=first-second
   elif operation=='MULTIPLY':value=first*second
   elif operation=='DIVIDE':value=numpy.divide(first,second,out=numpy.zeros_like(first),where=second!=0)
   elif operation=='ADD':value=first+second
   elif operation=='GREATER_THAN':value=(first>second).astype(numpy.float32)
   elif operation=='MAXIMUM':value=numpy.maximum(first,second)
   else:raise RuntimeError('Unsupported math '+operation)
  elif node.type=='MIX_RGB':
   factor=numpy.clip(input_value(node.inputs[0]),0,1)[:,None]
   if numpy.all(factor==0):value=input_value(node.inputs[1])
   elif numpy.all(factor==1):value=input_value(node.inputs[2])
   else:
    first=input_value(node.inputs[1]);second=input_value(node.inputs[2]);value=first*(1-factor)+second*factor
  elif node.type=='TEX_IMAGE':
   coordinates=input_value(node.inputs['Vector'])[:,:2];pixels=image_values(node.image);height,width=pixels.shape[:2]
   valid=(coordinates[:,0]>=0)&(coordinates[:,0]<=1)&(coordinates[:,1]>=0)&(coordinates[:,1]<=1)
   if node.extension=='CLIP' and not numpy.any(valid):
    value=numpy.zeros(sample_count) if socket.name=='Alpha' else numpy.zeros((sample_count,4));cache[key]=value;return value
   if node.extension=='REPEAT':coordinates=coordinates%1
   horizontal=numpy.clip(coordinates[:,0]*width-.5,0,width-1.00001);vertical=numpy.clip(coordinates[:,1]*height-.5,0,height-1.00001)
   first_horizontal=horizontal.astype(numpy.int32);first_vertical=vertical.astype(numpy.int32);horizontal_fraction=(horizontal-first_horizontal)[:,None];vertical_fraction=(vertical-first_vertical)[:,None]
   sampled=(pixels[first_vertical,first_horizontal]*(1-horizontal_fraction)+pixels[first_vertical,first_horizontal+1]*horizontal_fraction)*(1-vertical_fraction)+(pixels[first_vertical+1,first_horizontal]*(1-horizontal_fraction)+pixels[first_vertical+1,first_horizontal+1]*horizontal_fraction)*vertical_fraction
   if node.extension=='CLIP':sampled[~valid]=0
   if socket.name=='Alpha':value=sampled[:,3]
   else:
    value=sampled.copy()
    if node.image.colorspace_settings.name=='sRGB':value[:,:3]=decoded(value[:,:3])
  else:raise RuntimeError('Unsupported color node '+node.type)
  cache[key]=value;return value
 shader=next(node for node in nodes if node.type=='BSDF_PRINCIPLED')
 color=encoded(input_value(shader.inputs['Base Color'])[:,:3])
 roughness=input_value(shader.inputs['Roughness']);metallic=input_value(shader.inputs['Metallic'])
 normal_map=next((node for node in nodes if node.type=='NORMAL_MAP'),None)
 if normal_map is None:normal=numpy.tile([.5,.5,1],(sample_count,1))
 else:
  normal=input_value(normal_map.inputs['Color'])[:,:3]*2-1
  strength=input_value(normal_map.inputs['Strength']);normal[:,:2]*=strength[:,None]
  normal=normal/numpy.maximum(numpy.linalg.norm(normal,axis=1)[:,None],.000001);normal=normal*.5+.5
 return color,numpy.stack([numpy.ones(sample_count),roughness,metallic],axis=1),normal
def rasterize_bodywork(scene_object,image):
 mesh=scene_object.data;mesh.calc_loop_triangles();coordinates=mesh.uv_layers['UpdatedSponsorExportCoordinates'];height,width=image.size[1],image.size[0]
 pixels=numpy.zeros((height,width,4),dtype=numpy.float32);parameters=numpy.ones_like(pixels);normal_pixels=numpy.ones_like(pixels);normal_pixels[:,:,:2]=.5;covered=numpy.zeros((height,width),dtype=bool)
 for triangle in mesh.loop_triangles:
  texture_coordinates=numpy.array([list(coordinates.data[index].uv) for index in triangle.loops])*numpy.array([width,height])
  minimum=numpy.maximum(numpy.floor(texture_coordinates.min(axis=0)).astype(int),0);maximum=numpy.minimum(numpy.ceil(texture_coordinates.max(axis=0)).astype(int),[width-1,height-1])
  if numpy.any(maximum<minimum):continue
  vertical,horizontal=numpy.mgrid[minimum[1]:maximum[1]+1,minimum[0]:maximum[0]+1];delta=numpy.stack([horizontal+.5,vertical+.5],axis=-1)-texture_coordinates[0]
  first_edge=texture_coordinates[1]-texture_coordinates[0];second_edge=texture_coordinates[2]-texture_coordinates[0];determinant=first_edge[0]*second_edge[1]-first_edge[1]*second_edge[0]
  if abs(determinant)<.00001:continue
  first_weight=(delta[...,0]*second_edge[1]-delta[...,1]*second_edge[0])/determinant;second_weight=(first_edge[0]*delta[...,1]-first_edge[1]*delta[...,0])/determinant
  selected=(first_weight>=0)&(second_weight>=0)&(first_weight+second_weight<=1)
  if not numpy.any(selected):continue
  first_weight=first_weight[selected,None];second_weight=second_weight[selected,None]
  positions=numpy.array([list(scene_object.matrix_world@mesh.vertices[index].co) for index in triangle.vertices]);world=positions[0]+first_weight*(positions[1]-positions[0])+second_weight*(positions[2]-positions[0])
  coordinate_layers={}
  for layer in mesh.uv_layers:
   values=numpy.array([list(layer.data[index].uv) for index in triangle.loops]);coordinate_layers[layer.name]=values[0]+first_weight*(values[1]-values[0])+second_weight*(values[2]-values[0])
  colors,material_parameters,normals=evaluate_colors(mesh.materials[triangle.material_index],world,coordinate_layers)
  pixels[vertical[selected],horizontal[selected],:3]=colors;parameters[vertical[selected],horizontal[selected],:3]=material_parameters;normal_pixels[vertical[selected],horizontal[selected],:3]=normals;covered[vertical[selected],horizontal[selected]]=True
 for iteration in range(8):
  initial=covered.copy()
  for axis,shift in [(0,1),(0,-1),(1,1),(1,-1)]:
   available=numpy.roll(initial,shift,axis=axis)&~covered
   pixels[available]=numpy.roll(pixels,shift,axis=axis)[available];parameters[available]=numpy.roll(parameters,shift,axis=axis)[available];normal_pixels[available]=numpy.roll(normal_pixels,shift,axis=axis)[available];covered[available]=True
 pixels[:,:,3]=1;image.pixels.foreach_set(pixels.ravel())
 return parameters,normal_pixels
