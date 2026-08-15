---
name: telemetry-read-advise
description: Gate Formula-90 telemetry reading before an agent opens, parses, summarizes, graphs, or diagnoses a log, CSV, JSON, trace, or recording. Use whenever telemetry is requested or mentioned. First ask whether to use the last generated file and, if so, require an explicit percentage/range or named telemetry section; never assume the whole latest file is in scope.
---

# Telemetry read advise

Apply this gate before listing telemetry contents, choosing a file, reading file
metadata, parsing rows, rendering a graph, or running an analysis command.

## Mandatory first question

Ask the user this paired question, then wait for the answer:

```text
Quieres usar el ultimo archivo de telemetria generado? (si/no)
Si es si, que porcentaje/rango o seccion especifica debo leer?
```

Do not infer that "latest", "recent", or an unqualified telemetry request
authorizes reading the entire newest file. A valid scope is a percentage/range
such as `0-10%`, `12:00-14:30`, `rows 300-900`, or a named section such as
`braking`, `suspension_FL`, `RPM`, or `curb-impact`.

## Route the answer

- If the answer is `si`, require the requested percentage, timestamp/row range,
  or named section before reading the latest file.
- If the answer is `no`, request the exact file path and then the same bounded
  range or named section.
- If the user already supplied a file, still confirm whether it is the intended
  latest file and obtain the bounded scope before opening it.
- If the request says `all`, ask the user to explicitly confirm `0-100%` or to
  choose sections; do not silently expand scope.

After both the file choice and scope are explicit, use `telemetry` for capture,
validation, summarization, or diagnosis. Report the selected file and slice in
the analysis result so the evidence is reproducible.

Never delete, rotate, overwrite, or alter telemetry files as part of this gate.
