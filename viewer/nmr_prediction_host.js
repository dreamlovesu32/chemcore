export function createNmrPredictionHost(options) {
  async function predict(nucleus) {
    const engine = options.engine();
    if (!engine?.nmrPredictionRequestJson || !engine?.nmrResultDocumentJson) {
      throw new Error("This ChemSema build does not include the NMR document adapter.");
    }
    const requestJson = await engine.nmrPredictionRequestJson(nucleus);
    const provider = options.provider?.() || globalThis.ChemSemaNmr;
    if (!provider?.predict) {
      throw new Error("ChemSema NMR prediction rules are not installed yet.");
    }
    const response = await provider.predict(JSON.parse(requestJson));
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
