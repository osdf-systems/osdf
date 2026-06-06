import init, {
  verify_osdf,
  verify_osdf_with_config,
  version,
  core_version,
  build_commit,
} from "./pkg/osdf_wasm.js";

const dropZone = document.querySelector("#drop-zone");
const chooseButton = document.querySelector("#choose-file");
const fileInput = document.querySelector("#file-input");
const fileSummary = document.querySelector("#file-summary");
const resultPanel = document.querySelector("#verification-result");
const fingerprint = document.querySelector("#verifier-fingerprint");
const ledgerConfigInput = document.querySelector("#ledger-config");
const identityConfigInput = document.querySelector("#identity-config");

let lastReport = null;
let lastFileName = null;

await init();

fingerprint.textContent = `Verifier: osdf-wasm ${version()} · Core: osdf-core ${core_version()} · Build: ${build_commit()}`;

chooseButton.addEventListener("click", () => fileInput.click());

fileInput.addEventListener("change", async () => {
  const [file] = fileInput.files;
  if (file) {
    await verifyFile(file);
  }
});

dropZone.addEventListener("dragover", (event) => {
  event.preventDefault();
  dropZone.classList.add("drag-over");
});

dropZone.addEventListener("dragleave", () => {
  dropZone.classList.remove("drag-over");
});

dropZone.addEventListener("drop", async (event) => {
  event.preventDefault();
  dropZone.classList.remove("drag-over");

  const [file] = event.dataTransfer.files;
  if (file) {
    await verifyFile(file);
  }
});

dropZone.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    fileInput.click();
  }
});

document.querySelector("#export-report")?.addEventListener("click", () => {
  if (!lastReport) {
    return;
  }
  const payload = {
    format: "OSDF",
    verifierVersion: version(),
    coreVersion: core_version(),
    buildCommit: build_commit(),
    verifiedAt: new Date().toISOString(),
    fileName: lastFileName,
    documentId: lastReport.documentId ?? null,
    revision: lastReport.revision ?? null,
    profile: lastReport.profile ?? null,
    verificationMode: lastReport.verificationMode ?? null,
    signerIdentities: lastReport.signerIdentities ?? [],
    overall: lastReport.overall,
    signatureCount: lastReport.signatureCount ?? 0,
    sections: lastReport.sections ?? [],
    findings: lastReport.findings ?? [],
    checks: lastReport.checks ?? [],
  };
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${(lastFileName ?? "report").replace(/\.osdf$/i, "")}-verification.json`;
  anchor.click();
  URL.revokeObjectURL(url);
});

async function verifyFile(file) {
  lastFileName = file.name;
  showFileSummary(file);
  showPending();

  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const config = buildVerifierConfig();
    const report = config ? verify_osdf_with_config(bytes, JSON.stringify(config)) : verify_osdf(bytes);
    lastReport = report;
    renderReport(report, file.name);
  } catch (error) {
    lastReport = null;
    renderFatalError(error);
  }
}

function showFileSummary(file) {
  fileSummary.hidden = false;
  fileSummary.innerHTML = `
    <h2>Selected file</h2>
    <dl>
      <dt>Name</dt>
      <dd>${escapeHtml(file.name)}</dd>
      <dt>Size</dt>
      <dd>${file.size.toLocaleString()} bytes</dd>
    </dl>
  `;
}

function showPending() {
  resultPanel.hidden = false;
  resultPanel.innerHTML = `
    <h2>Verification</h2>
    <p>Checking document structure and cryptographic integrity…</p>
  `;
}

function buildVerifierConfig() {
  const ledgerJson = ledgerConfigInput?.value?.trim();
  const identityJson = identityConfigInput?.value?.trim();
  if (!ledgerJson && !identityJson) {
    return null;
  }

  const config = {};
  if (ledgerJson) {
    config.ledger = JSON.parse(ledgerJson);
  }
  if (identityJson) {
    config.identity = JSON.parse(identityJson);
  }
  return config;
}

function renderReport(report, fileName) {
  const passed = report.overall === "PASS";
  const bannerClass = passed ? "banner-pass" : "banner-fail";
  const bannerTitle = passed ? "File integrity verified" : "Verification failed";
  const bannerBody = passed
    ? "This document passed structural, cryptographic-signature, and embedded-ledger-proof checks in this verifier."
    : "Do not trust this file.";

  const caveats = buildCaveats(report);
  const caveatHtml = caveats
    .map((line) => `<li><span class="info-icon">ⓘ</span> ${escapeHtml(line)}</li>`)
    .join("");

  const headlineChecks = buildHeadlineChecks(report);
  const headlineHtml = headlineChecks
    .map(
      (check) => `
      <li class="headline-check ${statusClass(check.status)}">
        <span class="check-icon">${icon(check.status)}</span>
        <span>${escapeHtml(check.label)}</span>
      </li>`
    )
    .join("");

  const sections = sortSections(report.sections ?? [])
    .map((section) => renderSection(section, section.section === "verificationContext"))
    .join("");

  const failFindings = (report.findings ?? []).filter((item) => item.severity === "FAIL");
  const warnFindings = (report.findings ?? []).filter((item) => item.severity === "WARNING");

  const userFindings = [...failFindings, ...warnFindings]
    .map(
      (finding) => `
      <article class="finding ${finding.severity.toLowerCase()}">
        <h4>${icon(mapFindingStatus(finding.severity))} ${escapeHtml(finding.summary)}</h4>
        <p>${escapeHtml(finding.impact)}</p>
        <details class="technical-details">
          <summary>Show technical details</summary>
          <code>${escapeHtml(finding.code)}</code>
          <pre>${escapeHtml(finding.technical)}</pre>
        </details>
      </article>`
    )
    .join("");

  const technicalDetails = buildTechnicalDetails(report, fileName);

  resultPanel.innerHTML = `
    <div class="trust-banner ${bannerClass}">
      <strong>${icon(passed ? "PASS" : "FAIL")} ${escapeHtml(bannerTitle)}</strong>
      <p>${escapeHtml(bannerBody)}</p>
      ${passed && caveats.length > 0 ? `<ul class="banner-caveats">${caveatHtml}</ul>` : ""}
    </div>

    <section class="headline-summary">
      <h2>At a glance</h2>
      <ul class="headline-checks">${headlineHtml}</ul>
    </section>

    <details class="section-details" open>
      <summary>Verification details</summary>
      ${sections}
    </details>

    ${userFindings ? `<section class="findings">${userFindings}</section>` : ""}

    <details class="technical-panel">
      <summary>Technical details</summary>
      ${technicalDetails}
    </details>

    <div class="actions">
      <button id="export-report-inline" type="button">Export verification report</button>
    </div>
  `;

  document.querySelector("#export-report-inline")?.addEventListener("click", () => {
    document.querySelector("#export-report")?.click();
  });
}

function buildCaveats(report) {
  const caveats = [];
  if (
    !report.signerIdentities?.length &&
    (hasCheck(report, "OSDF_IDENTITY_NOT_RESOLVED", "INFO") ||
      hasCheck(report, "OSDF_VERIFICATION_IDENTITY_UNRESOLVED", "INFO"))
  ) {
    caveats.push("Signer identity has not been resolved.");
  }
  if (
    hasCheck(report, "OSDF_LIVE_LATEST_REVISION_NOT_CHECKED", "INFO") ||
    hasCheck(report, "OSDF_LEDGER_LATEST_REVISION_NOT_CHECKED", "INFO")
  ) {
    caveats.push("A live latest-revision check has not been performed.");
  }
  if (
    hasCheck(report, "OSDF_LIVE_REVOCATION_NOT_CHECKED", "INFO") ||
    hasCheck(report, "OSDF_REVOCATION_NOT_CONFIGURED", "INFO")
  ) {
    caveats.push("Revocation status has not been checked.");
  }
  if ((report.verificationMode ?? "offlineCryptographic") === "offlineCryptographic") {
    caveats.push("Verification ran in offline cryptographic mode.");
  }
  return caveats;
}

function buildHeadlineChecks(report) {
  const items = [];
  const push = (code, label, preferredStatus = "PASS") => {
    const check = findCheck(report, code);
    if (check) {
      items.push({ label: check.label, status: check.status });
      return;
    }
    if (preferredStatus) {
      items.push({ label, status: preferredStatus });
    }
  };

  push("OSDF_SIGNATURE_CRYPTO", "Signature cryptographically valid");
  push("OSDF_LEDGER_INCLUSION_PROOF_VALID", "Transparency proof valid");
  if (report.signerIdentities?.length > 0) {
    const identity = report.signerIdentities[0];
    const label = identity.department
      ? `Signer identity resolved · ${identity.displayName} · ${identity.department}`
      : `Signer identity resolved · ${identity.displayName}`;
    items.push({ label, status: "PASS" });
  } else {
    push("OSDF_IDENTITY_NOT_RESOLVED", "Signer identity not yet resolved", "INFO");
  }
  push("OSDF_REVOCATION_NOT_CONFIGURED", "Revocation check not configured", "INFO");

  if (items.length === 0) {
    return [{ label: report.overall === "PASS" ? "Checks completed" : "Checks failed", status: report.overall }];
  }
  return items;
}

function renderSection(section, expandedByDefault) {
  const checks = section.checks
    .map(
      (check) => `
      <li class="check ${statusClass(check.status)}">
        <span class="check-icon">${icon(check.status)}</span>
        <span class="check-body">
          <strong>${escapeHtml(check.label)}</strong>
          ${check.details ? `<span class="check-details">${escapeHtml(check.details)}</span>` : ""}
        </span>
      </li>`
    )
    .join("");

  if (section.section === "verificationContext") {
    return `
      <section class="verify-section verify-section-mode">
        <h3>${escapeHtml(section.title)}</h3>
        <ul class="checks">${checks}</ul>
      </section>`;
  }

  const openAttr = expandedByDefault ? " open" : "";
  return `
    <details class="verify-section"${openAttr}>
      <summary>${escapeHtml(section.title)}</summary>
      <ul class="checks">${checks}</ul>
    </details>`;
}

function buildTechnicalDetails(report, fileName) {
  const ledgerProof = findCheck(report, "OSDF_LEDGER_PROOF_PRESENT");
  const ledgerKey = findCheck(report, "OSDF_LEDGER_LOG_KEY_TRUSTED");
  const freshness = findCheck(report, "OSDF_LEDGER_TREE_HEAD_FRESHNESS_NOT_CHECKED");

  return `
    <dl class="technical-grid">
      <dt>File</dt>
      <dd>${escapeHtml(fileName)}</dd>
      <dt>Document ID</dt>
      <dd><code>${escapeHtml(report.documentId ?? "Unavailable")}</code></dd>
      <dt>Revision</dt>
      <dd>${escapeHtml(String(report.revision ?? "Unavailable"))}</dd>
      <dt>Profile</dt>
      <dd>${escapeHtml(report.profile ?? "Unavailable")}</dd>
      <dt>Verification mode</dt>
      <dd>${escapeHtml(formatVerificationMode(report.verificationMode))}</dd>
      <dt>Ledger entry</dt>
      <dd><code>${escapeHtml(ledgerProof?.details ?? "Not present")}</code></dd>
      <dt>Trusted ledger key</dt>
      <dd><code>${escapeHtml(ledgerKey?.details ?? "Not evaluated")}</code></dd>
      <dt>Tree-head timestamp</dt>
      <dd>${escapeHtml(extractTimestamp(freshness?.details) ?? "Not evaluated")}</dd>
      <dt>Resolved signer</dt>
      <dd>${escapeHtml(formatResolvedSigner(report))}</dd>
      <dt>Overall status</dt>
      <dd>${escapeHtml(report.overall)}</dd>
    </dl>
  `;
}

function sortSections(sections) {
  const order = {
    verificationContext: 0,
    container: 1,
    manifest: 2,
    revision: 3,
    signatures: 4,
    transparency: 5,
  };
  return [...sections].sort(
    (left, right) => (order[left.section] ?? 99) - (order[right.section] ?? 99)
  );
}

function findCheck(report, code) {
  return (report.checks ?? []).find((check) => check.code === code);
}

function hasCheck(report, code, status) {
  const check = findCheck(report, code);
  return Boolean(check && check.status === status);
}

function formatResolvedSigner(report) {
  const identity = report.signerIdentities?.[0];
  if (!identity) {
    return "Not resolved";
  }
  if (identity.department) {
    return `${identity.displayName} · ${identity.department}`;
  }
  return identity.displayName;
}

function formatVerificationMode(mode) {
  if (mode === "onlineEnhanced") {
    return "Online enhanced verification";
  }
  return "Offline cryptographic verification";
}

function extractTimestamp(details) {
  if (!details) {
    return null;
  }
  const match = details.match(/embedded checkpoint timestamp:\s*(.+)$/i);
  return match ? match[1] : details;
}

function signatureSummary(report) {
  if (report.revision === 0) {
    return "Not required for revision 0";
  }
  const crypto = findCheck(report, "OSDF_SIGNATURE_CRYPTO");
  const identity = findCheck(report, "OSDF_IDENTITY_NOT_RESOLVED");
  const count = report.signatureCount ?? 0;
  if (crypto?.status === "PASS") {
    const identityNote =
      identity?.status === "INFO" ? " · signer identity not yet resolved" : "";
    return `${count} cryptographically valid signature${count === 1 ? "" : "s"}${identityNote}`;
  }
  if (count > 0) {
    return `${count} signature file(s) present`;
  }
  return "No valid signature";
}

function renderFatalError(error) {
  resultPanel.hidden = false;
  resultPanel.innerHTML = `
    <div class="trust-banner banner-fail">
      <strong>✗ Verification failed</strong>
      <p>The selected file could not be verified.</p>
    </div>
    <pre class="fatal">${escapeHtml(String(error))}</pre>
  `;
}

function statusClass(status) {
  switch (status) {
    case "PASS":
      return "pass";
    case "WARNING":
      return "warning";
    case "INFO":
      return "info";
    default:
      return "fail";
  }
}

function mapFindingStatus(severity) {
  switch (severity) {
    case "WARNING":
      return "WARNING";
    case "INFO":
      return "INFO";
    default:
      return "FAIL";
  }
}

function icon(status) {
  switch (status) {
    case "PASS":
      return "✓";
    case "WARNING":
      return "⚠";
    case "INFO":
      return "ⓘ";
    default:
      return "✗";
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
