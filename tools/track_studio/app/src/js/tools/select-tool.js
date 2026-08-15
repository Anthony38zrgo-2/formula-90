// A simple tool interface used by the viewport tool palette.
export class SelectTool {
    constructor() {
        this.id = "select";
        this.label = "Select";
    }
    onPointerDown(x, z) {
        console.debug(`select down at (${x.toFixed(2)}, ${z.toFixed(2)})`);
    }
    onPointerMove(x, z) {
        console.debug(`select move at (${x.toFixed(2)}, ${z.toFixed(2)})`);
    }
    onPointerUp(x, z) {
        console.debug(`select up at (${x.toFixed(2)}, ${z.toFixed(2)})`);
    }
}
