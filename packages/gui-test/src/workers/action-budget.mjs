export const minimumCandidateActionBudgetMs = 30000;
export const candidateActionTransportReserveMs = 15000;

export function candidateActionBudgetIsValid(budgetMs, completionTimeoutMs) {
  return Number.isInteger(budgetMs)
    && budgetMs >= minimumCandidateActionBudgetMs
    && Number.isInteger(completionTimeoutMs)
    && completionTimeoutMs + candidateActionTransportReserveMs <= budgetMs;
}
