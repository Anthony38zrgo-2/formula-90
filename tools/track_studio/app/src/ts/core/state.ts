// Minimal UI state store for the presentation layer.
//
// Holds UI-facing state (current track id, diagnostics) and notifies
// subscribers. It deliberately does not own persisted data or domain rules.

import type { Diagnostic } from "./api";

type Listener = () => void;

export class AppState {
  private _trackId = "untitled";
  private _diagnostics: Diagnostic[] = [];
  private readonly listeners = new Set<Listener>();

  get trackId(): string {
    return this._trackId;
  }

  get diagnostics(): Diagnostic[] {
    return this._diagnostics;
  }

  setTrackId(trackId: string): void {
    this._trackId = trackId;
    this.notify();
  }

  setDiagnostics(diagnostics: Diagnostic[]): void {
    this._diagnostics = diagnostics;
    this.notify();
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }
}

export const appState = new AppState();
