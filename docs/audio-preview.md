# Preview A/B de audio

AUDIO-004 reduce el human gate a un comando y dos archivos audibles:

```powershell
.\preview_audio.ps1
```

El comando:

1. construye el banco candidato desde `source-assets/audio/`;
2. valida manifiesto, hashes y formato WAV;
3. renderiza el mismo escenario con el banco runtime y el candidato;
4. escribe todo bajo `scratch/audio/ab/`;
5. no modifica ni promueve `game/sounds/`.

Escuchar, en este orden:

```text
scratch/audio/ab/current/idle_to_redline_with_shifts.wav
scratch/audio/ab/candidate/idle_to_redline_with_shifts.wav
```

La decisión humana es solamente una de estas:

- **keep**: el candidato merece continuar;
- **reject**: se descarta sin ejecutar FULL;
- **iterate**: se ajustan fuentes o receta y se repite el mismo comando.

Para probar otra familia sin crear un proceso nuevo:

```powershell
.\preview_audio.ps1 --scenario impact
.\preview_audio.ps1 --scenario road_to_sand_with_kerb
.\preview_audio.ps1 --scenario grass_skid
```

`comparison.json` conserva paths, hashes, peak, RMS y findings del validador.
Estas métricas detectan errores baratos; no sustituyen la escucha ni intentan
decidir automáticamente si el producto es bueno.
