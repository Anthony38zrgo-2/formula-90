# Pipeline de audio V10

Python se usa únicamente fuera del juego para sintetizar WAV originales. `tools/audio/generate_engine_samples.py` lee `config/audio_synthesis_v10.yaml` y produce cinco capas continuas y dos cambios one-shot, mono PCM16 a 44.1 kHz, además de hashes y loops sugeridos. La semilla 9010 hace reproducible el banco conceptual; no imita una grabación, equipo o motor comercial.

```powershell
scripts\setup_audio_tools_windows.ps1
.venv\Scripts\python.exe tools\audio\generate_engine_samples.py --config config\audio_synthesis_v10.yaml --output game\assets\audio\engines\v10_prototype --force
scripts\test_audio_tools_windows.ps1
build\tools\analyze_audio.exe game\assets\audio\engines\v10_prototype\engine_mid.wav
```

Las capas se precargan al iniciar `EngineAudioController`; no hay Python ni disco durante el procesamiento. Los loops continuos reutilizan buffers PCM y los cambios se reproducen una sola vez. `analyze_audio` informa duración, formato, peak/dBFS, RMS, DC, clipping, silencio y continuidad; `validate_assets` exige mono, 44.1 kHz y ausencia de clipping.
