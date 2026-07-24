export function createNmrPredictionHost(options) {
  async function predict(nucleus, conditionOverrides = {}) {
    const engine = options.engine();
    if (!engine?.nmrPredictionRequestJson || !engine?.nmrResultDocumentJson) {
      throw new Error("This ChemSema build does not include the NMR document adapter.");
    }
    const provider = options.provider?.() || globalThis.ChemSemaNmr;
    if (!provider?.predict) {
      throw new Error("ChemSema NMR prediction rules are not installed yet.");
    }
    const request = JSON.parse(await engine.nmrPredictionRequestJson(nucleus));
    if (request.conditions || Object.keys(conditionOverrides).length > 0) {
      request.conditions = predictionConditions(request.conditions || {}, conditionOverrides);
    }
    const response = await provider.predict(request);
    const responseJson = typeof response === "string" ? response : JSON.stringify(response);
    const documentJson = await engine.nmrResultDocumentJson(responseJson);
    const title = nucleus === "13C"
      ? "ChemNMR 13C Estimation"
      : "ChemNMR 1H Estimation";
    await options.openDocumentTab(JSON.parse(documentJson), title);
    return true;
  }

  return { predict };
}

function predictionConditions(defaults, overrides) {
  const conditions = { ...defaults, ...overrides };
  if (!["CDCl3", "DMSO-d6"].includes(conditions.solvent)) {
    throw new Error(`Unsupported NMR solvent '${conditions.solvent}'.`);
  }
  if (!Number.isFinite(conditions.frequencyMHz) || conditions.frequencyMHz <= 0) {
    throw new Error("NMR frequency must be a positive finite number.");
  }
  if (!Number.isFinite(conditions.temperatureKelvin) || conditions.temperatureKelvin <= 0) {
    throw new Error("NMR temperature must be a positive finite number.");
  }
  return conditions;
}
