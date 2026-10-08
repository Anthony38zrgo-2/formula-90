import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
import export_f1_2030_v10


def bake_headrest_material(output_directory):
    headrest = bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding']
    headrest.data = headrest.data.copy()
    original_coordinates = headrest.data.uv_layers['CockpitHeadrestSurfaceCoordinates']
    export_coordinates = headrest.data.uv_layers.new(name='HeadrestExportTextureCoordinates')
    minimum = [min(coordinate.uv[axis] for coordinate in original_coordinates.data) for axis in range(2)]
    maximum = [max(coordinate.uv[axis] for coordinate in original_coordinates.data) for axis in range(2)]
    for source_coordinate, export_coordinate in zip(original_coordinates.data, export_coordinates.data):
        export_coordinate.uv = [0.002 + 0.996 * (source_coordinate.uv[axis] - minimum[axis]) / (maximum[axis] - minimum[axis]) for axis in range(2)]
    headrest.data.uv_layers.active_index = len(headrest.data.uv_layers) - 1
    export_coordinates.active_render = True
    material = headrest.data.materials[0].copy()
    headrest.data.materials[0] = material
    scene = bpy.context.scene
    scene.render.engine = 'CYCLES'
    scene.cycles.device = 'CPU'
    scene.cycles.samples = 1
    scene.render.bake.margin = 8
    bpy.ops.object.select_all(action='DESELECT')
    headrest.select_set(True)
    bpy.context.view_layer.objects.active = headrest
    baked_images = {}
    for texture_name, bake_type in (('HeadrestCarbonBaseColor', 'DIFFUSE'), ('HeadrestCarbonNormal', 'NORMAL')):
        image = bpy.data.images.new(texture_name, width=2048, height=2048, alpha=False)
        image.generated_color = (0.02, 0.02, 0.02, 1) if bake_type == 'DIFFUSE' else (0.5, 0.5, 1.0, 1)
        if bake_type == 'NORMAL':
            image.colorspace_settings.name = 'Non-Color'
        texture = material.node_tree.nodes.new('ShaderNodeTexImage')
        texture.image = image
        texture.select = True
        material.node_tree.nodes.active = texture
        print('Baking ' + texture_name, flush=True)
        if bake_type == 'DIFFUSE':
            bpy.ops.object.bake(type=bake_type, pass_filter={'COLOR'}, uv_layer=export_coordinates.name)
        else:
            bpy.ops.object.bake(type=bake_type, uv_layer=export_coordinates.name)
        image.filepath_raw = str(validate_output_path(PROJECT_DIRECTORY, output_directory / (texture_name + '.png'), 'preview').path)
        image.file_format = 'PNG'
        image.save()
        image.pack()
        baked_images[texture_name] = image
    runtime_material = bpy.data.materials.new('FormulaOne2030HeadrestExportCarbon')
    runtime_material.use_nodes = True
    surface = runtime_material.node_tree.nodes['Principled BSDF']
    surface.inputs['Metallic'].default_value = 0.0
    surface.inputs['Roughness'].default_value = 0.44
    surface.inputs['Coat Weight'].default_value = 0.04
    coordinates = runtime_material.node_tree.nodes.new('ShaderNodeUVMap')
    coordinates.uv_map = export_coordinates.name
    color_texture = runtime_material.node_tree.nodes.new('ShaderNodeTexImage')
    color_texture.image = baked_images['HeadrestCarbonBaseColor']
    normal_texture = runtime_material.node_tree.nodes.new('ShaderNodeTexImage')
    normal_texture.image = baked_images['HeadrestCarbonNormal']
    normal_conversion = runtime_material.node_tree.nodes.new('ShaderNodeNormalMap')
    normal_conversion.uv_map = export_coordinates.name
    runtime_material.node_tree.links.new(coordinates.outputs['UV'], color_texture.inputs['Vector'])
    runtime_material.node_tree.links.new(coordinates.outputs['UV'], normal_texture.inputs['Vector'])
    runtime_material.node_tree.links.new(color_texture.outputs['Color'], surface.inputs['Base Color'])
    runtime_material.node_tree.links.new(normal_texture.outputs['Color'], normal_conversion.inputs['Color'])
    runtime_material.node_tree.links.new(normal_conversion.outputs['Normal'], surface.inputs['Normal'])
    headrest.data.materials[0] = runtime_material
    for texture_coordinates in list(headrest.data.uv_layers):
        if texture_coordinates.name != export_coordinates.name:
            headrest.data.uv_layers.remove(texture_coordinates)
    return {'material': runtime_material.name, 'texture_resolution': 2048, 'baked_textures': {name: image.filepath_raw for name, image in baked_images.items()}, 'roughness': 0.44, 'metallic': 0.0}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--physics-profile', type=Path, required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    output_directory = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    source_path = Path(bpy.data.filepath)
    source_digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
    material_report = bake_headrest_material(output_directory)
    sys.argv = [sys.argv[0], '--', str(output_directory), str(options.physics_profile)]
    export_f1_2030_v10.main()
    report_path = output_directory / 'export_report.json'
    report = json.loads(report_path.read_text())
    report['source_path'] = str(source_path)
    report['source_sha256'] = source_digest
    report['headrest_material'] = material_report
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    if hashlib.sha256(source_path.read_bytes()).hexdigest() != source_digest:
        raise RuntimeError('The exporter changed the authoring source')
    print('Five vehicle exports completed without changing the authoring source', flush=True)


if __name__ == '__main__':
    main()
