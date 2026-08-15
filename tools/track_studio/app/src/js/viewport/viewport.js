// Viewport placeholder: hosts the plan 2D surface and the active tool.
import { SelectTool } from "../tools/select-tool";
export class Viewport {
    constructor(container) {
        this.tool = new SelectTool();
        this.canvas = document.createElement("canvas");
        this.canvas.width = 800;
        this.canvas.height = 500;
        container.appendChild(this.canvas);
        this.canvas.addEventListener("pointerdown", (event) => this.tool.onPointerDown(event.offsetX, event.offsetY));
        this.canvas.addEventListener("pointermove", (event) => this.tool.onPointerMove(event.offsetX, event.offsetY));
        this.canvas.addEventListener("pointerup", (event) => this.tool.onPointerUp(event.offsetX, event.offsetY));
    }
    setTool(tool) {
        this.tool = tool;
    }
}
