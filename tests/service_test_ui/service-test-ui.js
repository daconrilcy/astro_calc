(function () {
  "use strict";

  const RUNNABLE_AVAILABILITY = new Set(["active", "beta"]);
  const TERMINAL_STATUSES = new Set(["completed", "failed", "safety_rejected", "cancelled", "expired"]);
  const GEOCODE_REQUIRED_MESSAGE = "Resoudre le lieu avant de generer.";
  const FULL_NATAL_TIME_REQUIRED = "Heure de naissance requise pour ce service.";

  const state = {
    services: [],
    location: null,
    fullNatalByInput: null,
  };

  function todayIso() {
    return new Date().toISOString().slice(0, 10);
  }

  function normalizeTime(raw) {
    if (!raw) return "";
    return raw.length === 5 ? `${raw}:00` : raw;
  }

  function readForm() {
    return {
      birthDate: valueOf("birthDate"),
      birthTime: normalizeTime(valueOf("birthTime")),
      timezone: valueOf("timezone") || "Europe/Paris",
      city: valueOf("city"),
      country: valueOf("country"),
      targetDate: valueOf("targetDate") || todayIso(),
      language: valueOf("language") || "fr",
      audience: valueOf("audience") || "beginner",
      llmApiKey: valueOf("llmApiKey"),
      calculatorApiKey: valueOf("calculatorApiKey"),
      location: state.location,
    };
  }

  function valueOf(id) {
    const element = document.getElementById(id);
    return element ? element.value.trim() : "";
  }

  function filterRunnableServices(services) {
    return (services || [])
      .filter((service) => RUNNABLE_AVAILABILITY.has(service.availability))
      .sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0));
  }

  function buildNatalSimplifiedPayload(input) {
    const birth = {
      date: input.birthDate,
      location: {
        latitude: input.location.latitude,
        longitude: input.location.longitude,
        label: input.location.label,
      },
    };
    if (input.birthTime) birth.time = input.birthTime;
    if (input.timezone) birth.timezone = input.timezone;
    return {
      request_contract_version: "astro_simplified_natal_request_v1",
      birth,
      input_metadata: {
        location_label: input.location.label,
      },
      calculation: {
        zodiacal_reference_system: "tropical",
        house_system: "placidus",
      },
    };
  }

  function buildFullNatalPayload(input, projectionLevel) {
    return {
      request_contract_version: "astro_engine_request_v1",
      calculation: {
        type: "natal",
        zodiacal_reference_system: "tropical",
        coordinate_reference_system: "geocentric",
        house_system: "placidus",
      },
      birth: {
        date: input.birthDate,
        time: input.birthTime,
        timezone: input.timezone,
        location: {
          label: input.location.label,
          latitude: input.location.latitude,
          longitude: input.location.longitude,
          country_code: input.location.countryCode || null,
        },
        time_precision: "exact",
      },
      projection: {
        contract_version: "llm_projection_natal_v1",
        level: projectionLevel || "compact",
      },
    };
  }

  function buildHoroscopePayload(service, input, chartCalculationId) {
    const code = service.service_code;
    const base = {
      timezone: input.timezone,
      target_language: input.language,
      chart_calculation_id: String(chartCalculationId),
      audience_level: input.audience === "expert" ? "expert" : "general",
    };

    if (code.includes("next_7_days")) {
      return {
        ...base,
        anchor_date: input.targetDate,
      };
    }

    const payload = {
      ...base,
      date: input.targetDate,
    };

    if (code === "horoscope_premium_daily_local_2h_slots") {
      payload.location = {
        latitude: input.location.latitude,
        longitude: input.location.longitude,
        label: input.location.label,
      };
      payload.detail_level = "premium_rich";
    }

    return payload;
  }

  function buildJobBody(service, payload, input) {
    return {
      service_code: service.service_code,
      payload,
      user_language: input.language,
      audience_level: integrationAudienceLevel(input.audience),
    };
  }

  function integrationAudienceLevel(value) {
    return ["beginner", "intermediate", "expert"].includes(value) ? value : "beginner";
  }

  function serviceNeedsFullNatal(service) {
    return service.calculation_mode === "full_natal" || isHoroscopeService(service);
  }

  function serviceNeedsLocation(service) {
    return service.calculation_mode === "simplified_natal" ||
      service.calculation_mode === "full_natal" ||
      isHoroscopeService(service);
  }

  function isHoroscopeService(service) {
    return service.product_code === "horoscope" ||
      service.service_code.startsWith("horoscope_") ||
      String(service.orchestration_mode || "").startsWith("horoscope_") ||
      String(service.contracts && service.contracts.payload || "").startsWith("horoscope_");
  }

  function serviceCanRun(service, input) {
    if (serviceNeedsLocation(service) && !input.location) return GEOCODE_REQUIRED_MESSAGE;
    if (serviceNeedsFullNatal(service) && !input.birthTime) return FULL_NATAL_TIME_REQUIRED;
    if (!input.birthDate) return "Date de naissance requise.";
    return "";
  }

  async function apiFetch(path, options) {
    const headers = {
      ...(options && options.headers ? options.headers : {}),
    };
    if (options && options.body) headers["content-type"] = "application/json";
    const apiKey = apiKeyForPath(path);
    if (apiKey) headers["X-API-Key"] = apiKey;
    const response = await fetch(path, {
      ...options,
      headers,
    });
    const text = await response.text();
    const body = parseResponseBody(text, response.headers.get("content-type") || "");
    if (!response.ok) {
      const message = errorMessageFromBody(body) || response.statusText || `HTTP ${response.status}`;
      const err = new Error(message || `HTTP ${response.status}`);
      err.status = response.status;
      err.body = body;
      throw err;
    }
    return body;
  }

  function apiKeyForPath(path) {
    if (path.startsWith("/api/calculator/")) {
      return valueOf("calculatorApiKey") || valueOf("llmApiKey");
    }
    if (path.startsWith("/api/llm/")) {
      return valueOf("llmApiKey");
    }
    return "";
  }

  function parseResponseBody(text, contentType) {
    if (!text) return null;
    if (contentType.includes("application/json") || contentType.includes("+json")) {
      try {
        return JSON.parse(text);
      } catch (err) {
        return { error: { code: "INVALID_JSON_RESPONSE", message: text } };
      }
    }
    try {
      return JSON.parse(text);
    } catch (err) {
      return { error: { code: "NON_JSON_RESPONSE", message: text } };
    }
  }

  function errorMessageFromBody(body) {
    if (!body) return "";
    if (body.error && typeof body.error === "object") {
      return body.error.message || body.error.code || "";
    }
    if (typeof body.message === "string") return body.message;
    return "";
  }

  async function loadHealth() {
    await Promise.all([
      setHealth("llmStatus", "/api/llm/health/ready"),
      setHealth("calculatorStatus", "/api/calculator/health/ready"),
    ]);
  }

  async function setHealth(id, path) {
    const element = document.getElementById(id);
    if (!element) return;
    try {
      await apiFetch(path, { method: "GET" });
      element.classList.remove("error");
      element.classList.add("ok");
    } catch (err) {
      element.classList.remove("ok");
      element.classList.add("error");
    }
  }

  async function loadServices() {
    const count = document.getElementById("serviceCount");
    if (count) count.textContent = "Chargement...";
    const data = await apiFetch("/api/llm/v1/services", { method: "GET" });
    state.services = filterRunnableServices(data.services || []);
    renderServices();
  }

  async function resolveLocation() {
    const input = readForm();
    const result = document.getElementById("locationResult");
    if (!input.city || !input.country) {
      if (result) result.textContent = "Ville et pays requis.";
      return;
    }
    if (result) result.textContent = "Resolution du lieu...";
    const params = new URLSearchParams({ city: input.city, country: input.country });
    const data = await apiFetch(`/api/geocode?${params.toString()}`, { method: "GET" });
    const latitude = Number(data.latitude);
    const longitude = Number(data.longitude);
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
      throw new Error("Le service de geocodage n'a pas renvoye de coordonnees valides.");
    }
    state.location = {
      latitude,
      longitude,
      label: data.label,
      countryCode: data.country_code || null,
    };
    state.fullNatalByInput = null;
    if (result) {
      result.textContent = `${state.location.label} - lat ${state.location.latitude}, lon ${state.location.longitude}`;
    }
    renderServices();
  }

  function renderServices() {
    const grid = document.getElementById("serviceGrid");
    const count = document.getElementById("serviceCount");
    const template = document.getElementById("serviceCardTemplate");
    if (!grid || !template) return;
    grid.textContent = "";
    if (count) count.textContent = `${state.services.length} service(s) actif(s) ou beta`;

    state.services.forEach((service) => {
      const input = readForm();
      const blockedReason = serviceCanRun(service, input);
      const card = template.content.firstElementChild.cloneNode(true);
      card.dataset.serviceCode = service.service_code;
      card.querySelector("h3").textContent = service.label_fr || service.service_code;
      card.querySelector(".service-description").textContent = service.description_fr || "";
      const availability = card.querySelector(".availability");
      availability.textContent = service.availability;
      availability.classList.toggle("beta", service.availability === "beta");
      card.querySelector(".service-meta").innerHTML = [
        service.service_code,
        service.profile_code || service.interpretation_profile_code,
        service.product_code || service.quality_tier,
        service.payload_contract || (service.contracts && service.contracts.payload),
      ].filter(Boolean).map((item) => `<span class="meta-pill">${escapeHtml(item)}</span>`).join("");
      const button = card.querySelector(".run-service");
      button.disabled = Boolean(blockedReason);
      button.textContent = blockedReason || "Generer en reel";
      button.addEventListener("click", () => runService(service, card));
      attachResultControls(card);
      grid.appendChild(card);
    });
  }

  function attachResultControls(card) {
    const tabs = card.querySelectorAll(".tab");
    const reading = card.querySelector(".reading-view");
    const json = card.querySelector(".json-view");
    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        tabs.forEach((item) => item.classList.remove("active"));
        tab.classList.add("active");
        const showJson = tab.dataset.tab === "json";
        reading.hidden = showJson;
        json.hidden = !showJson;
      });
    });
    card.querySelector(".toggle-tech").addEventListener("click", () => {
      const details = card.querySelector(".tech-details");
      details.hidden = !details.hidden;
    });
    card.querySelector(".copy-json").addEventListener("click", async () => {
      const button = card.querySelector(".copy-json");
      try {
        if (!navigator.clipboard || !navigator.clipboard.writeText) {
          throw new Error("clipboard unavailable");
        }
        await navigator.clipboard.writeText(json.textContent || "");
        flashButtonLabel(button, "Copie");
      } catch (err) {
        flashButtonLabel(button, "Copie impossible");
      }
    });
  }

  async function runService(service, card) {
    const input = readForm();
    const blockedReason = serviceCanRun(service, input);
    if (blockedReason) {
      showError(card, blockedReason);
      return;
    }

    const started = Date.now();
    const button = card.querySelector(".run-service");
    button.disabled = true;
    button.textContent = "Generation...";
    showProgress(card, "Preparation du payload...");

    try {
      let payload;
      if (isHoroscopeService(service)) {
        showProgress(card, "Calcul natal de reference...");
        const chartId = await ensureFullNatalChartId(input);
        payload = buildHoroscopePayload(service, input, chartId);
      } else if (service.calculation_mode === "simplified_natal") {
        payload = buildNatalSimplifiedPayload(input);
      } else {
        payload = buildFullNatalPayload(input, projectionLevelForService(service));
      }

      const body = buildJobBody(service, payload, input);
      const idempotencyKey = `${service.service_code}-${newClientId()}`;
      showProgress(card, "Soumission du job...");
      const submitted = await apiFetch("/api/llm/v1/jobs", {
        method: "POST",
        headers: {
          "Idempotency-Key": idempotencyKey,
          "X-Tenant-Id": "service-test-ui",
        },
        body: JSON.stringify(body),
      });
      const finalStatus = submitted.result ? submitted : await pollJob(submitted.run_id, card);
      renderResult(card, finalStatus, Date.now() - started);
    } catch (err) {
      showError(card, err.message, err.body || null);
    } finally {
      button.disabled = false;
      button.textContent = "Generer en reel";
    }
  }

  function projectionLevelForService(service) {
    const code = service.service_code || "";
    const tier = service.quality_tier || "";
    const profile = service.profile_code || service.interpretation_profile_code || "";
    if (code.includes("premium") || tier.includes("premium") || profile.includes("premium")) {
      return "rich";
    }
    return "compact";
  }

  async function ensureFullNatalChartId(input) {
    const fingerprint = JSON.stringify({
      date: input.birthDate,
      time: input.birthTime,
      timezone: input.timezone,
      location: input.location,
    });
    if (state.fullNatalByInput && state.fullNatalByInput.fingerprint === fingerprint) {
      return state.fullNatalByInput.chartId;
    }

    const payload = buildFullNatalPayload(input, "compact");
    const response = await apiFetch("/api/calculator/v1/calculations/natal", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    const chartId = response && response.calculation_result && response.calculation_result.chart_calculation_id;
    if (!chartId) throw new Error("Le calcul natal ne contient pas chart_calculation_id.");
    state.fullNatalByInput = { fingerprint, chartId };
    return chartId;
  }

  async function pollJob(runId, card) {
    let last;
    for (let attempt = 0; attempt < 900; attempt += 1) {
      await sleep(attempt === 0 ? 1000 : 2000);
      last = await apiFetch(`/api/llm/v1/jobs/${encodeURIComponent(runId)}`, {
        method: "GET",
        headers: { "X-Tenant-Id": "service-test-ui" },
      });
      showProgress(card, `Job ${last.status}...`);
      if (TERMINAL_STATUSES.has(last.status)) return last;
    }
    throw new Error(`Timeout en attente du job ${runId}.`);
  }

  function showProgress(card, message) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    const tech = card.querySelector(".tech-details");
    region.hidden = false;
    reading.hidden = false;
    card.querySelector(".json-view").hidden = true;
    reading.innerHTML = `<div class="reading-block"><h4>${escapeHtml(message)}</h4><p>Veuillez patienter.</p></div>`;
    tech.textContent = message;
  }

  function showError(card, message, body) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    const json = card.querySelector(".json-view");
    region.hidden = false;
    reading.innerHTML = `<div class="reading-block"><h4 class="error-text">Erreur</h4><p>${escapeHtml(message)}</p></div>`;
    json.textContent = body ? JSON.stringify(body, null, 2) : "";
    card.querySelector(".tech-details").textContent = message;
  }

  function renderResult(card, jobStatus, elapsedMs) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    const json = card.querySelector(".json-view");
    const tech = card.querySelector(".tech-details");
    region.hidden = false;
    json.textContent = JSON.stringify(jobStatus, null, 2);
    tech.innerHTML = renderTechDetails(jobStatus, elapsedMs);
    reading.innerHTML = renderReadableResult(jobStatus);
  }

  function renderTechDetails(jobStatus, elapsedMs) {
    const error = jobStatus.error ? `<br>Erreur: ${escapeHtml(jobStatus.error.message || jobStatus.error.code || "")}` : "";
    return [
      `Statut: ${escapeHtml(jobStatus.status || "")}`,
      `Service: ${escapeHtml(jobStatus.service_code || "")}`,
      `Run: ${escapeHtml(jobStatus.run_id || "")}`,
      `Duree UI: ${Math.round(elapsedMs / 1000)} s${error}`,
    ].join("<br>");
  }

  function renderReadableResult(jobStatus) {
    if (jobStatus.status !== "completed" && jobStatus.status !== "safety_rejected") {
      return `<div class="reading-block"><h4 class="error-text">${escapeHtml(jobStatus.status || "Erreur")}</h4><p>${escapeHtml(errorMessage(jobStatus))}</p></div>`;
    }

    const result = jobStatus.result || jobStatus;
    const reading = extractReadingPayload(result);
    const sections = normalizeReadingSections(reading);
    if (!sections.length) {
      return `<div class="reading-block"><h4>Aucun rendu simplifie disponible</h4><p>Utiliser l'onglet JSON brut pour inspecter la reponse.</p></div>`;
    }
    return sections.map(renderSection).join("");
  }

  function extractReadingPayload(result) {
    if (!result) return null;
    if (result.reading && result.reading.reading) return result.reading.reading;
    if (result.reading && result.reading.status === "success") return result.reading.reading;
    if (result.reading) return result.reading;
    return result;
  }

  function normalizeReadingSections(payload) {
    if (!payload || typeof payload !== "object") return [];
    const sections = [];

    if (payload.error) {
      const message = payload.error.message || payload.error.code || "La generation n'a pas abouti.";
      const violations = Array.isArray(payload.violations) ? payload.violations : [];
      sections.push({
        title: payload.status === "safety_rejected" ? "Generation rejetee" : "Erreur",
        paragraphs: [message, ...violations].filter(Boolean),
      });
    }

    if (payload.summary) {
      sections.push({
        title: payload.summary.title || "Resume",
        paragraphs: [payload.summary.short_text || payload.summary.text].filter(Boolean),
      });
    }

    if (Array.isArray(payload.chapters)) {
      payload.chapters.forEach((chapter) => {
        sections.push({
          title: chapter.title || chapter.code || "Chapitre",
          paragraphs: splitParagraphs(chapter.body),
        });
      });
    }

    if (payload.dominant_theme) {
      sections.push({
        title: payload.dominant_theme.theme || "Theme dominant",
        paragraphs: [payload.dominant_theme.text].filter(Boolean),
      });
    }

    if (payload.week_overview) {
      sections.push({
        title: payload.week_overview.title || "Vue de la semaine",
        paragraphs: [payload.week_overview.text, payload.week_overview.trajectory].filter(Boolean),
      });
    }

    addStringSection(sections, "Conseil", payload.advice);
    addObjectAdvice(sections, payload.advice);
    addStringSection(sections, "Point de vigilance", payload.watch_point);
    addObjectTextSection(sections, "Vigilance", payload.watch_summary);

    addArraySections(sections, "Creneau", payload.slots, ["text", "advice", "watch_point"]);
    addArraySections(sections, "Meilleur moment", payload.best_slots, ["reason"]);
    addArraySections(sections, "Moment de vigilance", payload.watch_slots, ["reason"]);
    addArraySections(sections, "Timeline", payload.timeline, ["text", "advice", "watch_point"]);
    addArraySections(sections, "Jour cle", payload.key_days, ["reason"]);
    addArraySections(sections, "Meilleur jour", payload.best_days, ["reason"]);
    addArraySections(sections, "Jour de vigilance", payload.watch_days, ["reason"]);
    addArraySections(sections, "Timeline quotidienne", payload.daily_timeline, ["text", "advice"]);
    addArraySections(sections, "Domaine", payload.domain_sections, ["text"]);
    addArraySections(sections, "Fenetre favorable", payload.best_windows, ["reason", "advice"]);
    addArraySections(sections, "Fenetre de vigilance", payload.watch_windows, ["reason", "advice"]);
    addObjectAdvice(sections, payload.strategy, "Strategie");

    return sections.filter((section) => section.paragraphs.length > 0 || section.items);
  }

  function addStringSection(sections, title, value) {
    if (typeof value === "string" && value.trim()) {
      sections.push({ title, paragraphs: [value] });
    }
  }

  function addObjectTextSection(sections, title, value) {
    if (value && typeof value === "object" && value.text) {
      sections.push({ title, paragraphs: [value.text] });
    }
  }

  function addObjectAdvice(sections, value, title) {
    if (!value || typeof value !== "object") return;
    const paragraphs = [value.main, value.best_use, value.avoid, value.overview, value.action].filter(Boolean);
    if (paragraphs.length) sections.push({ title: title || "Conseil", paragraphs });
  }

  function addArraySections(sections, fallbackTitle, list, paragraphKeys) {
    if (!Array.isArray(list)) return;
    list.forEach((item) => {
      if (!item || typeof item !== "object") return;
      const title = item.title || item.day_label || item.slot_label || item.date || item.domain || fallbackTitle;
      const meta = [item.slot_label, item.day_label, item.date]
        .filter((value) => value && value !== title);
      const paragraphs = paragraphKeys.flatMap((key) => splitParagraphs(item[key])).filter(Boolean);
      sections.push({ title, meta, paragraphs });
    });
  }

  function splitParagraphs(value) {
    if (!value || typeof value !== "string") return [];
    return value.split(/\n{2,}|\r?\n/).map((part) => part.trim()).filter(Boolean);
  }

  function renderSection(section) {
    const meta = Array.isArray(section.meta) && section.meta.length
      ? `<div class="reading-meta">${section.meta.map(escapeHtml).join(" · ")}</div>`
      : "";
    return `<section class="reading-block"><h4>${escapeHtml(section.title)}</h4>${meta}${section.paragraphs.map((p) => `<p>${escapeHtml(p)}</p>`).join("")}</section>`;
  }

  function errorMessage(jobStatus) {
    return (jobStatus.error && (jobStatus.error.message || jobStatus.error.code)) || "La generation n'a pas abouti.";
  }

  function escapeHtml(value) {
    return String(value || "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#039;");
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function newClientId() {
    if (window.crypto && typeof window.crypto.randomUUID === "function") {
      return window.crypto.randomUUID();
    }
    const random = window.crypto && window.crypto.getRandomValues
      ? Array.from(window.crypto.getRandomValues(new Uint32Array(4)), (part) => part.toString(16)).join("")
      : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
    return random;
  }

  function flashButtonLabel(button, label) {
    if (!button) return;
    const previous = button.textContent;
    button.textContent = label;
    window.setTimeout(() => {
      button.textContent = previous;
    }, 1200);
  }

  function boot() {
    const targetDate = document.getElementById("targetDate");
    if (targetDate && !targetDate.value) targetDate.value = todayIso();
    document.getElementById("resolveLocation").addEventListener("click", () => {
      resolveLocation().catch((err) => {
        const result = document.getElementById("locationResult");
        if (result) result.textContent = err.message;
      });
    });
    document.getElementById("reloadServices").addEventListener("click", () => {
      loadServices().catch((err) => {
        const count = document.getElementById("serviceCount");
        if (count) count.textContent = err.message;
      });
    });
    ["birthDate", "birthTime", "timezone", "city", "country", "targetDate", "language", "audience"].forEach((id) => {
      const element = document.getElementById(id);
      if (element) element.addEventListener("change", () => {
        if (["birthDate", "birthTime", "timezone"].includes(id)) state.fullNatalByInput = null;
        if (["city", "country"].includes(id)) {
          state.location = null;
          state.fullNatalByInput = null;
          const result = document.getElementById("locationResult");
          if (result) result.textContent = "Lieu modifie, resolution a relancer.";
        }
        renderServices();
      });
    });
    loadHealth().catch(() => {});
    loadServices().catch((err) => {
      const count = document.getElementById("serviceCount");
      if (count) count.textContent = err.message;
    });
  }

  window.AstralServiceTestUi = {
    buildFullNatalPayload,
    buildHoroscopePayload,
    buildNatalSimplifiedPayload,
    buildJobBody,
    filterRunnableServices,
    integrationAudienceLevel,
    isHoroscopeService,
    newClientId,
    normalizeReadingSections,
    parseResponseBody,
    projectionLevelForService,
    serviceCanRun,
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      if (document.getElementById("serviceGrid")) boot();
    });
  } else if (document.getElementById("serviceGrid")) {
    boot();
  }
}());
