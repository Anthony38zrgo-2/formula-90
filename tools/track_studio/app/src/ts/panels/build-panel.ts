// Build panel: compiles BuildIR and shows the incremental build plan.

import { api } from "../core/api";

export function setupBuildPanel(
  buildButton: HTMLButtonElement,
  output: HTMLElement,
): void {
  buildButton.addEventListener("click", async () => {
    output.textContent = "Building...";
    try {
      const buildIr = await api.compileBuildIr();
      const plan = await api.buildPlan();
      output.textContent =
        `BuildIR compiled. Dirty subsystems: ${plan.join(", ") || "(none)"}`;
      console.debug("build_ir", buildIr);
    } catch (error) {
      output.textContent = `Build failed: ${String(error)}`;
    }
  });
}
