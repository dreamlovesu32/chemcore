export function evaluateQualification({ reports, expectedScenarioIds, evidenceAudit }) {
  const diagnostics = [...(evidenceAudit.diagnostics || [])];
  const runIds = new Set();
  const evidenceKeys = new Set();
  for (const report of reports) {
    if (runIds.has(report.runId)) diagnostics.push(`duplicate-run-id:${report.runId}`);
    if (evidenceKeys.has(`${report.evidenceKey}:${report.runId}`)) diagnostics.push(`duplicate-evidence-record:${report.evidenceKey}:${report.runId}`);
    runIds.add(report.runId);
    evidenceKeys.add(`${report.evidenceKey}:${report.runId}`);
  }
  const expected = new Set(expectedScenarioIds);
  for (const report of reports) {
    if (!expected.has(report.scenarioId)) diagnostics.push(`unexpected-scenario:${report.scenarioId}`);
  }
  const productionCandidates = new Set(reports
    .filter((report) => report.driver === "production-black-box")
    .map((report) => report.environment?.candidateSha256)
    .filter(Boolean));
  if (productionCandidates.size !== 1) {
    diagnostics.push(`production-candidate-count:${productionCandidates.size}`);
  }
  const scenarioResults = [...expectedScenarioIds].sort().map((scenarioId) => {
    const matching = reports.filter((report) => report.scenarioId === scenarioId);
    const failedRuns = matching.filter((report) => report.status === "failed").map((report) => report.runId);
    const passedRuns = matching.filter((report) => report.status === "passed").map((report) => report.runId);
    const status = matching.length === 0 ? "missing" : failedRuns.length > 0 ? "failed" : passedRuns.length > 0 ? "passed" : "failed";
    return { scenarioId, status, runIds: matching.map((report) => report.runId), passedRuns, failedRuns };
  });
  const failedRunCount = reports.filter((report) => report.status === "failed").length;
  const missingScenarioCount = scenarioResults.filter((result) => result.status === "missing").length;
  const failedScenarioCount = scenarioResults.filter((result) => result.status === "failed").length;
  const passedScenarioCount = scenarioResults.filter((result) => result.status === "passed").length;
  const status = diagnostics.length === 0 && failedRunCount === 0 && missingScenarioCount === 0 && failedScenarioCount === 0
    ? "passed"
    : "failed";
  return {
    status,
    candidateSha256: productionCandidates.size === 1 ? [...productionCandidates][0] : null,
    reportCount: reports.length,
    expectedScenarioCount: expectedScenarioIds.length,
    passedScenarioCount,
    failedScenarioCount,
    missingScenarioCount,
    failedRunCount,
    reports: reports.map((report) => ({
      runId: report.runId,
      scenarioId: report.scenarioId,
      driver: report.driver,
      status: report.status,
      evidenceKey: report.evidenceKey,
      candidateSha256: report.environment?.candidateSha256 || null,
    })),
    scenarioResults,
    evidence: evidenceAudit.summary,
    diagnostics,
  };
}
