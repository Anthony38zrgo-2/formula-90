import pathlib, re
p = pathlib.Path(r'D:\Formula90s\game\scenes\vehicles\jordan_197\jordan_197.tscn')
content = p.read_text(encoding='utf-8')
print("=== NODES ===")
for m in re.finditer(r'\[node name="([^"]+)"', content):
    print(m.group(1))
print("=== NODE_PATHS ===")
for line in content.splitlines():
    if 'vehicle_node' in line or 'front_left_wheel' in line or 'front_right_wheel' in line or 'rear_left_wheel' in line or 'rear_right_wheel' in line or 'wheel_node' in line:
        print(line.strip())
print("=== RESOURCES ===")
for line in content.splitlines():
    if 'PackedScene' in line and 'jordan_197' in line:
        m = re.search(r'path="res://([^"]+)"', line)
        if m:
            disk = pathlib.Path(r'D:\Formula90s\game') / m.group(1).replace('/', '\\')
            print(f"RESOURCE {m.group(1)} exists:{disk.exists()}")
print("=== TRANSFORMS ===")
for i,line in enumerate(content.splitlines(), start=1):
    if 'ChassisVisual' in line or 'Visual' in line or 'transform' in line:
        print(f"{i}: {line}")
