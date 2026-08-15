// Viewport placeholder: hosts the plan 2D surface and the active tool.

import type { Tool } from "../tools/select-tool";
import { SelectTool } from "../tools/select-tool";

export class Viewport {
  private readonly canvas: HTMLCanvasElement;
  private tool: Tool = new SelectTool();

  constructor(container: HTMLElement) {
    this.canvas = document.createElement("canvas");
    this.canvas.width = 800;
    this.canvas.height = 500;
    container.appendChild(this.canvas);
    this.canvas.addEventListener("pointerdown", (event) =>
      this.tool.onPointerDown(event.offsetX, event.offsetY),
    );
    this.canvas.addEventListener("pointermove", (event) =>
      this.tool.onPointerMove(event.offsetX, event.offsetY),
    );
    this.canvas.addEventListener("pointerup", (event) =>
      this.tool.onPointerUp(event.offsetX, event.offsetY),
    );
  }

  setTool(tool: Tool): void {
    this.tool = tool;
  }
}
