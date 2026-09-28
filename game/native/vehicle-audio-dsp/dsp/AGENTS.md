# dsp — DSP stage sources

Scope: the DSP stage implementations linked into the audio library.
Consumers: the library build via `CMakeLists.txt`.
Rules: stage boundaries match the Rust DSP chain; changes on either side run the ABI checker.
