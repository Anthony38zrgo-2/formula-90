"""Bake spatial paint masks and generated artwork into one opaque body albedo.
This changes UVs/materials only, never the body geometry.
"""
import bpy,math
def apply(body,img,destination):
    source=[v.co.copy() for v in body.data.vertices]
    skinuv=body.data.uv_layers.new(name='SkinProjection')
    noseuv=body.data.uv_layers.new(name='NoseProjection')
    fractions={}
    for v in body.data.vertices:
        co=v.co
        if co.y>.85:
            zs=[p.z for p in source if abs(p.y-co.y)<.055]
            fractions[v.index]=min(1,max(0,(co.z-min(zs))/max(.001,max(zs)-min(zs))))
        else: fractions[v.index]=.5
    for loop in body.data.loops:
        co=source[loop.vertex_index]
        skinuv.data[loop.index].uv=(min(.99,max(.01,(co.y+1.56)/2.6)),.025+min(1,max(0,(co.z+.25)/.48))*.485)
        noseuv.data[loop.index].uv=(min(.99,max(.01,(co.y-.91)/1.78)),.565+.43*fractions[loop.vertex_index])
    mat=bpy.data.materials.new('MAT_JORDAN_1997_PAINT'); mat.use_nodes=True
    nodes=mat.node_tree.nodes; links=mat.node_tree.links; nodes.clear()
    def connect(value,socket):
        if hasattr(value,'node'): links.new(value,socket)
        else: socket.default_value=value
    def mathnode(op,a,b=0):
        n=nodes.new('ShaderNodeMath'); n.operation=op
        connect(a,n.inputs[0]); connect(b,n.inputs[1]); return n.outputs[0]
    def mul(*args):
        out=args[0]
        for a in args[1:]: out=mathnode('MULTIPLY',out,a)
        return out
    def mix(a,b,factor):
        n=nodes.new('ShaderNodeMixRGB'); connect(factor,n.inputs[0]); connect(a,n.inputs[1]); connect(b,n.inputs[2]); return n.outputs[0]
    def texture(uvname):
        uv=nodes.new('ShaderNodeUVMap'); uv.uv_map=uvname
        t=nodes.new('ShaderNodeTexImage'); t.image=img; links.new(uv.outputs['UV'],t.inputs['Vector']); return t
    position=nodes.new('ShaderNodeNewGeometry'); xyz=nodes.new('ShaderNodeSeparateXYZ')
    links.new(position.outputs['Position'],xyz.inputs[0]); x,y,z=xyz.outputs
    absx=mathnode('ABSOLUTE',x)
    region=mul(mathnode('GREATER_THAN',absx,.27),mathnode('GREATER_THAN',y,-1.48),mathnode('LESS_THAN',y,.72))
    border=mathnode('ADD',.16,mathnode('MULTIPLY',.018,mathnode('SINE',mathnode('MULTIPLY',mathnode('ADD',y,.2),1.7))))
    skin=texture('SkinProjection'); nose=texture('NoseProjection')
    yellow=(1,.8,.001,1); red=(.82,.004,.001,1)
    color=mix(yellow,skin.outputs['Color'],mul(region,mathnode('LESS_THAN',z,border)))
    stripe=mul(region,mathnode('GREATER_THAN',z,border),mathnode('LESS_THAN',z,mathnode('ADD',border,.024)))
    color=mix(color,red,stripe)
    intake=mul(mathnode('LESS_THAN',absx,.14),mathnode('GREATER_THAN',z,.445),mathnode('GREATER_THAN',y,-.42),mathnode('LESS_THAN',y,-.07))
    color=mix(color,red,intake)
    color=mix(color,nose.outputs['Color'],mul(nose.outputs['Alpha'],mathnode('GREATER_THAN',y,.91)))
    emission=nodes.new('ShaderNodeEmission'); links.new(color,emission.inputs['Color'])
    output=nodes.new('ShaderNodeOutputMaterial'); links.new(emission.outputs[0],output.inputs['Surface'])
    body.data.materials.clear(); body.data.materials.append(mat)
    for p in body.data.polygons: p.material_index=0
    uv=body.data.uv_layers.new(name='BakedLivery'); body.data.uv_layers.active=uv
    uv.active_render=True
    bpy.ops.object.select_all(action='DESELECT'); body.select_set(True); bpy.context.view_layer.objects.active=body
    bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.012)
    bpy.ops.object.mode_set(mode='OBJECT')
    baked=bpy.data.images.new('Jordan1997BodyAlbedo',width=2048,height=2048,alpha=False)
    target=nodes.new('ShaderNodeTexImage'); target.image=baked; nodes.active=target; target.select=True
    scene=bpy.context.scene; scene.render.engine='CYCLES'; scene.cycles.samples=1; scene.cycles.device='CPU'
    bpy.ops.object.bake(type='EMIT',margin=12,use_clear=True)
    baked.filepath_raw=str(destination); baked.file_format='PNG'; baked.save(); baked.pack()
    # Export an ordinary single-texture opaque PBR material, no runtime shader or
    # transparency, and a single UV set supported by the Godot glTF importer.
    for layer in list(body.data.uv_layers):
        if layer.name!='BakedLivery': body.data.uv_layers.remove(layer)
    body.data.uv_layers[0].name='UVMap'
    nodes.clear(); bs=nodes.new('ShaderNodeBsdfPrincipled'); out=nodes.new('ShaderNodeOutputMaterial')
    t=nodes.new('ShaderNodeTexImage'); t.image=baked
    links.new(t.outputs['Color'],bs.inputs['Base Color']); bs.inputs['Roughness'].default_value=.25
    links.new(bs.outputs[0],out.inputs['Surface']); mat.diffuse_color=yellow
