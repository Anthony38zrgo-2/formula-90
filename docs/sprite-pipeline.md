# Pipeline C++ de sprites

Las herramientas se compilan con `scripts/build_asset_tools_windows.ps1` o `scripts/build_asset_tools_linux.sh`. Se usa `stb_image`/`stb_image_write`, fijado al commit `f58f558c120e9b32c217290b80bad1a0729fbb2c`, porque aporta PNG/JPG portable sin introducir OpenCV. WEBP se rechaza explícitamente al no estar soportado por esta dependencia.

```powershell
build\tools\prepare_sprite.exe --input references/vehicles/v10/source/v10_reference.png --output game/assets/sprites/vehicles/v10/frames/rear.png --metadata game/assets/sprites/vehicles/v10/frames/rear.json --config config/sprite_import.yaml --crop-rect 0,5195,1000,450 --orientation rear --force
build\tools\build_sprite_sheet.exe --input-dir game/assets/sprites/vehicles/v10/frames --output game/assets/sprites/vehicles/v10/v10_sheet.png --metadata game/assets/sprites/vehicles/v10/v10_sheet.json --config config/sprite_sheet.yaml --force
build\tools\validate_assets.exe .
```

`prepare_sprite` valida formato/tamaño/corrupción, preserva alpha, admite fondo `preserve`, `corners` o `color`, protege contra eliminación total, recorta, añade padding, escala nearest y alinea abajo. Emite PNG RGBA y JSON verificable. `--dry-run` no escribe y `--force` es obligatorio para sobrescribir.

`build_sprite_sheet` mantiene orden y ángulos explícitos, valida dimensiones, advierte duplicados/faltantes y genera regiones. Admite `error`, `warn`, `skip` y `nearest_available`. El atlas GPT actual aporta los 16 frames físicos, usa política `error` para ausencias y desactiva el espejado.
