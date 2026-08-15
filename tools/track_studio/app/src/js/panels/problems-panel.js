// Problems panel: renders the current validation diagnostics.
import { appState } from "../core/state";
function severityClass(severity) {
    switch (severity) {
        case "Error":
            return "diag-error";
        case "Warning":
            return "diag-warning";
        default:
            return "diag-info";
    }
}
export function renderDiagnostics(container) {
    const diagnostics = appState.diagnostics;
    container.innerHTML = "";
    if (diagnostics.length === 0) {
        container.textContent = "No problems.";
        return;
    }
    for (const diagnostic of diagnostics) {
        const row = document.createElement("div");
        row.className = severityClass(diagnostic.severity);
        row.textContent = `[${diagnostic.severity}] ${diagnostic.code}: ${diagnostic.message}`;
        container.appendChild(row);
    }
}
export function subscribeProblemsPanel(container) {
    appState.subscribe(() => renderDiagnostics(container));
}
