# Modify

Classify as `MODIFY`. Start with an analysis report and an explicit hypothesis:

```text
region -> intended change -> preserved invariants -> validation command
```

Copy the source into `working/`, use NumPy/SciPy/Trimesh for selection and
PyMeshLab for mesh mutation. Select regions from geometry or a contractually
stable generated schema, never guessed indices. Apply one causal change, then
run validation with `--compare-to` the original.

Low-confidence classifications are read-only until reviewed.
