// A simple tool interface used by the viewport tool palette.

export interface Tool {
  readonly id: string;
  readonly label: string;
  onPointerDown(_x: number, _z: number): void;
  onPointerMove(_x: number, _z: number): void;
  onPointerUp(_x: number, _z: number): void;
}

export class SelectTool implements Tool {
  readonly id = "select";
  readonly label = "Select";

  onPointerDown(x: number, z: number): void {
    console.debug(`select down at (${x.toFixed(2)}, ${z.toFixed(2)})`);
  }

  onPointerMove(x: number, z: number): void {
    console.debug(`select move at (${x.toFixed(2)}, ${z.toFixed(2)})`);
  }

  onPointerUp(x: number, z: number): void {
    console.debug(`select up at (${x.toFixed(2)}, ${z.toFixed(2)})`);
  }
}
