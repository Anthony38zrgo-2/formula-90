import os, sys, subprocess, glob

vs_path = r"C:\Program Files\Microsoft Visual Studio\2022\Community"
vcvars_bat = os.path.join(vs_path, "VC", "Auxiliary", "Build", "vcvars64.bat")

root = r"d:\Formula90s"
sources = glob.glob(os.path.join(root, "native", "src", "*.cpp"))
for folder in ["core", "vehicle", "camera", "presentation", "ui", "audio", "sim"]:
    sources += glob.glob(os.path.join(root, "native", "src", folder, "*.cpp"))

includes = [
    os.path.join(root, "native", "include"),
    os.path.join(root, "third_party", "godot-cpp", "include"),
    os.path.join(root, "third_party", "godot-cpp", "gen", "include"),
    os.path.join(root, "third_party", "godot-cpp", "gdextension"),
]

inc_flags = " ".join([f'/I"{inc}"' for inc in includes])

targets = ["debug", "release"]
for target in targets:
    is_debug = (target == "debug")
    out_name = f"libformula90s.windows.template_{target}.x86_64.dll"
    out_path = os.path.join(root, "game", "addons", "formula90s", "bin", out_name)
    godot_lib = os.path.join(root, "third_party", "godot-cpp", "bin", f"libgodot-cpp.windows.template_{target}.x86_64.lib")
    
    cflags = "/std:c++20 /Zc:__cplusplus /EHsc /MT /W3 /utf-8 /D_CRT_SECURE_NO_WARNINGS /DNOMINMAX /D_USE_MATH_DEFINES /DTYPED_METHOD_BIND"
    if is_debug:
        cflags += " /Od /Z7 /DDEBUG_ENABLED /DDEBUG_METHODS_ENABLED"
    else:
        cflags += " /O2 /GL /DNDEBUG"

    src_list = " ".join([f'"{s}"' for s in sources])
    cmd = f'call "{vcvars_bat}" && cl {cflags} {inc_flags} {src_list} /link /DLL /OUT:"{out_path}" "{godot_lib}" user32.lib advapi32.lib shell32.lib ws2_32.lib'
    print(f"Building {out_name}...")
    res = subprocess.run(cmd, shell=True, cwd=root, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"ERROR building {out_name}:")
        print(res.stdout)
        print(res.stderr)
        sys.exit(1)
    else:
        print(f"SUCCESS: {out_name} generated ({os.path.getsize(out_path)} bytes)")

print("All GDExtension DLLs built successfully!")
