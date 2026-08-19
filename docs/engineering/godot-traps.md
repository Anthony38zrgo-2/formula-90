# Godot 4 Anti-Traps & Coding Standards

This document is **mandatory reading** for any agent (e.g., DeepSeek/Flash) before executing code modifications in this project. Its purpose is to neutralize statistical biases towards Godot 3 and prevent syntax corruption in Godot 4.

## 1. Absolute Node Rule (`.tscn`)
- **LLM Bias:** Editing a `.tscn` file by deleting lines that look like "unnecessary metadata".
- **Godot 4 Reality:** If a script exports a node (`@export var my_node: Node`), the engine saves its reference in an array called `node_paths=PackedStringArray(...)` within the node's definition in the `.tscn`.
- **Strict Rule:** 🛑 NEVER delete or modify the `node_paths` attribute when editing a `.tscn`. If you do, the references will silently become null and you will break the game. Use surgical text replacements (`replace_file_content`), never regenerate entire blocks.

## 2. Signal Syntax
- **LLM Bias (Godot 3):** `button.connect("pressed", self, "_on_button_pressed")`
- **Godot 4 Reality:** The engine uses a strict object-based syntax (First-class signals).
- **Strict Rule:** Always use: `button.pressed.connect(_on_button_pressed)`. NEVER use the old string syntax.

## 3. Exported Variables (`@export`)
- **LLM Bias (Godot 3):** `export(NodePath) var path` or `export(float) var speed`
- **Godot 4 Reality:** All exports use annotations.
- **Strict Rule:** Always use `@export var node: Node` or `@export var speed: float = 10.0`.

## 4. Node Searching (`get_node` and `%`)
- **LLM Bias:** Using `$Parent/Child/Button` assuming immutable hierarchies.
- **Strict Rule:** If you are programming interfaces (UI), encourage the use of Scene Unique Nodes (`%Button`) instead of long paths prone to breaking.

## 5. Beware of Damping and Multipliers (Physics Engine)
- The `damping_ratio` in GEVP is the $\zeta$ coefficient. A value of 1.0 is critical damping. NEVER set it to disproportionate values (e.g., 5.0) assuming you are compensating for mass. The physics engine already calculates the force by multiplying $\zeta$ by the spring stiffness.
- **Before touching physics**, read `docs/game-design/ai-physics-manual.md`.
