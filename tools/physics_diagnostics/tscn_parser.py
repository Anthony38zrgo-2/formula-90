#!/usr/bin/env python3
"""
tscn_parser.py
==============
Reads a Godot .tscn file and extracts numeric properties into a dictionary.
This prevents AI agents from using stale defaults in the diagnostic tools.
"""

import re
import os

def parse_tscn(filepath: str) -> dict:
    if not os.path.exists(filepath):
        print(f"⚠️  [tscn_parser] File not found: {filepath}. Using defaults.")
        return {}

    parsed_data = {}
    
    # Regex to match basic property assignments: var_name = 0.5
    prop_pattern = re.compile(r'^([a-zA-Z0-9_]+)\s*=\s*([0-9\.-]+)$')
    
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                match = prop_pattern.match(line)
                if match:
                    key = match.group(1)
                    val_str = match.group(2)
                    try:
                        # Attempt float conversion
                        val = float(val_str)
                        parsed_data[key] = val
                    except ValueError:
                        pass
    except Exception as e:
        print(f"⚠️  [tscn_parser] Error reading {filepath}: {e}")
        
    return parsed_data

def update_config_from_tscn(cfg: dict, tscn_path: str):
    """
    Updates the provided cfg dictionary with values found in the tscn file.
    Only updates keys that already exist in cfg to avoid bloat.
    """
    tscn_data = parse_tscn(tscn_path)
    for key, val in tscn_data.items():
        # Match names. Sometimes Godot saves things exactly as our keys.
        mapped_key = key.replace("_multiplier", "_mult")
        if mapped_key in cfg:
            cfg[mapped_key] = val
        elif key in cfg:
            cfg[key] = val
        # Handle specific mappings if needed (e.g. if the tscn uses a different name)
        
    return cfg

if __name__ == "__main__":
    test_path = "../../game/scenes/vehicles/f1_2026_car.tscn"
    data = parse_tscn(test_path)
    print(f"Parsed {len(data)} properties from {test_path}")
    print("Sample:")
    for k in ['front_spring_length', 'front_damping_ratio', 'rear_bump_stop_multiplier']:
        if k in data:
            print(f"  {k} = {data[k]}")
