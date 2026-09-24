export type PreflightStatus = "not_ready" | "ready_for_preflight";
export type CheckResult = "satisfied" | "missing_observation" | "rejected";

export interface PrerequisiteCheck {
  prerequisite: string;
  result: CheckResult;
  withheld_surfaces?: string[];
}

export interface SurfaceCarriage {
  surface: string;
  preventive: Record<string, Delivery>;
  carried: Record<string, Delivery>;
}

export interface BindingSurfaces {
  surfaces: SurfaceCarriage[];
}

export type CandidateObservation =
  | { kind: "not_declared" }
  | { kind: "unattributed" }
  | { kind: "observed"; surfaces: BindingSurfaces };

export interface CandidatePreflight {
  profile: string;
  profile_revision: number;
  model: string;
  runtime: string;
  observation: CandidateObservation;
}

export interface PreflightReport {
  status: PreflightStatus;
  profile_id: string;
  checks: PrerequisiteCheck[];
  activation_pending: string[];
  candidates: CandidatePreflight[];
}

interface Delivery {
  kind: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(isString);
}

function isInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value);
}

function isStatus(value: unknown): value is PreflightStatus {
  return value === "not_ready" || value === "ready_for_preflight";
}

function isCheckResult(value: unknown): value is CheckResult {
  return value === "satisfied" || value === "missing_observation" || value === "rejected";
}

function isDelivery(value: unknown): value is Delivery {
  return isRecord(value) && isString(value.kind);
}

function isDeliveryMap(value: unknown): value is Record<string, Delivery> {
  return (
    isRecord(value) &&
    Object.values(value).every((entry) => isDelivery(entry))
  );
}

function isSurfaceCarriage(value: unknown): value is SurfaceCarriage {
  return (
    isRecord(value) &&
    isString(value.surface) &&
    isDeliveryMap(value.preventive) &&
    isDeliveryMap(value.carried)
  );
}

function isBindingSurfaces(value: unknown): value is BindingSurfaces {
  return (
    isRecord(value) &&
    Array.isArray(value.surfaces) &&
    value.surfaces.every(isSurfaceCarriage)
  );
}

function isCandidateObservation(value: unknown): value is CandidateObservation {
  if (!isRecord(value) || !isString(value.kind)) {
    return false;
  }
  if (value.kind === "not_declared" || value.kind === "unattributed") {
    return true;
  }
  return value.kind === "observed" && isBindingSurfaces(value.surfaces);
}

function isCandidate(value: unknown): value is CandidatePreflight {
  return (
    isRecord(value) &&
    isString(value.profile) &&
    isInteger(value.profile_revision) &&
    isString(value.model) &&
    isString(value.runtime) &&
    isCandidateObservation(value.observation)
  );
}

function isCheck(value: unknown): value is PrerequisiteCheck {
  return (
    isRecord(value) &&
    isString(value.prerequisite) &&
    isCheckResult(value.result) &&
    (value.withheld_surfaces === undefined ||
      isStringArray(value.withheld_surfaces))
  );
}

export function parsePreflightReport(payload: string): PreflightReport {
  const parsed: unknown = JSON.parse(payload);
  if (!isRecord(parsed)) {
    throw new Error("preflight report is not an object");
  }
  if (!isStatus(parsed.status)) {
    throw new Error("preflight report has an invalid status");
  }
  if (!isString(parsed.profile_id)) {
    throw new Error("preflight report missing profile_id");
  }
  if (!Array.isArray(parsed.checks) || !parsed.checks.every(isCheck)) {
    throw new Error("preflight report has invalid checks");
  }
  if (!isStringArray(parsed.activation_pending)) {
    throw new Error("preflight report has invalid activation gates");
  }
  if (!Array.isArray(parsed.candidates) || !parsed.candidates.every(isCandidate)) {
    throw new Error("preflight report has invalid candidates");
  }
  return {
    status: parsed.status,
    profile_id: parsed.profile_id,
    checks: parsed.checks,
    activation_pending: parsed.activation_pending,
    candidates: parsed.candidates,
  };
}

function surfaceIsWithheld(surface: SurfaceCarriage): boolean {
  return (
    Object.values(surface.preventive).some((delivery) => delivery.kind === "missing") ||
    Object.values(surface.carried).some((delivery) => delivery.kind !== "declared")
  );
}

/** The surfaces for which an observed candidate carries no demanded carrier. */
export function withheldSurfaces(candidate: CandidatePreflight): string[] {
  if (candidate.observation.kind !== "observed") {
    return [];
  }
  return candidate.observation.surfaces.surfaces
    .filter(surfaceIsWithheld)
    .map((surface) => surface.surface);
}

/** A refusal with no runtime observation is visibly empty, not silently ready. */
export function observationSummary(candidate: CandidatePreflight): string {
  switch (candidate.observation.kind) {
    case "not_declared":
      return "no runtime declaration; surface evidence is empty";
    case "unattributed":
      return "runtime observation is not attributable to this Host; surface evidence is empty";
    case "observed": {
      const withheld = withheldSurfaces(candidate);
      return withheld.length === 0
        ? "all demanded surface carriers are declared"
        : `withheld: ${withheld.join(", ")}`;
    }
  }
}
