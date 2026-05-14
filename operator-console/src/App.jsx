import React, { useEffect, useMemo, useState } from "react";

const DEFAULT_API_BASE = "http://localhost:8080";
const DEFAULT_PROFILE_ID = "00000000-0000-0000-0000-000000000001";
const STORAGE_KEYS = {
  apiBase: "apiBase",
  token: "token",
  profileId: "profileId",
  artifactTypeFilter: "artifactTypeFilter",
  statusFilter: "statusFilter",
  fitFilter: "fitFilter",
};

const STATUS_PRESETS = [
  { label: "Active", value: "" },
  { label: "All", value: "all" },
  { label: "Discovered", value: "Discovered" },
  { label: "Parsed", value: "Parsed" },
  { label: "Scored", value: "Scored" },
  { label: "Applied", value: "Applied" },
  { label: "Queued for Review", value: "QueuedForReview" },
  { label: "Skipped", value: "Skipped" },
  { label: "Rejected", value: "Rejected" },
  { label: "Archived", value: "Archived" },
  { label: "Closed", value: "Closed" },
];

const ARTIFACT_TYPE_PRESETS = [
  { label: "All", value: "" },
  { label: "readable_web_page", value: "readable_web_page" },
  { label: "search_results", value: "search_results" },
  { label: "job_fit_report", value: "job_fit_report" },
  { label: "cover_letter", value: "cover_letter" },
  { label: "daily_report", value: "daily_report" },
];

const FIT_FILTER_PRESETS = [
  { label: "All", value: "" },
  { label: "Primary Fit 75+", value: "primary75" },
  { label: "OE Fit 75+", value: "oe75" },
  { label: "Remote Only", value: "remote" },
  { label: "Needs Review", value: "needsReview" },
  { label: "Rejected / Archived", value: "final" },
];

const MAIN_TABS = [
  { label: "Operator", value: "operator" },
  { label: "Tasks", value: "tasks" },
  { label: "Employment", value: "opportunities" },
  { label: "Artifacts", value: "artifacts" },
];

const STATUS_REASON_PRESETS = [
  "Not remote",
  "Too admin-heavy for primary",
  "Too high-risk for OE",
  "Rejection received",
  "Test record",
  "Compensation too low",
  "Heavy meetings/on-call",
];

const RISK_FLAG_LABELS = {
  on_site_or_hybrid: "On-site / hybrid",
  heavy_meetings: "Heavy meetings",
  on_call: "On-call",
  sole_owner: "Sole owner",
  client_facing: "Client-facing",
  strict_availability: "Strict availability",
  heavy_travel: "Heavy travel",
  conflict_risk: "Conflict risk",
  unclear_scope: "Unclear scope",
};

const styles = {
  page: {
    minHeight: "100vh",
    background: "#f6f7fb",
    color: "#111827",
    fontFamily:
      'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    padding: 24,
  },
  shell: {
    maxWidth: 1400,
    margin: "0 auto",
    display: "flex",
    flexDirection: "column",
    gap: 16,
  },
  header: {
    background: "white",
    borderRadius: 18,
    padding: 20,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.08)",
    display: "grid",
    gridTemplateColumns: "1fr minmax(280px, 520px)",
    gap: 16,
    alignItems: "center",
  },
  profilePanel: {
    background: "white",
    borderRadius: 18,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "grid",
    gridTemplateColumns: "minmax(220px, 0.4fr) minmax(280px, 1fr) auto",
    gap: 12,
    alignItems: "center",
  },
  title: {
    fontSize: 28,
    margin: "4px 0 6px",
    letterSpacing: "-0.03em",
  },
  subtitle: {
    margin: 0,
    color: "#64748b",
    fontSize: 14,
  },
  eyebrow: {
    color: "#64748b",
    fontWeight: 700,
    fontSize: 13,
    textTransform: "uppercase",
    letterSpacing: "0.08em",
  },
  controls: {
    display: "flex",
    flexDirection: "column",
    gap: 8,
  },
  chatPanel: {
    background: "white",
    borderRadius: 18,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "grid",
    gridTemplateColumns: "minmax(260px, 1fr) auto",
    gap: 10,
    alignItems: "end",
  },
  chatMessages: {
    gridColumn: "1 / -1",
    display: "flex",
    flexDirection: "column",
    gap: 8,
    maxHeight: 220,
    overflow: "auto",
  },
  chatMessage: {
    borderRadius: 12,
    padding: 10,
    fontSize: 14,
    lineHeight: 1.45,
    border: "1px solid #e2e8f0",
    background: "#f8fafc",
    overflowWrap: "anywhere",
  },
  chatMessageUser: {
    background: "#eef2ff",
    borderColor: "#c7d2fe",
  },
  chatMessageAssistant: {
    background: "#f8fafc",
    borderColor: "#e2e8f0",
  },
  taskGrid: {
    display: "grid",
    gridTemplateColumns: "minmax(260px, 0.9fr) minmax(320px, 1.1fr)",
    gap: 16,
    alignItems: "start",
  },
  taskRunList: {
    display: "flex",
    flexDirection: "column",
    gap: 8,
    marginTop: 10,
  },
  row: {
    display: "flex",
    gap: 8,
  },
  input: {
    width: "100%",
    border: "1px solid #d1d5db",
    borderRadius: 12,
    padding: "10px 12px",
    fontSize: 14,
    background: "white",
    color: "#111827",
  },
  button: {
    border: "1px solid #cbd5e1",
    borderRadius: 12,
    padding: "10px 12px",
    fontSize: 14,
    fontWeight: 700,
    background: "#111827",
    color: "white",
    cursor: "pointer",
    whiteSpace: "nowrap",
  },
  buttonSecondary: {
    border: "1px solid #cbd5e1",
    borderRadius: 12,
    padding: "8px 10px",
    fontSize: 13,
    fontWeight: 700,
    background: "white",
    color: "#111827",
    cursor: "pointer",
    whiteSpace: "nowrap",
  },
  buttonDanger: {
    border: "1px solid #fecdd3",
    borderRadius: 12,
    padding: "8px 10px",
    fontSize: 13,
    fontWeight: 700,
    background: "#fff1f2",
    color: "#9f1239",
    cursor: "pointer",
    whiteSpace: "nowrap",
  },
  grid4: {
    display: "grid",
    gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
    gap: 12,
  },
  dashboardGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(6, minmax(0, 1fr))",
    gap: 12,
  },
  metricCard: {
    background: "white",
    borderRadius: 16,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
  },
  metricLabel: {
    color: "#64748b",
    fontSize: 13,
    marginBottom: 6,
  },
  metricValue: {
    fontSize: 24,
    fontWeight: 800,
  },
  message: {
    borderRadius: 14,
    padding: 12,
    fontSize: 14,
    fontWeight: 600,
  },
  error: {
    background: "#fff1f2",
    color: "#9f1239",
    border: "1px solid #fecdd3",
  },
  notice: {
    background: "#ecfdf5",
    color: "#065f46",
    border: "1px solid #a7f3d0",
  },
  duplicateWarning: {
    marginTop: 10,
    borderRadius: 12,
    padding: 10,
    background: "#fffbeb",
    color: "#92400e",
    border: "1px solid #fde68a",
    fontSize: 13,
    fontWeight: 700,
    lineHeight: 1.35,
  },
  filterBar: {
    background: "white",
    borderRadius: 18,
    padding: 14,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "grid",
    gridTemplateColumns: "minmax(260px, 1fr) 1.4fr 1fr 1.2fr",
    gap: 12,
    alignItems: "start",
  },
  criteriaPanel: {
    background: "white",
    borderRadius: 18,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "grid",
    gridTemplateColumns: "minmax(220px, 0.45fr) minmax(320px, 1fr) auto",
    gap: 12,
    alignItems: "start",
  },
  textarea: {
    width: "100%",
    minHeight: 92,
    border: "1px solid #d1d5db",
    borderRadius: 12,
    padding: "10px 12px",
    fontSize: 14,
    background: "white",
    color: "#111827",
    resize: "vertical",
    lineHeight: 1.45,
    fontFamily: "inherit",
  },
  mainTabs: {
    background: "white",
    borderRadius: 18,
    padding: 10,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "flex",
    gap: 8,
    flexWrap: "wrap",
  },
  presetGroup: {
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  presetButtons: {
    display: "flex",
    gap: 6,
    flexWrap: "wrap",
  },
  presetButton: {
    border: "1px solid #cbd5e1",
    borderRadius: 999,
    padding: "7px 9px",
    fontSize: 12,
    fontWeight: 800,
    background: "white",
    color: "#334155",
    cursor: "pointer",
    whiteSpace: "nowrap",
  },
  activePresetButton: {
    background: "#111827",
    borderColor: "#111827",
    color: "white",
  },
  jobUrlPanel: {
    background: "white",
    borderRadius: 18,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    display: "grid",
    gridTemplateColumns: "auto minmax(320px, 1fr) auto auto",
    gap: 10,
    alignItems: "center",
  },
  label: {
    color: "#334155",
    fontSize: 14,
    fontWeight: 800,
    whiteSpace: "nowrap",
  },
  mainGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: 16,
  },
  sectionTitle: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    marginBottom: 10,
  },
  h2: {
    fontSize: 18,
    margin: 0,
  },
  list: {
    display: "flex",
    flexDirection: "column",
    gap: 10,
  },
  card: {
    background: "white",
    borderRadius: 16,
    padding: 16,
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
    border: "1px solid transparent",
  },
  selectedCard: {
    border: "1px solid #334155",
    boxShadow: "0 6px 24px rgba(15, 23, 42, 0.12)",
  },
  cardHeader: {
    display: "flex",
    justifyContent: "space-between",
    gap: 12,
    alignItems: "flex-start",
  },
  cardTitle: {
    fontWeight: 800,
    fontSize: 16,
    marginBottom: 4,
  },
  muted: {
    color: "#64748b",
    fontSize: 13,
  },
  text: {
    color: "#334155",
    fontSize: 14,
    lineHeight: 1.5,
  },
  badge: {
    display: "inline-flex",
    borderRadius: 999,
    padding: "4px 8px",
    fontSize: 12,
    fontWeight: 800,
    background: "#f1f5f9",
    color: "#334155",
  },
  badgeGood: {
    background: "#dcfce7",
    color: "#166534",
  },
  badgeWarn: {
    background: "#fef3c7",
    color: "#92400e",
  },
  badgeBad: {
    background: "#ffe4e6",
    color: "#9f1239",
  },
  badgeInfo: {
    background: "#dbeafe",
    color: "#1d4ed8",
  },
  riskFlags: {
    display: "flex",
    gap: 6,
    flexWrap: "wrap",
    marginTop: 10,
  },
  actions: {
    display: "flex",
    gap: 8,
    flexWrap: "wrap",
    marginTop: 12,
  },
  tabs: {
    display: "flex",
    gap: 6,
    flexWrap: "wrap",
    marginTop: 14,
    marginBottom: 12,
  },
  tab: {
    border: "1px solid #cbd5e1",
    borderRadius: 999,
    padding: "7px 10px",
    fontSize: 13,
    fontWeight: 800,
    background: "white",
    color: "#334155",
    cursor: "pointer",
  },
  activeTab: {
    background: "#111827",
    borderColor: "#111827",
    color: "white",
  },
  detailsGrid: {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    gap: 16,
  },
  fieldGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    gap: 10,
  },
  field: {
    background: "#f8fafc",
    border: "1px solid #e2e8f0",
    borderRadius: 12,
    padding: 10,
    minWidth: 0,
  },
  fieldLabel: {
    color: "#64748b",
    fontSize: 12,
    fontWeight: 800,
    marginBottom: 4,
  },
  fieldValue: {
    color: "#111827",
    fontSize: 14,
    lineHeight: 1.35,
    overflowWrap: "anywhere",
  },
  debugDetails: {
    marginTop: 10,
  },
  debugSummary: {
    cursor: "pointer",
    color: "#334155",
    fontSize: 13,
    fontWeight: 800,
    marginBottom: 8,
  },
  pre: {
    maxHeight: 360,
    overflow: "auto",
    whiteSpace: "pre-wrap",
    background: "#020617",
    color: "#e2e8f0",
    borderRadius: 14,
    padding: 14,
    fontSize: 12,
    lineHeight: 1.5,
  },
  empty: {
    background: "white",
    borderRadius: 16,
    padding: 24,
    color: "#64748b",
    boxShadow: "0 1px 8px rgba(15, 23, 42, 0.06)",
  },
};

function authHeaders(token) {
  return token?.trim() ? { Authorization: `Bearer ${token.trim()}` } : {};
}

async function apiFetch(apiBase, token, path, options = {}) {
  const response = await fetch(`${apiBase.replace(/\/$/, "")}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(token),
      ...(options.headers || {}),
    },
  });

  const text = await response.text();
  let data = null;

  if (text) {
    try {
      data = JSON.parse(text);
    } catch {
      data = { raw: text };
    }
  }

  if (!response.ok) {
    const message = data?.error || data?.message || `${response.status} ${response.statusText}`;
    throw new Error(message);
  }

  return data;
}

function safeDate(value) {
  if (!value) return "—";
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function compactText(value, max = 180) {
  if (!value) return "—";
  return value.length > max ? `${value.slice(0, max)}…` : value;
}

function statusStyle(status) {
  const normalized = String(status || "").toLowerCase();
  if (["scored", "parsed", "succeeded"].includes(normalized)) {
    return { ...styles.badge, ...styles.badgeGood };
  }
  if (["discovered", "queuedforreview", "queued_for_review"].includes(normalized)) {
    return { ...styles.badge, ...styles.badgeWarn };
  }
  if (["failed", "skipped", "closed"].includes(normalized)) {
    return { ...styles.badge, ...styles.badgeBad };
  }
  return styles.badge;
}

function formatSalary(item) {
  if (!item?.salary_min && !item?.salary_max) return "—";
  const min = item.salary_min ? `$${Number(item.salary_min).toLocaleString()}` : "—";
  const max = item.salary_max ? `$${Number(item.salary_max).toLocaleString()}` : "—";
  return `${min} - ${max}`;
}

function formatScore(value) {
  return value === null || value === undefined ? "—" : Number(value).toLocaleString();
}

function formatTrack(value) {
  if (!value) return "—";
  return String(value)
    .replace(/_/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function riskFlagLabel(value) {
  return RISK_FLAG_LABELS[value] || formatTrack(value);
}

function stringifyJson(value) {
  if (!value) return "{}";
  return JSON.stringify(value, null, 2);
}

function isToday(value) {
  if (!value) return false;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return false;

  const now = new Date();
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

function normalizeStatus(value) {
  return String(value || "")
    .replace(/_/g, "")
    .toLowerCase();
}

function normalizeStatusFilterValue(value) {
  const normalized = normalizeStatus(value);
  return STATUS_PRESETS.find((preset) => normalizeStatus(preset.value) === normalized)?.value || "";
}

function readStoredSetting(key, fallback) {
  try {
    return window.localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

function writeStoredSetting(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Ignore storage errors so the console keeps working in private/restricted browsers.
  }
}

function profileApiPath(profileId, path) {
  return `/api/employment/profiles/${profileId || DEFAULT_PROFILE_ID}${path}`;
}

function getArtifactSourceUrl(artifact) {
  return artifact?.location || artifact?.metadata?.source_url || artifact?.content_json?.source_url || "";
}

function findDuplicateOpportunityForArtifact(artifact, opportunities) {
  const artifactId = artifact?.id;
  const sourceUrl = getArtifactSourceUrl(artifact).trim();

  return (
    opportunities.find((opportunity) => {
      const artifactMatch = artifactId && opportunity.source_artifact_id === artifactId;
      const sourceUrlMatch =
        sourceUrl && opportunity.source_url && opportunity.source_url.trim() === sourceUrl;

      return artifactMatch || sourceUrlMatch;
    }) || null
  );
}

function describeOpportunity(opportunity) {
  if (!opportunity) return "Untitled opportunity";

  return [
    opportunity.title || "Untitled opportunity",
    opportunity.company || "Unknown company",
    opportunity.status || "unknown status",
  ].join(" · ");
}

export default function App() {
  const [apiBase, setApiBase] = useState(() => readStoredSetting(STORAGE_KEYS.apiBase, DEFAULT_API_BASE));
  const [token, setToken] = useState(() => readStoredSetting(STORAGE_KEYS.token, ""));
  const [profiles, setProfiles] = useState([]);
  const [selectedProfileId, setSelectedProfileId] = useState(() =>
    readStoredSetting(STORAGE_KEYS.profileId, DEFAULT_PROFILE_ID),
  );
  const [criteriaDraft, setCriteriaDraft] = useState("");
  const [opportunities, setOpportunities] = useState([]);
  const [allOpportunities, setAllOpportunities] = useState([]);
  const [artifacts, setArtifacts] = useState([]);
  const [todayStats, setTodayStats] = useState({
    artifacts: 0,
    opportunities: 0,
    needsParse: 0,
    needsScore: 0,
    highFit: 0,
    failedRuns: 0,
  });
  const [selectedOpportunity, setSelectedOpportunity] = useState(null);
  const [selectedArtifact, setSelectedArtifact] = useState(null);
  const [artifactContent, setArtifactContent] = useState(null);
  const [opTasks, setOpTasks] = useState([]);
  const [taskRunsByTaskId, setTaskRunsByTaskId] = useState({});
  const [jobUrl, setJobUrl] = useState("");
  const [opportunityDetailTab, setOpportunityDetailTab] = useState("summary");
  const [artifactDetailTab, setArtifactDetailTab] = useState("text");
  const [activeMainTab, setActiveMainTab] = useState("operator");
  const [coverLetterDirection, setCoverLetterDirection] = useState("");
  const [coverLetterDraft, setCoverLetterDraft] = useState("");
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState([]);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState(() =>
    normalizeStatusFilterValue(readStoredSetting(STORAGE_KEYS.statusFilter, "")),
  );
  const [artifactTypeFilter, setArtifactTypeFilter] = useState(() =>
    readStoredSetting(STORAGE_KEYS.artifactTypeFilter, "readable_web_page"),
  );
  const [fitFilter, setFitFilter] = useState(() => readStoredSetting(STORAGE_KEYS.fitFilter, ""));
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState("");
  const [chatLoading, setChatLoading] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const filteredOpportunities = useMemo(() => {
    const q = query.trim().toLowerCase();
    return opportunities.filter((item) => {
      const queryMatch =
        !q ||
        [item.title, item.company, item.location, item.source_url, item.description_text, item.score_reason]
          .filter(Boolean)
          .some((value) => String(value).toLowerCase().includes(q));
      if (!queryMatch) return false;

      if (fitFilter === "primary75") return Number(item.primary_fit_score || 0) >= 75;
      if (fitFilter === "oe75") return Number(item.oe_fit_score || 0) >= 75;
      if (fitFilter === "remote") return String(item.remote_type || "").toLowerCase() === "remote";
      if (fitFilter === "needsReview") {
        return (
          formatTrack(item.recommended_track).toLowerCase() === "manual review" ||
          (item.risk_flags || []).some((flag) =>
            ["conflict_risk", "strict_availability", "unclear_scope"].includes(flag),
          )
        );
      }
      if (fitFilter === "final") return ["rejected", "archived"].includes(normalizeStatus(item.status));
      return true;
    });
  }, [opportunities, query, fitFilter]);

  const filteredArtifacts = useMemo(() => {
    const q = query.trim().toLowerCase();
    return artifacts.filter((item) => {
      if (!q) return true;
      return [item.name, item.artifact_type, item.location, item.metadata?.title, item.metadata?.source_url]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(q));
    });
  }, [artifacts, query]);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === selectedProfileId) || null,
    [profiles, selectedProfileId],
  );
  const taskRuns = useMemo(
    () => Object.values(taskRunsByTaskId).flatMap((runs) => runs || []),
    [taskRunsByTaskId],
  );
  const criteriaChanged = criteriaDraft !== (selectedProfile?.criteria || "");

  async function loadTaskData() {
    const tasksResponse = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, "/op-tasks"));
    const tasks = tasksResponse?.items || [];
    const runResponses = await Promise.all(
      tasks.map((task) =>
        apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/op-tasks/${task.id}/runs`)).catch(() => ({
          runs: [],
        })),
      ),
    );
    const runsByTaskId = {};
    tasks.forEach((task, index) => {
      runsByTaskId[task.id] = runResponses[index]?.runs || [];
    });

    return {
      tasks,
      runsByTaskId,
      runs: runResponses.flatMap((response) => response?.runs || []),
    };
  }

  const computeTodayStats = (allOpportunities, allArtifacts, runs) => {
    return {
      artifacts: allArtifacts.filter((artifact) => isToday(artifact.created_at)).length,
      opportunities: allOpportunities.filter((opportunity) => isToday(opportunity.first_seen_at)).length,
      needsParse: allOpportunities.filter((opportunity) =>
        ["discovered", "queuedforreview"].includes(normalizeStatus(opportunity.status)),
      ).length,
      needsScore: allOpportunities.filter((opportunity) => normalizeStatus(opportunity.status) === "parsed").length,
      highFit: allOpportunities.filter(
        (opportunity) =>
          Number(opportunity.primary_fit_score || opportunity.fit_score || 0) >= 80 ||
          Number(opportunity.oe_fit_score || 0) >= 80,
      ).length,
      failedRuns: runs.filter((run) => normalizeStatus(run.status) === "failed" && isToday(run.completed_at || run.started_at)).length,
    };
  };

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.apiBase, apiBase);
  }, [apiBase]);

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.token, token);
  }, [token]);

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.profileId, selectedProfileId);
  }, [selectedProfileId]);

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.artifactTypeFilter, artifactTypeFilter);
  }, [artifactTypeFilter]);

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.statusFilter, statusFilter);
  }, [statusFilter]);

  useEffect(() => {
    writeStoredSetting(STORAGE_KEYS.fitFilter, fitFilter);
  }, [fitFilter]);

  async function loadProfiles() {
    try {
      const response = await apiFetch(apiBase, token, "/api/employment/profiles");
      const profileItems = response?.profiles || [];
      setProfiles(profileItems);

      if (profileItems.length && !profileItems.some((profile) => profile.id === selectedProfileId)) {
        setSelectedProfileId(profileItems[0].id);
      }
    } catch (err) {
      setError(err.message || "Failed to load employment profiles.");
    }
  }

  async function createProfile() {
    const displayName = window.prompt("Profile name");
    if (!displayName?.trim()) return;

    setActionLoading("create-profile");
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, "/api/employment/profiles", {
        method: "POST",
        body: JSON.stringify({ display_name: displayName.trim() }),
      });
      const profile = response?.profile;
      if (!profile?.id) {
        throw new Error("Profile was created without an id.");
      }

      setProfiles((items) => [...items, profile].sort((a, b) => a.display_name.localeCompare(b.display_name)));
      setSelectedProfileId(profile.id);
      setSelectedOpportunity(null);
      setSelectedArtifact(null);
      setArtifactContent(null);
      setNotice("Created profile.");
    } catch (err) {
      setError(err.message || "Failed to create profile.");
    } finally {
      setActionLoading("");
    }
  }

  async function saveProfileCriteria() {
    if (!selectedProfileId) return;

    setActionLoading("save-criteria");
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, `/api/employment/profiles/${selectedProfileId}`, {
        method: "PUT",
        body: JSON.stringify({ criteria: criteriaDraft }),
      });
      const profile = response?.profile;
      if (!profile?.id) {
        throw new Error("Profile update did not return a profile.");
      }

      setProfiles((items) => items.map((item) => (item.id === profile.id ? profile : item)));
      setNotice("Saved profile criteria.");
    } catch (err) {
      setError(err.message || "Failed to save profile criteria.");
    } finally {
      setActionLoading("");
    }
  }

  function mergeOpportunity(opportunity) {
    if (!opportunity?.id) return;

    const upsert = (items) => {
      const exists = items.some((item) => item.id === opportunity.id);
      if (!exists) return [opportunity, ...items];
      return items.map((item) => (item.id === opportunity.id ? opportunity : item));
    };

    setOpportunities(upsert);
    setAllOpportunities(upsert);
    setSelectedOpportunity((current) => {
      if (!current || current.id === opportunity.id) return opportunity;
      return current;
    });
  }

  async function loadAll() {
    setLoading(true);
    setError("");
    setNotice("");

    try {
      const opportunityPath = statusFilter
        ? `${profileApiPath(selectedProfileId, "/opportunities")}?status=${encodeURIComponent(statusFilter)}`
        : profileApiPath(selectedProfileId, "/opportunities");

      const artifactParams = new URLSearchParams();
      if (artifactTypeFilter) artifactParams.set("artifact_type", artifactTypeFilter);
      artifactParams.set("limit", "50");
      artifactParams.set("include_content", "false");

      const [opportunityResponse, artifactResponse, dashboardOpportunityResponse, dashboardArtifactResponse] =
        await Promise.all([
        apiFetch(apiBase, token, opportunityPath),
        apiFetch(apiBase, token, `${profileApiPath(selectedProfileId, "/op-task-artifacts")}?${artifactParams.toString()}`),
        apiFetch(apiBase, token, `${profileApiPath(selectedProfileId, "/opportunities")}?limit=200`),
        apiFetch(
          apiBase,
          token,
          `${profileApiPath(selectedProfileId, "/op-task-artifacts")}?limit=200&include_content=false`,
        ),
      ]);

      const allOpportunityItems = dashboardOpportunityResponse?.opportunities || [];
      const allArtifacts = dashboardArtifactResponse?.artifacts || [];
      const taskData = await loadTaskData();

      setOpportunities(opportunityResponse?.opportunities || []);
      setAllOpportunities(allOpportunityItems);
      setArtifacts(artifactResponse?.artifacts || []);
      setOpTasks(taskData.tasks);
      setTaskRunsByTaskId(taskData.runsByTaskId);
      setTodayStats(computeTodayStats(allOpportunityItems, allArtifacts, taskData.runs));
    } catch (err) {
      setError(err.message || "Failed to load Operator Console data.");
    } finally {
      setLoading(false);
    }
  }

  async function selectArtifact(artifact) {
    setSelectedArtifact(artifact);
    setArtifactContent(null);
    setArtifactDetailTab("text");
    setError("");

    try {
      const response = await apiFetch(
        apiBase,
        token,
        profileApiPath(selectedProfileId, `/op-task-artifacts/${artifact.id}/content`),
      );
      setArtifactContent(response);
    } catch (err) {
      setError(err.message || "Failed to load artifact content.");
    }
  }

  async function sendChatMessage() {
    const message = chatInput.trim();
    if (!message) return;

    setChatLoading(true);
    setError("");
    setNotice("");
    setChatInput("");
    setChatMessages((items) => [...items, { role: "user", content: message }]);

    try {
      const response = await apiFetch(apiBase, token, "/api/operator/chat", {
        method: "POST",
        body: JSON.stringify({
          message,
          include_home: true,
          profile_id: selectedProfileId,
        }),
      });

      setChatMessages((items) => [
        ...items,
        {
          role: "assistant",
          content: response?.message || "",
          mode: response?.mode,
          data: response?.data,
        },
      ]);

      const artifact = response?.data?.artifact;
      if (artifact?.id) {
        setSelectedArtifact(artifact);
        setArtifactDetailTab("text");
        setArtifactContent({
          artifact_id: artifact.id,
          name: artifact.name,
          artifact_type: artifact.artifact_type,
          content_text: artifact.content_text,
          content_json: artifact.content_json,
        });
      }

      await loadAll();
    } catch (err) {
      setError(err.message || "Failed to run chat request.");
      setChatMessages((items) => [
        ...items,
        { role: "assistant", content: err.message || "Failed to run chat request.", mode: "error" },
      ]);
    } finally {
      setChatLoading(false);
    }
  }

  async function runOpTask(taskId) {
    setActionLoading(`run-task-${taskId}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/op-tasks/${taskId}/run`), {
        method: "POST",
      });
      const run = response?.run;
      setTaskRunsByTaskId((current) => ({
        ...current,
        [taskId]: run ? [run, ...(current[taskId] || [])] : current[taskId] || [],
      }));
      setNotice(run?.summary || "Task run completed.");
      await loadAll();
    } catch (err) {
      setError(err.message || "Failed to run task.");
    } finally {
      setActionLoading("");
    }
  }

  async function readJobUrl({ createOpportunity = false } = {}) {
    const url = jobUrl.trim();
    if (!url) {
      setError("Enter a job URL first.");
      return;
    }

    const actionKey = createOpportunity ? "read-create-url" : "read-url";
    setActionLoading(actionKey);
    setError("");
    setNotice("");

    try {
      const createTaskResponse = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, "/op-tasks"), {
        method: "POST",
        body: JSON.stringify({
          name: "Read Job URL",
          task_type: "reader.read_url",
          description: "Read a job posting URL from the Operator Console.",
          input_json: { url },
          enabled: true,
        }),
      });

      const taskId = createTaskResponse?.task?.id;
      if (!taskId) {
        throw new Error("Reader task was created without an id.");
      }

      const runResponse = await apiFetch(
        apiBase,
        token,
        profileApiPath(selectedProfileId, `/op-tasks/${taskId}/run`),
        {
          method: "POST",
        },
      );

      const run = runResponse?.run;
      const artifact = run?.artifacts?.[0];
      if (!artifact?.id) {
        throw new Error(run?.summary || "Reader task completed without an artifact.");
      }

      setSelectedArtifact(artifact);
      setArtifactDetailTab("text");
      setArtifactContent({
        artifact_id: artifact.id,
        name: artifact.name,
        artifact_type: artifact.artifact_type,
        content_text: artifact.content_text,
        content_json: artifact.content_json,
      });

      if (createOpportunity) {
        const duplicateOpportunity = findDuplicateOpportunityForArtifact(artifact, allOpportunities);

        if (duplicateOpportunity) {
          if (!confirmDuplicateAction(duplicateOpportunity, "Read → Create Opportunity")) {
            setNotice("Read job URL and saved a readable artifact. Existing opportunity left unchanged.");
            await loadAll();
            return;
          }

          mergeOpportunity(duplicateOpportunity);
          setNotice("Read job URL and opened the existing matching opportunity.");
        } else {
          const opportunityResponse = await apiFetch(
            apiBase,
            token,
            profileApiPath(selectedProfileId, `/opportunities/from-artifact/${artifact.id}`),
            { method: "POST" },
          );
          mergeOpportunity(opportunityResponse.opportunity);
          setNotice("Read job URL and created an employment opportunity.");
        }

        setOpportunityDetailTab("summary");
      } else {
        setNotice("Read job URL and saved a readable artifact.");
      }

      await loadAll();
    } catch (err) {
      setError(err.message || "Failed to read job URL.");
    } finally {
      setActionLoading("");
    }
  }

  function confirmDuplicateAction(duplicateOpportunity, actionLabel) {
    if (!duplicateOpportunity) return true;

    return window.confirm(
      `An opportunity already exists for this artifact or source URL:\n\n${describeOpportunity(
        duplicateOpportunity,
      )}\n\n${actionLabel} will use the existing opportunity instead of creating a duplicate.`,
    );
  }

  async function createOpportunityFromArtifact(artifactId, duplicateOpportunity = null) {
    setActionLoading(`create-${artifactId}`);
    setError("");
    setNotice("");

    try {
      if (duplicateOpportunity) {
        mergeOpportunity(duplicateOpportunity);
        setOpportunityDetailTab("summary");
        setNotice("Opened existing opportunity instead of creating a duplicate.");
        return;
      }

      const response = await apiFetch(
        apiBase,
        token,
        profileApiPath(selectedProfileId, `/opportunities/from-artifact/${artifactId}`),
        { method: "POST" },
      );

      mergeOpportunity(response.opportunity);
      setOpportunityDetailTab("summary");
      setNotice("Created employment opportunity from artifact.");
      await loadAll();
    } catch (err) {
      setError(err.message || "Failed to create opportunity from artifact.");
    } finally {
      setActionLoading("");
    }
  }

  async function runArtifactOpportunityWorkflow(
    artifactId,
    { parse = false, score = false, duplicateOpportunity = null } = {},
  ) {
    const actionKey = score ? `create-parse-score-${artifactId}` : `create-parse-${artifactId}`;
    setActionLoading(actionKey);
    setError("");
    setNotice("");

    try {
      let response = null;
      let opportunity = duplicateOpportunity;

      if (!opportunity) {
        response = await apiFetch(
          apiBase,
          token,
          profileApiPath(selectedProfileId, `/opportunities/from-artifact/${artifactId}`),
          { method: "POST" },
        );

        opportunity = response.opportunity;
      }

      if (parse) {
        response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${opportunity.id}/parse`), {
          method: "POST",
        });
        opportunity = response.opportunity;
      }

      if (score) {
        response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${opportunity.id}/score`), {
          method: "POST",
        });
        opportunity = response.opportunity;
      }

      mergeOpportunity(opportunity);
      setOpportunityDetailTab(score ? "summary" : "parsed");
      if (duplicateOpportunity) {
        setNotice(score ? "Parsed and scored existing opportunity." : "Parsed existing opportunity.");
      } else {
        setNotice(score ? "Created, parsed, and scored opportunity." : "Created and parsed opportunity.");
      }
    } catch (err) {
      setError(err.message || "Failed to run artifact workflow.");
    } finally {
      setActionLoading("");
    }
  }

  async function parseOpportunity(id) {
    setActionLoading(`parse-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${id}/parse`), {
        method: "POST",
      });

      mergeOpportunity(response.opportunity);
      setOpportunityDetailTab("parsed");
      setNotice("Parsed opportunity with LLM.");
    } catch (err) {
      setError(err.message || "Failed to parse opportunity.");
    } finally {
      setActionLoading("");
    }
  }

  async function scoreOpportunity(id) {
    setActionLoading(`score-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${id}/score`), {
        method: "POST",
      });

      mergeOpportunity(response.opportunity);
      setOpportunityDetailTab("summary");
      setNotice("Scored opportunity.");
    } catch (err) {
      setError(err.message || "Failed to score opportunity.");
    } finally {
      setActionLoading("");
    }
  }

  async function generateCoverLetter(id) {
    setActionLoading(`cover-letter-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(
        apiBase,
        token,
        profileApiPath(selectedProfileId, `/opportunities/${id}/cover-letter`),
        {
          method: "POST",
          body: JSON.stringify({ direction: coverLetterDirection }),
        },
      );

      setCoverLetterDraft(response?.cover_letter || "");
      setOpportunityDetailTab("coverLetter");
      setNotice("Generated cover letter draft.");
    } catch (err) {
      setError(err.message || "Failed to generate cover letter.");
    } finally {
      setActionLoading("");
    }
  }

  async function archiveOpportunity(id, reason = "") {
    setActionLoading(`archive-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${id}/archive`), {
        method: "POST",
        body: JSON.stringify({ reason: reason || null }),
      });

      mergeOpportunity(response.opportunity);
      setNotice("Opportunity archived.");
    } catch (err) {
      setError(err.message || "Failed to archive opportunity.");
    } finally {
      setActionLoading("");
    }
  }

  function promptStatusReason(actionLabel) {
    const value = window.prompt(
      `${actionLabel} reason:\n\n${STATUS_REASON_PRESETS.map((reason, index) => `${index + 1}. ${reason}`).join(
        "\n",
      )}\n\nEnter a number or custom reason.`,
      STATUS_REASON_PRESETS[0],
    );
    if (value === null) return null;

    const presetIndex = Number(value.trim());
    if (Number.isInteger(presetIndex) && presetIndex >= 1 && presetIndex <= STATUS_REASON_PRESETS.length) {
      return STATUS_REASON_PRESETS[presetIndex - 1];
    }

    return value.trim();
  }

  async function rejectOpportunity(id, reason = "") {
    setActionLoading(`reject-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${id}/reject`), {
        method: "POST",
        body: JSON.stringify({ reason: reason || null }),
      });

      mergeOpportunity(response.opportunity);
      setNotice("Opportunity marked as rejected.");
    } catch (err) {
      setError(err.message || "Failed to reject opportunity.");
    } finally {
      setActionLoading("");
    }
  }

  async function restoreOpportunity(id, reason = "") {
    setActionLoading(`restore-${id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(apiBase, token, profileApiPath(selectedProfileId, `/opportunities/${id}/restore`), {
        method: "POST",
        body: JSON.stringify({ reason: reason || null }),
      });

      mergeOpportunity(response.opportunity);
      setNotice("Opportunity restored to review queue.");
    } catch (err) {
      setError(err.message || "Failed to restore opportunity.");
    } finally {
      setActionLoading("");
    }
  }

  function openOpportunitySourceUrl(opportunity) {
    if (!opportunity?.source_url) {
      setError("Opportunity has no source URL.");
      return;
    }

    window.open(opportunity.source_url, "_blank", "noopener,noreferrer");
  }

  async function viewOpportunitySourceArtifact(opportunity) {
    if (!opportunity?.source_artifact_id) {
      setError("Opportunity has no source artifact.");
      return;
    }

    setActionLoading(`view-artifact-${opportunity.id}`);
    setError("");
    setNotice("");

    try {
      const response = await apiFetch(
        apiBase,
        token,
        profileApiPath(selectedProfileId, `/op-task-artifacts/${opportunity.source_artifact_id}`),
      );
      await selectArtifact(response.artifact);
      setNotice("Loaded source artifact.");
    } catch (err) {
      setError(err.message || "Failed to load source artifact.");
    } finally {
      setActionLoading("");
    }
  }

  useEffect(() => {
    loadProfiles();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    setSelectedOpportunity(null);
    setSelectedArtifact(null);
    setArtifactContent(null);
    loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedProfileId]);

  useEffect(() => {
    setCriteriaDraft(selectedProfile?.criteria || "");
  }, [selectedProfile?.id, selectedProfile?.criteria]);

  useEffect(() => {
    setCoverLetterDraft("");
    setCoverLetterDirection("");
  }, [selectedOpportunity?.id]);

  return (
    <div style={styles.page}>
      <div style={styles.shell}>
        <header style={styles.header}>
          <div>
            <div style={styles.eyebrow}>Local Operator Console</div>
            <h1 style={styles.title}>Local Operator</h1>
            <p style={styles.subtitle}>
              Profile-scoped chat, task runs, artifacts, and employment review.
            </p>
          </div>
          <div style={styles.controls}>
            <div style={styles.row}>
              <input style={styles.input} value={apiBase} onChange={(e) => setApiBase(e.target.value)} />
              <button style={styles.button} onClick={loadAll} disabled={loading}>
                {loading ? "Loading…" : "Refresh"}
              </button>
            </div>
            <input
              style={styles.input}
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Bearer token, only if auth is enabled"
              type="password"
            />
          </div>
        </header>

        <section style={styles.profilePanel}>
          <div>
            <div style={styles.fieldLabel}>Active Profile</div>
            <div style={styles.cardTitle}>{selectedProfile?.display_name || "Default"}</div>
            <div style={styles.muted}>{selectedProfileId}</div>
          </div>
          <select
            style={styles.input}
            value={selectedProfileId}
            onChange={(e) => setSelectedProfileId(e.target.value)}
            title="Profile"
          >
            {profiles.length === 0 ? (
              <option value={selectedProfileId}>{selectedProfile?.display_name || "Default"}</option>
            ) : (
              profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.display_name}
                </option>
              ))
            )}
          </select>
          <button
            style={styles.buttonSecondary}
            onClick={createProfile}
            disabled={actionLoading === "create-profile"}
          >
            New Profile
          </button>
        </section>

        <section style={styles.mainTabs}>
          {MAIN_TABS.map((tab) => (
            <TabButton key={tab.value} active={activeMainTab === tab.value} onClick={() => setActiveMainTab(tab.value)}>
              {tab.label}
            </TabButton>
          ))}
        </section>

        {activeMainTab === "operator" ? <section style={styles.chatPanel}>
          {chatMessages.length ? (
            <div style={styles.chatMessages}>
              {chatMessages.map((message, index) => (
                <div
                  key={`${message.role}-${index}`}
                  style={{
                    ...styles.chatMessage,
                    ...(message.role === "user" ? styles.chatMessageUser : styles.chatMessageAssistant),
                  }}
                >
                  <strong>{message.role === "user" ? "You" : "Local Operator"}:</strong> {message.content}
                  {message.mode ? <div style={styles.muted}>{message.mode}</div> : null}
                </div>
              ))}
            </div>
          ) : null}
          <input
            style={styles.input}
            value={chatInput}
            onChange={(e) => setChatInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") sendChatMessage();
            }}
            placeholder="Ask Local Operator or read a URL..."
          />
          <button style={styles.button} onClick={sendChatMessage} disabled={chatLoading}>
            {chatLoading ? "Working…" : "Send"}
          </button>
        </section> : null}

        {activeMainTab === "operator" ? <section style={styles.jobUrlPanel}>
          <label style={styles.label} htmlFor="job-url-input">
            Job URL
          </label>
          <input
            id="job-url-input"
            style={styles.input}
            value={jobUrl}
            onChange={(e) => setJobUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") readJobUrl();
            }}
            placeholder="https://company.com/job/..."
          />
          <button
            style={styles.buttonSecondary}
            onClick={() => readJobUrl()}
            disabled={actionLoading === "read-url" || actionLoading === "read-create-url"}
          >
            {actionLoading === "read-url" ? "Reading…" : "Read URL"}
          </button>
          <button
            style={styles.button}
            onClick={() => readJobUrl({ createOpportunity: true })}
            disabled={actionLoading === "read-url" || actionLoading === "read-create-url"}
          >
            {actionLoading === "read-create-url" ? "Reading…" : "Read → Create Opportunity"}
          </button>
        </section> : null}

        {activeMainTab === "opportunities" ? <section style={styles.criteriaPanel}>
          <div>
            <div style={styles.sectionTitle}>
              <h2 style={styles.h2}>Profile Criteria</h2>
            </div>
            <div style={styles.muted}>{selectedProfile?.display_name || "Default"}</div>
          </div>
          <textarea
            style={styles.textarea}
            value={criteriaDraft}
            onChange={(e) => setCriteriaDraft(e.target.value)}
            placeholder="Target roles, must-haves, dealbreakers, location, salary, work style, technologies..."
          />
          <button
            style={styles.button}
            onClick={saveProfileCriteria}
            disabled={!criteriaChanged || actionLoading === "save-criteria"}
          >
            {actionLoading === "save-criteria" ? "Saving…" : "Save Criteria"}
          </button>
        </section> : null}

        <section>
          <div style={styles.sectionTitle}>
            <h2 style={styles.h2}>Today’s Work</h2>
            <span style={styles.muted}>Daily review</span>
          </div>
          <div style={styles.dashboardGrid}>
            <Metric label="New Artifacts Today" value={todayStats.artifacts} />
            <Metric label="New Opportunities Today" value={todayStats.opportunities} />
            <Metric label="Needs Parse" value={todayStats.needsParse} />
            <Metric label="Needs Score" value={todayStats.needsScore} />
            <Metric label="High Fit" value={todayStats.highFit} />
            <Metric label="Failed Runs" value={todayStats.failedRuns} />
          </div>
        </section>

        <section style={styles.grid4}>
          <Metric label="Opportunities" value={opportunities.length} />
          <Metric label="Artifacts" value={artifacts.length} />
          <Metric label="Selected Opportunity" value={selectedOpportunity?.title || "—"} compact />
          <Metric label="Selected Artifact" value={selectedArtifact?.name || "—"} compact />
        </section>

        {error ? <div style={{ ...styles.message, ...styles.error }}>{error}</div> : null}
        {notice ? <div style={{ ...styles.message, ...styles.notice }}>{notice}</div> : null}

        {["opportunities", "artifacts"].includes(activeMainTab) ? <section style={styles.filterBar}>
          <input
            style={styles.input}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search opportunities and artifacts"
          />
          <PresetGroup
            label="Status"
            value={statusFilter}
            options={STATUS_PRESETS}
            onChange={setStatusFilter}
          />
          <PresetGroup
            label="Artifact Type"
            value={artifactTypeFilter}
            options={ARTIFACT_TYPE_PRESETS}
            onChange={setArtifactTypeFilter}
          />
          <PresetGroup
            label="Fit"
            value={fitFilter}
            options={FIT_FILTER_PRESETS}
            onChange={setFitFilter}
          />
        </section> : null}

        {activeMainTab === "tasks" ? (
          <section style={styles.taskGrid}>
            <div>
              <div style={styles.sectionTitle}>
                <h2 style={styles.h2}>Tasks</h2>
                <span style={styles.muted}>{opTasks.length} configured</span>
              </div>
              <div style={styles.list}>
                {opTasks.map((task) => (
                  <TaskCard
                    key={task.id}
                    task={task}
                    runs={taskRunsByTaskId[task.id] || []}
                    onRun={() => runOpTask(task.id)}
                    actionLoading={actionLoading}
                  />
                ))}
                {!opTasks.length ? <div style={styles.empty}>No tasks found.</div> : null}
              </div>
            </div>

            <section style={styles.card}>
              <h2 style={styles.h2}>Task Runs</h2>
              <div style={styles.taskRunList}>
                {taskRuns.slice(0, 20).map((run) => (
                  <RunSummary key={run.id} run={run} />
                ))}
                {!taskRuns.length ? <p style={styles.muted}>No task runs yet.</p> : null}
              </div>
            </section>
          </section>
        ) : null}

        {activeMainTab === "opportunities" ? (
          <>
            <section>
              <div style={styles.sectionTitle}>
                <h2 style={styles.h2}>Employment Opportunities</h2>
                <span style={styles.muted}>{filteredOpportunities.length} shown</span>
              </div>
              <div style={styles.list}>
                {filteredOpportunities.map((item) => (
                  <OpportunityCard
                    key={item.id}
                    item={item}
                    selected={selectedOpportunity?.id === item.id}
                    onSelect={() => {
                      setSelectedOpportunity(item);
                      setOpportunityDetailTab("summary");
                    }}
                    onParse={() => parseOpportunity(item.id)}
                    onScore={() => scoreOpportunity(item.id)}
                    onArchive={() => {
                      const reason = promptStatusReason("Archive");
                      if (reason !== null) archiveOpportunity(item.id, reason);
                    }}
                    onReject={() => {
                      const reason = promptStatusReason("Reject");
                      if (reason !== null) rejectOpportunity(item.id, reason);
                    }}
                    onRestore={() => restoreOpportunity(item.id)}
                    onOpenSource={() => openOpportunitySourceUrl(item)}
                    onViewArtifact={() => {
                      viewOpportunitySourceArtifact(item);
                      setActiveMainTab("artifacts");
                    }}
                    actionLoading={actionLoading}
                  />
                ))}
                {!filteredOpportunities.length ? <div style={styles.empty}>No opportunities found.</div> : null}
              </div>
            </section>

            <section style={styles.card}>
              <h2 style={styles.h2}>Opportunity Detail</h2>
              {selectedOpportunity ? (
                <OpportunityDetail
                  opportunity={selectedOpportunity}
                  activeTab={opportunityDetailTab}
                  onTabChange={setOpportunityDetailTab}
                  onParse={() => parseOpportunity(selectedOpportunity.id)}
                  onScore={() => scoreOpportunity(selectedOpportunity.id)}
                  coverLetterDirection={coverLetterDirection}
                  onCoverLetterDirectionChange={setCoverLetterDirection}
                  coverLetterDraft={coverLetterDraft}
                  onCoverLetterDraftChange={setCoverLetterDraft}
                  onGenerateCoverLetter={() => generateCoverLetter(selectedOpportunity.id)}
                  actionLoading={actionLoading}
                />
              ) : (
                <p style={styles.muted}>Select an opportunity to review parsed fields and scoring.</p>
              )}
            </section>
          </>
        ) : null}

        {activeMainTab === "artifacts" ? (
          <>
            <div style={styles.sectionTitle}>
              <h2 style={styles.h2}>Artifacts</h2>
              <span style={styles.muted}>{filteredArtifacts.length} shown</span>
            </div>
            <div style={styles.list}>
              {filteredArtifacts.map((item) => {
                const duplicateOpportunity = findDuplicateOpportunityForArtifact(item, allOpportunities);

                return (
                  <ArtifactCard
                    key={item.id}
                    item={item}
                    duplicateOpportunity={duplicateOpportunity}
                    selected={selectedArtifact?.id === item.id}
                    onSelect={() => selectArtifact(item)}
                    onCreate={() => {
                      if (!confirmDuplicateAction(duplicateOpportunity, "Create Opportunity")) return;
                      createOpportunityFromArtifact(item.id, duplicateOpportunity);
                    }}
                    onCreateParse={() => {
                      if (!confirmDuplicateAction(duplicateOpportunity, "Create + Parse")) return;
                      runArtifactOpportunityWorkflow(item.id, {
                        parse: true,
                        duplicateOpportunity,
                      });
                    }}
                    onCreateParseScore={() => {
                      if (!confirmDuplicateAction(duplicateOpportunity, "Create + Parse + Score")) return;
                      runArtifactOpportunityWorkflow(item.id, {
                        parse: true,
                        score: true,
                        duplicateOpportunity,
                      });
                    }}
                    actionLoading={actionLoading}
                  />
                );
              })}
              {!filteredArtifacts.length ? <div style={styles.empty}>No artifacts found.</div> : null}
            </div>

            <section style={styles.card}>
              <h2 style={styles.h2}>Artifact Content</h2>
              {selectedArtifact ? (
                <ArtifactDetail
                  artifact={selectedArtifact}
                  artifactContent={artifactContent}
                  activeTab={artifactDetailTab}
                  onTabChange={setArtifactDetailTab}
                />
              ) : (
                <p style={styles.muted}>Select an artifact to review readable content.</p>
              )}
            </section>
          </>
        ) : null}
      </div>
    </div>
  );
}

function Metric({ label, value, compact = false }) {
  return (
    <div style={styles.metricCard}>
      <div style={styles.metricLabel}>{label}</div>
      <div style={{ ...styles.metricValue, fontSize: compact ? 16 : 24, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {value}
      </div>
    </div>
  );
}

function Detail({ label, value }) {
  return (
    <div style={{ marginBottom: 8, ...styles.text }}>
      <strong>{label}:</strong> {value || "—"}
    </div>
  );
}

function PresetGroup({ label, value, options, onChange }) {
  return (
    <div style={styles.presetGroup}>
      <div style={styles.fieldLabel}>{label}</div>
      <div style={styles.presetButtons}>
        {options.map((option) => (
          <button
            key={option.label}
            type="button"
            style={{
              ...styles.presetButton,
              ...(value === option.value ? styles.activePresetButton : {}),
            }}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

function TabButton({ active, children, onClick }) {
  return (
    <button type="button" style={{ ...styles.tab, ...(active ? styles.activeTab : {}) }} onClick={onClick}>
      {children}
    </button>
  );
}

function Field({ label, value }) {
  return (
    <div style={styles.field}>
      <div style={styles.fieldLabel}>{label}</div>
      <div style={styles.fieldValue}>{value || "—"}</div>
    </div>
  );
}

function JsonDebug({ value, summary = "Show JSON" }) {
  return (
    <details style={styles.debugDetails}>
      <summary style={styles.debugSummary}>{summary}</summary>
      <pre style={styles.pre}>{stringifyJson(value)}</pre>
    </details>
  );
}

function RiskFlags({ flags }) {
  if (!flags?.length) return null;

  return (
    <div style={styles.riskFlags}>
      {flags.map((flag) => (
        <span key={flag} style={{ ...styles.badge, ...styles.badgeBad }}>
          {riskFlagLabel(flag)}
        </span>
      ))}
    </div>
  );
}

function OpportunityDetail({
  opportunity,
  activeTab,
  onTabChange,
  onParse,
  onScore,
  coverLetterDirection,
  onCoverLetterDirectionChange,
  coverLetterDraft,
  onCoverLetterDraftChange,
  onGenerateCoverLetter,
  actionLoading,
}) {
  return (
    <div>
      <div style={styles.tabs}>
        <TabButton active={activeTab === "summary"} onClick={() => onTabChange("summary")}>
          Summary
        </TabButton>
        <TabButton active={activeTab === "parsed"} onClick={() => onTabChange("parsed")}>
          Parsed Fields
        </TabButton>
        <TabButton active={activeTab === "raw"} onClick={() => onTabChange("raw")}>
          Raw JSON
        </TabButton>
        <TabButton active={activeTab === "source"} onClick={() => onTabChange("source")}>
          Source Artifact
        </TabButton>
        <TabButton active={activeTab === "coverLetter"} onClick={() => onTabChange("coverLetter")}>
          Cover Letter
        </TabButton>
      </div>

      <div style={styles.actions}>
        <button
          style={styles.buttonSecondary}
          onClick={onParse}
          disabled={actionLoading === `parse-${opportunity.id}`}
        >
          {actionLoading === `parse-${opportunity.id}` ? "Parsing…" : "Parse"}
        </button>
        <button
          style={styles.buttonSecondary}
          onClick={onScore}
          disabled={actionLoading === `score-${opportunity.id}`}
        >
          {actionLoading === `score-${opportunity.id}` ? "Scoring…" : "Score"}
        </button>
        <button style={styles.button} onClick={() => onTabChange("coverLetter")}>
          Cover Letter
        </button>
      </div>

      {activeTab === "summary" ? (
        <>
          <div style={styles.fieldGrid}>
            <Field label="Title" value={opportunity.title} />
            <Field label="Company" value={opportunity.company} />
            <Field label="Location" value={opportunity.location} />
            <Field label="Remote Type" value={opportunity.remote_type} />
            <Field label="Salary" value={formatSalary(opportunity)} />
            <Field label="Primary Fit" value={formatScore(opportunity.primary_fit_score ?? opportunity.fit_score)} />
            <Field label="OE Fit" value={formatScore(opportunity.oe_fit_score)} />
            <Field label="Recommended Track" value={formatTrack(opportunity.recommended_track)} />
            <Field label="Status" value={opportunity.status} />
            <Field label="Source URL" value={opportunity.source_url} />
          </div>
          <RiskFlags flags={opportunity.risk_flags || []} />
          {opportunity.score_reason ? <p style={styles.text}>{opportunity.score_reason}</p> : null}
          {opportunity.skip_recommendation ? (
            <div style={{ ...styles.message, ...styles.error }}>{opportunity.skip_recommendation}</div>
          ) : null}
        </>
      ) : null}

      {activeTab === "parsed" ? (
        <div>
          <p style={styles.text}>{opportunity.description_text || "No parsed description yet."}</p>
          <JsonDebug value={opportunity.extracted_json || {}} summary="Show parsed JSON" />
        </div>
      ) : null}

      {activeTab === "raw" ? <JsonDebug value={opportunity} summary="Show full opportunity JSON" /> : null}

      {activeTab === "source" ? (
        <div style={styles.fieldGrid}>
          <Field label="Source Artifact ID" value={opportunity.source_artifact_id} />
          <Field label="Source URL" value={opportunity.source_url} />
          <Field label="First Seen" value={safeDate(opportunity.first_seen_at)} />
          <Field label="Last Seen" value={safeDate(opportunity.last_seen_at)} />
        </div>
      ) : null}

      {activeTab === "coverLetter" ? (
        <div style={styles.list}>
          <div>
            <div style={styles.fieldLabel}>Direction</div>
            <textarea
              style={styles.textarea}
              value={coverLetterDirection}
              onChange={(e) => onCoverLetterDirectionChange(e.target.value)}
              placeholder="Tone, points to emphasize, constraints, recipient details, or anything to avoid..."
            />
          </div>
          <div style={styles.actions}>
            <button
              style={styles.button}
              onClick={onGenerateCoverLetter}
              disabled={actionLoading === `cover-letter-${opportunity.id}`}
            >
              {actionLoading === `cover-letter-${opportunity.id}` ? "Generating…" : "Generate Draft"}
            </button>
          </div>
          <div>
            <div style={styles.fieldLabel}>Draft</div>
            <textarea
              style={{ ...styles.textarea, minHeight: 320 }}
              value={coverLetterDraft}
              onChange={(e) => onCoverLetterDraftChange(e.target.value)}
              placeholder="Generated cover letter draft will appear here."
            />
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ArtifactDetail({ artifact, artifactContent, activeTab, onTabChange }) {
  const readableText = artifactContent?.content_text || artifact.content_text || "";
  const contentJson = artifactContent?.content_json || artifact.content_json || {};
  const metadata = artifact.metadata || contentJson?.metadata || {};

  return (
    <div>
      <div style={styles.tabs}>
        <TabButton active={activeTab === "text"} onClick={() => onTabChange("text")}>
          Readable Text
        </TabButton>
        <TabButton active={activeTab === "json"} onClick={() => onTabChange("json")}>
          Raw JSON
        </TabButton>
        <TabButton active={activeTab === "metadata"} onClick={() => onTabChange("metadata")}>
          Source Metadata
        </TabButton>
      </div>

      {activeTab === "text" ? (
        <pre style={styles.pre}>{readableText || "No content loaded."}</pre>
      ) : null}

      {activeTab === "json" ? <JsonDebug value={contentJson} summary="Show artifact content JSON" /> : null}

      {activeTab === "metadata" ? (
        <div>
          <div style={styles.fieldGrid}>
            <Field label="Name" value={artifact.name} />
            <Field label="Artifact Type" value={artifact.artifact_type} />
            <Field label="Location" value={artifact.location} />
            <Field label="Created" value={safeDate(artifact.created_at)} />
            <Field label="Artifact ID" value={artifact.id} />
            <Field label="Run ID" value={artifact.run_id} />
          </div>
          <JsonDebug value={metadata} summary="Show source metadata JSON" />
        </div>
      ) : null}
    </div>
  );
}

function TaskCard({ task, runs, onRun, actionLoading }) {
  const latestRun = runs?.[0];
  const priority = task.input_json?.priority || "normal";
  const modelPurpose = task.input_json?.model_purpose || task.input_json?.modelPurpose || "task";

  return (
    <div style={styles.card}>
      <div style={styles.cardHeader}>
        <div>
          <div style={styles.cardTitle}>{task.name}</div>
          <div style={styles.muted}>{task.task_type}</div>
        </div>
        <span style={statusStyle(task.status)}>{task.status}</span>
      </div>
      {task.description ? <p style={styles.text}>{task.description}</p> : null}
      <div style={styles.riskFlags}>
        <span style={styles.badge}>Priority {priority}</span>
        <span style={styles.badge}>Model {modelPurpose}</span>
        <span style={styles.badge}>Runs {runs?.length || 0}</span>
      </div>
      {latestRun ? <RunSummary run={latestRun} compact /> : null}
      <div style={styles.actions}>
        <button style={styles.button} onClick={onRun} disabled={actionLoading === `run-task-${task.id}`}>
          {actionLoading === `run-task-${task.id}` ? "Running…" : "Run"}
        </button>
      </div>
      <JsonDebug value={task.input_json || {}} summary="Show task input" />
    </div>
  );
}

function RunSummary({ run, compact = false }) {
  return (
    <div style={compact ? { marginTop: 10 } : styles.field}>
      <div style={styles.cardHeader}>
        <span style={statusStyle(run.status)}>{run.status}</span>
        <span style={styles.muted}>{safeDate(run.completed_at || run.started_at)}</span>
      </div>
      {run.summary ? <p style={styles.text}>{compact ? compactText(run.summary, 180) : run.summary}</p> : null}
      <div style={styles.muted}>
        {run.artifacts?.length || 0} artifacts · {run.work_items?.length || 0} work items
      </div>
    </div>
  );
}

function OpportunityCard({
  item,
  selected,
  onSelect,
  onParse,
  onScore,
  onArchive,
  onReject,
  onRestore,
  onOpenSource,
  onViewArtifact,
  actionLoading,
}) {
  const isArchived = item.status === "Archived";
  const isRejected = item.status === "Rejected";
  const isClosedOrFinal = ["Archived", "Rejected", "Closed"].includes(item.status);

  return (
    <div style={{ ...styles.card, ...(selected ? styles.selectedCard : {}) }} onClick={onSelect}>
      <div style={styles.cardHeader}>
        <div>
          <div style={styles.cardTitle}>{item.title || "Untitled opportunity"}</div>
          <div style={styles.muted}>{item.company || "Unknown company"} · {item.location || "Unknown location"}</div>
        </div>
        <span style={statusStyle(item.status)}>{item.status || "unknown"}</span>
      </div>
      <p style={styles.text}>{compactText(item.description_text)}</p>
      <div style={styles.riskFlags}>
        <span style={{ ...styles.badge, ...styles.badgeGood }}>
          Primary {formatScore(item.primary_fit_score ?? item.fit_score)}
        </span>
        <span style={{ ...styles.badge, ...styles.badgeInfo }}>OE {formatScore(item.oe_fit_score)}</span>
        <span style={styles.badge}>{formatTrack(item.recommended_track)}</span>
      </div>
      <RiskFlags flags={item.risk_flags || []} />
      <div style={styles.muted}>Remote: {item.remote_type || "—"} · Updated: {safeDate(item.last_seen_at)}</div>
      {item.score_reason ? <p style={styles.text}>{compactText(item.score_reason, 220)}</p> : null}
      <div style={styles.actions}>
        {!isClosedOrFinal && (
          <>
            <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onParse(); }}>
              {actionLoading === `parse-${item.id}` ? "Parsing…" : "Parse"}
            </button>
            <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onScore(); }}>
              {actionLoading === `score-${item.id}` ? "Scoring…" : "Score"}
            </button>
          </>
        )}
        {!isClosedOrFinal && (
          <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onArchive(); }}>
            {actionLoading === `archive-${item.id}` ? "Archiving…" : "Archive"}
          </button>
        )}
        {!isClosedOrFinal && (
          <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onReject(); }}>
            {actionLoading === `reject-${item.id}` ? "Rejecting…" : "Reject"}
          </button>
        )}
        {(isArchived || isRejected) && (
          <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onRestore(); }}>
            {actionLoading === `restore-${item.id}` ? "Restoring…" : "Restore"}
          </button>
        )}
        <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onOpenSource(); }}>
          Open Source URL
        </button>
        <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onViewArtifact(); }}>
          {actionLoading === `view-artifact-${item.id}` ? "Loading…" : "View Source Artifact"}
        </button>
      </div>
    </div>
  );
}

function ArtifactCard({
  item,
  duplicateOpportunity,
  selected,
  onSelect,
  onCreate,
  onCreateParse,
  onCreateParseScore,
  actionLoading,
}) {
  return (
    <div style={{ ...styles.card, ...(selected ? styles.selectedCard : {}) }} onClick={onSelect}>
      <div style={styles.cardHeader}>
        <div>
          <div style={styles.cardTitle}>{item.name || "Untitled artifact"}</div>
          <div style={styles.muted}>{item.location || "No location"}</div>
        </div>
        <span style={styles.badge}>{item.artifact_type}</span>
      </div>
      <div style={styles.muted}>Created: {safeDate(item.created_at)}</div>
      {duplicateOpportunity ? (
        <div style={styles.duplicateWarning}>
          Matching opportunity exists: {describeOpportunity(duplicateOpportunity)}
        </div>
      ) : null}
      <div style={styles.actions}>
        <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onSelect(); }}>
          Review
        </button>
        <button style={styles.button} onClick={(e) => { e.stopPropagation(); onCreate(); }}>
          {actionLoading === `create-${item.id}` ? "Creating…" : "Create Opportunity"}
        </button>
        <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onCreateParse(); }}>
          {actionLoading === `create-parse-${item.id}` ? "Working…" : "Create + Parse"}
        </button>
        <button style={styles.buttonSecondary} onClick={(e) => { e.stopPropagation(); onCreateParseScore(); }}>
          {actionLoading === `create-parse-score-${item.id}` ? "Working…" : "Create + Parse + Score"}
        </button>
      </div>
    </div>
  );
}
