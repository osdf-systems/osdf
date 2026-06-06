const MFA_KEY = "osdf-gateway-mfa";
const DEMO_CODE = "847291";

const mfaGate = document.querySelector("#mfa-gate");
const app = document.querySelector("#app");
const mfaInput = document.querySelector("#mfa-input");
const mfaError = document.querySelector("#mfa-error");
const mfaBootError = document.querySelector("#mfa-boot-error");
const bootErrors = () => document.querySelectorAll(".boot-error");
const verificationBanner = document.querySelector("#verification-banner");
const documentFrame = document.querySelector("#document-frame");
const ledgerConfigInput = document.querySelector("#ledger-config");
const identityConfigInput = document.querySelector("#identity-config");
const fileInput = document.querySelector("#file-input");

let wasmModule = null;
let wasmInitPromise = null;

setupMfa();
setupDocumentControls();

function setupMfa() {
  if (sessionStorage.getItem(MFA_KEY) === "ok") {
    unlock();
    return;
  }

  document.querySelector("#mfa-submit").addEventListener("click", tryMfa);
  mfaInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      tryMfa();
    }
  });
}

function setupDocumentControls() {
  document.querySelectorAll("[data-fixture]").forEach((button) => {
    button.addEventListener("click", async () => {
      const url = button.getAttribute("data-fixture");
      try {
        const response = await fetch(url);
        if (!response.ok) {
          showBootError(`Could not load ${url} (${response.status}). Serve from repo root via serve-demo.ps1.`);
          return;
        }
        const bytes = new Uint8Array(await response.arrayBuffer());
        await openPackage(bytes, url.split("/").pop());
      } catch (error) {
        showBootError(`Failed to load fixture: ${error}`);
      }
    });
  });

  document.querySelector("#choose-file").addEventListener("click", () => fileInput.click());
  fileInput.addEventListener("change", async () => {
    const [file] = fileInput.files;
    if (file) {
      await openPackage(new Uint8Array(await file.arrayBuffer()), file.name);
    }
  });
}

function tryMfa() {
  const code = mfaInput.value.replace(/\s/g, "");
  if (code === DEMO_CODE) {
    sessionStorage.setItem(MFA_KEY, "ok");
    mfaError.hidden = true;
    unlock();
    return;
  }
  mfaError.hidden = false;
  mfaInput.focus();
  mfaInput.select();
}

function unlock() {
  mfaGate.hidden = true;
  mfaGate.setAttribute("aria-hidden", "true");
  app.hidden = false;
  app.removeAttribute("aria-hidden");
  clearBootError();
}

function showBootError(message) {
  console.error(message);

  if (mfaGate && !mfaGate.hidden && mfaBootError) {
    mfaBootError.hidden = false;
    mfaBootError.textContent = message;
    return;
  }

  const targets = bootErrors();
  if (targets.length) {
    for (const element of targets) {
      element.hidden = false;
      element.textContent = message;
    }
    return;
  }

  alert(message);
}

function clearBootError() {
  if (mfaBootError) {
    mfaBootError.hidden = true;
    mfaBootError.textContent = "";
  }
  for (const element of bootErrors()) {
    element.hidden = true;
    element.textContent = "";
  }
}

async function ensureWasm() {
  if (wasmModule) {
    return wasmModule;
  }
  if (!wasmInitPromise) {
    wasmInitPromise = import("../web/pkg/osdf_wasm.js")
      .then(async (mod) => {
        await mod.default();
        wasmModule = mod;
        return mod;
      })
      .catch((error) => {
        wasmInitPromise = null;
        throw error;
      });
  }
  return wasmInitPromise;
}

function buildVerifierConfig() {
  const config = {};
  const ledgerText = ledgerConfigInput.value.trim();
  const identityText = identityConfigInput.value.trim();
  if (ledgerText) {
    config.ledger = JSON.parse(ledgerText);
  }
  if (identityText) {
    config.identity = JSON.parse(identityText);
  }
  return Object.keys(config).length ? JSON.stringify(config) : "{}";
}

async function openPackage(bytes, fileName) {
  clearBootError();

  let mod;
  try {
    mod = await ensureWasm();
  } catch (error) {
    showBootError(
      `WASM verifier failed to load. Run .\\scripts\\build-wasm.ps1 and refresh. (${error})`
    );
    return;
  }

  let report;
  try {
    report = mod.verify_osdf_with_config(bytes, buildVerifierConfig());
  } catch (error) {
    showBootError(`Verification failed: ${error}`);
    return;
  }

  renderVerificationBanner(report, fileName, mod);

  if (report.overall === "FAIL") {
    documentFrame.hidden = true;
    return;
  }

  let documentJson;
  try {
    documentJson = mod.read_package_entry(bytes, "content/document.json");
  } catch (error) {
    showBootError(`Could not read document content: ${error}`);
    return;
  }

  let form;
  try {
    form = JSON.parse(documentJson);
  } catch {
    documentFrame.hidden = false;
    documentFrame.innerHTML =
      "<div class='doc-body'><p>Unsupported content — expected JSON tax form in <code>content/document.json</code>.</p></div>";
    return;
  }

  if (form.type !== "taxForm") {
    documentFrame.hidden = false;
    documentFrame.innerHTML =
      "<div class='doc-body'><p>This gateway PoC renders <code>taxForm</code> documents only.</p></div>";
    return;
  }

  renderTaxForm(form, report, fileName);
}

function renderVerificationBanner(report, fileName, mod) {
  const status = report.overall ?? "INFO";
  verificationBanner.hidden = false;
  verificationBanner.className = `verification-banner ${status.toLowerCase()}`;

  const caveats = (report.findings ?? [])
    .filter((finding) => finding.severity !== "INFO")
    .map((finding) => finding.summary)
    .slice(0, 4);

  verificationBanner.innerHTML = `
    <h2>${status === "PASS" ? "File integrity verified" : status === "WARNING" ? "Verified with caveats" : "Verification failed"}</h2>
    <p><strong>${escapeHtml(fileName)}</strong> · revision ${report.revision ?? "?"} · ${escapeHtml(report.documentId ?? "unknown document")}</p>
    <p class="privacy-note">Gateway verifier osdf-wasm ${escapeHtml(mod.version())} · core ${escapeHtml(mod.core_version())}</p>
    ${caveats.length ? `<ul>${caveats.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}
  `;
}

function renderTaxForm(form, report, fileName) {
  documentFrame.hidden = false;

  const sections = (form.sections ?? [])
    .map((section) => {
      const fields = (section.fields ?? [])
        .map((field) => {
          const wide = field.inputType === "text" && field.id === "address";
          const value = field.value ?? "";
          const empty = !String(value).trim();
          const valueClass =
            field.inputType === "currency" ? "currency" : empty ? "empty" : "";
          const display = empty ? "(blank)" : escapeHtml(String(value));
          return `
            <div class="field ${wide ? "field-wide" : ""}">
              <label for="field-${escapeHtml(field.id)}">${escapeHtml(field.label)}</label>
              <div id="field-${escapeHtml(field.id)}" class="field-value ${valueClass}">${display}</div>
            </div>
          `;
        })
        .join("");

      return `
        <section class="form-section">
          <h3>${escapeHtml(section.title ?? section.id)}</h3>
          <div class="field-grid">${fields}</div>
        </section>
      `;
    })
    .join("");

  documentFrame.innerHTML = `
    <header class="doc-header">
      <span class="revision-badge">Revision ${report.revision ?? "?"}</span>
      <h2>${escapeHtml(form.title ?? fileName)}</h2>
      <p class="lead">${escapeHtml(form.revisionLabel ?? "")}</p>
      <div class="doc-meta">
        <span>Tax year ${escapeHtml(String(form.taxYear ?? ""))}</span>
        <span>${escapeHtml(form.issuer ?? "")}</span>
        <span>Form ${escapeHtml(form.formId ?? "")}</span>
      </div>
    </header>
    <div class="doc-body">${sections}</div>
  `;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

window.addEventListener("error", (event) => {
  if (event.filename?.includes("gateway.js")) {
    showBootError(
      `Script failed to load. Open http://localhost:8081/gateway/ after running .\\scripts\\serve-demo.ps1.`
    );
  }
});

window.addEventListener("unhandledrejection", (event) => {
  showBootError(`Unexpected error: ${event.reason}`);
});
