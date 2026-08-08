export const DEFAULT_RESOURCE_LIMITS = Object.freeze({ cpuUnits: 10, memoryGiB: 30 });

export class ResourceBudget {
  constructor(limits = DEFAULT_RESOURCE_LIMITS) {
    this.limits = { ...limits };
    this.allocations = new Map();
  }

  usage() {
    return [...this.allocations.values()].reduce(
      (total, allocation) => ({
        cpuUnits: total.cpuUnits + allocation.cpuUnits,
        memoryGiB: total.memoryGiB + allocation.memoryGiB,
      }),
      { cpuUnits: 0, memoryGiB: 0 },
    );
  }

  canAdmit(request) {
    const usage = this.usage();
    return usage.cpuUnits + request.cpuUnits <= this.limits.cpuUnits
      && usage.memoryGiB + request.memoryGiB <= this.limits.memoryGiB;
  }

  admit(id, request) {
    if (this.allocations.has(id)) {
      throw new Error(`Worker ${id} already has a resource allocation.`);
    }
    if (!this.canAdmit(request)) {
      throw new Error(`Resource budget exceeded by ${id}: requested ${request.cpuUnits} CPU/${request.memoryGiB} GiB; current ${JSON.stringify(this.usage())}; limit ${JSON.stringify(this.limits)}.`);
    }
    this.allocations.set(id, { ...request });
    return this.usage();
  }

  release(id) {
    if (!this.allocations.delete(id)) {
      throw new Error(`Worker ${id} has no resource allocation.`);
    }
    return this.usage();
  }
}
