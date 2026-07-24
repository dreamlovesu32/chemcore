import initNmr, {
  capabilities_json as capabilitiesJson,
  predict_json as predictJson,
} from "./nmr-engine/chemsema_nmr.js";

export function createBundledNmrProvider(runtime = {}) {
  const initialize = runtime.initialize || initNmr;
  const predict = runtime.predictJson || predictJson;
  const capabilities = runtime.capabilitiesJson || capabilitiesJson;
  let initialization = null;

  function ready() {
    initialization ||= Promise.resolve(initialize());
    return initialization;
  }

  return {
    async capabilities() {
      await ready();
      return JSON.parse(capabilities());
    },

    async predict(request) {
      await ready();
      return JSON.parse(predict(JSON.stringify(request)));
    },
  };
}
