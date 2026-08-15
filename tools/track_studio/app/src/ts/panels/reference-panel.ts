// Reference layer panel: add a reference image and calibrate it.

import { api } from "../core/api";

export function setupReferencePanel(
  container: HTMLElement,
  addButton: HTMLButtonElement,
  imagePathInput: HTMLInputElement,
  xInput: HTMLInputElement,
  zInput: HTMLInputElement,
  output: HTMLElement,
): void {
  addButton.addEventListener("click", async () => {
    const imagePath = imagePathInput.value.trim();
    const x = Number(xInput.value);
    const z = Number(zInput.value);
    try {
      await api.referenceAdd(imagePath, x, z, 1.0);
      output.textContent = `Added reference layer: ${imagePath}`;
    } catch (error) {
      output.textContent = `Reference add failed: ${String(error)}`;
    }
  });

  void container;
}

export async function calibrateReference(
  layerId: string,
  imageDistancePx: number,
  realDistanceM: number,
  output: HTMLElement,
): Promise<void> {
  try {
    await api.referenceCalibrate(layerId, imageDistancePx, realDistanceM);
    output.textContent = `Calibrated ${layerId}: ${imageDistancePx}px / ${realDistanceM}m`;
  } catch (error) {
    output.textContent = `Calibration failed: ${String(error)}`;
  }
}
