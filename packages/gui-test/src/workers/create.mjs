import { HyperVCoordinator } from "./hyperv.mjs";
import { PhysicalWindowsCoordinator } from "./physical-windows.mjs";

export function createWorkerCoordinator(profile, options = {}) {
  if (profile?.kind === "hyper-v") return new HyperVCoordinator(profile, options);
  if (profile?.kind === "physical-windows") return new PhysicalWindowsCoordinator(profile, options);
  throw new Error(`Unsupported GUI worker kind ${JSON.stringify(profile?.kind)}.`);
}
