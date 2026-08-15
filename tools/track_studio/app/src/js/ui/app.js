// Application bootstrap for the presentation layer.
//
// Wires the toolbar to the typed Tauri API and renders the panels. It only
// holds UI state; persistence, geometry and validation stay in Rust.
import { api } from "../core/api";
import { appState } from "../core/state";
import { setupBuildPanel } from "../panels/build-panel";
import { setupReferencePanel } from "../panels/reference-panel";
import { subscribeProblemsPanel } from "../panels/problems-panel";
import { Viewport } from "../viewport/viewport";
function byId(id) {
    const element = document.getElementById(id);
    if (element === null) {
        throw new Error(`missing element #${id}`);
    }
    return element;
}
async function refreshDiagnostics() {
    const diagnostics = await api.validate();
    appState.setDiagnostics(diagnostics);
}
async function openProject() {
    const path = window.prompt("Path to .f90track project");
    if (path === null || path.trim() === "") {
        return;
    }
    const trackId = await api.projectOpen(path.trim());
    appState.setTrackId(trackId);
    await refreshDiagnostics();
}
function wireToolbar() {
    byId("open").addEventListener("click", () => void openProject());
    byId("save").addEventListener("click", () => void api.projectSave());
    byId("undo").addEventListener("click", () => void api.undo());
    byId("redo").addEventListener("click", () => void api.redo());
    byId("validate").addEventListener("click", () => void refreshDiagnostics());
}
export function bootstrap() {
    const viewport = new Viewport(byId("viewport"));
    void viewport;
    wireToolbar();
    subscribeProblemsPanel(byId("problems"));
    setupBuildPanel(byId("build"), byId("build-output"));
    setupReferencePanel(byId("reference"), byId("reference-add"), byId("reference-path"), byId("reference-x"), byId("reference-z"), byId("reference-output"));
    appState.subscribe(() => {
        byId("track-id").textContent = appState.trackId;
    });
    byId("track-id").textContent = appState.trackId;
}
if (typeof document !== "undefined") {
    bootstrap();
}
