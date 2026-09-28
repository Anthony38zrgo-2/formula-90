# stb — Single-header image libraries

Scope: vendored `stb` headers used by native tooling for image decode and encode.
Consumers: `tools/validation/` and native build targets.
Rules: upgrade by replacing the vendored headers wholesale; no local forks.
