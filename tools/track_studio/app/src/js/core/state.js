// Minimal UI state store for the presentation layer.
//
// Holds UI-facing state (current track id, diagnostics) and notifies
// subscribers. It deliberately does not own persisted data or domain rules.
export class AppState {
    constructor() {
        this._trackId = "untitled";
        this._diagnostics = [];
        this.listeners = new Set();
    }
    get trackId() {
        return this._trackId;
    }
    get diagnostics() {
        return this._diagnostics;
    }
    setTrackId(trackId) {
        this._trackId = trackId;
        this.notify();
    }
    setDiagnostics(diagnostics) {
        this._diagnostics = diagnostics;
        this.notify();
    }
    subscribe(listener) {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
    }
    notify() {
        for (const listener of this.listeners) {
            listener();
        }
    }
}
export const appState = new AppState();
