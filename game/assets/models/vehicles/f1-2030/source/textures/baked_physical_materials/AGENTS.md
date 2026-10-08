# baked_physical_materials

Scope: reproducible vehicle PBR atlases and generated albedo reference. Consumers: canonical f1_2030.blend and its GLB exports. Base color uses sRGB; normal and packed occlusion/roughness/metallic use Non-Color. Packed channels: red occlusion, green roughness, blue metallic. Preserve source material node trees and original UV layers. Historical decal geometry is a projection input; the canonical chassis and tire printing use flat color textures under ../reference_balanced_finish/.
