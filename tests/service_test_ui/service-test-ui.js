(function () {
  "use strict";

  const TERMINAL_READING_STATUSES = new Set(["completed", "failed", "safety_rejected", "cancelled", "expired"]);
  const GENERAL_AUDIENCES = new Set(["general", "beginner", "intermediate", "expert"]);
  const PUBLIC_SERVICES = [
    {
      service_code: "natal_simplified_free",
      label_fr: "Natal simplifie Free",
      description_fr: "Parcours public V2 simplifie sans heure obligatoire.",
      tier: "free",
      kind: "natal_simplified",
      endpoint: "/api/gateway/v2/natal/simplified/free",
      availability: "current",
      sort_order: 10,
    },
    {
      service_code: "natal_simplified_basic",
      label_fr: "Natal simplifie Basic",
      description_fr: "Parcours public V2 simplifie avec tier Basic.",
      tier: "basic",
      kind: "natal_simplified",
      endpoint: "/api/gateway/v2/natal/simplified/basic",
      availability: "current",
      sort_order: 11,
    },
    {
      service_code: "natal_simplified_premium",
      label_fr: "Natal simplifie Premium",
      description_fr: "Parcours public V2 simplifie avec tier Premium.",
      tier: "premium",
      kind: "natal_simplified",
      endpoint: "/api/gateway/v2/natal/simplified/premium",
      availability: "current",
      sort_order: 12,
    },
    {
      service_code: "natal_full_free",
      label_fr: "Natal full Free",
      description_fr: "Parcours public V2 full natal.",
      tier: "free",
      kind: "natal_full",
      endpoint: "/api/gateway/v2/natal/full/free",
      availability: "current",
      sort_order: 20,
    },
    {
      service_code: "natal_full_basic",
      label_fr: "Natal full Basic",
      description_fr: "Parcours public V2 full natal avec tier Basic.",
      tier: "basic",
      kind: "natal_full",
      endpoint: "/api/gateway/v2/natal/full/basic",
      availability: "current",
      sort_order: 21,
    },
    {
      service_code: "natal_full_premium",
      label_fr: "Natal full Premium",
      description_fr: "Parcours public V2 full natal avec tier Premium.",
      tier: "premium",
      kind: "natal_full",
      endpoint: "/api/gateway/v2/natal/full/premium",
      availability: "current",
      sort_order: 22,
    },
    {
      service_code: "horoscope_free_daily",
      label_fr: "Horoscope daily Free",
      description_fr: "Gateway V2 quotidien compact.",
      tier: "free",
      kind: "horoscope_daily",
      endpoint: "/api/gateway/v2/horoscope/daily/free",
      availability: "current",
      sort_order: 30,
    },
    {
      service_code: "horoscope_basic_daily_natal_3_slots",
      label_fr: "Horoscope daily Basic",
      description_fr: "Gateway V2 quotidien en trois slots.",
      tier: "basic",
      kind: "horoscope_daily",
      endpoint: "/api/gateway/v2/horoscope/daily/basic",
      availability: "current",
      sort_order: 31,
    },
    {
      service_code: "horoscope_premium_daily_local_2h_slots",
      label_fr: "Horoscope daily Premium",
      description_fr: "Gateway V2 premium local sur 12 creneaux.",
      tier: "premium",
      kind: "horoscope_daily",
      endpoint: "/api/gateway/v2/horoscope/daily/premium",
      availability: "current",
      sort_order: 32,
    },
    {
      service_code: "horoscope_free_next_7_days_natal",
      label_fr: "Horoscope period Free",
      description_fr: "Gateway V2 periode des 7 prochains jours.",
      tier: "free",
      kind: "horoscope_period",
      endpoint: "/api/gateway/v2/horoscope/period/free",
      availability: "current",
      sort_order: 40,
    },
    {
      service_code: "horoscope_basic_next_7_days_natal",
      label_fr: "Horoscope period Basic",
      description_fr: "Gateway V2 periode Basic.",
      tier: "basic",
      kind: "horoscope_period",
      endpoint: "/api/gateway/v2/horoscope/period/basic",
      availability: "current",
      sort_order: 41,
    },
    {
      service_code: "horoscope_premium_next_7_days_natal",
      label_fr: "Horoscope period Premium",
      description_fr: "Gateway V2 periode Premium.",
      tier: "premium",
      kind: "horoscope_period",
      endpoint: "/api/gateway/v2/horoscope/period/premium",
      availability: "current",
      sort_order: 42,
    },
  ];

  const state = {
    location: null,
    gatewayServices: PUBLIC_SERVICES.slice().sort((a, b) => a.sort_order - b.sort_order),
    diagnosticServices: [],
    natalChartByFingerprint: null,
  };

  function todayIso() {
    return new Date().toISOString().slice(0, 10);
  }

  function normalizeTime(raw) {
    if (!raw) return "";
    return raw.length === 5 ? `${raw}:00` : raw;
  }

  function valueOf(id) {
    const element = document.getElementById(id);
    return element ? element.value.trim() : "";
  }

  function readForm() {
    return {
      birthDate: valueOf("birthDate"),
      birthTime: normalizeTime(valueOf("birthTime")),
      timezone: valueOf("timezone") || "Europe/Paris",
      city: valueOf("city"),
      country: valueOf("country"),
      targetDate: valueOf("targetDate") || todayIso(),
      language: (valueOf("language") || "fr").toLowerCase(),
      audience: valueOf("audience") || "general",
      llmApiKey: valueOf("llmApiKey"),
      calculatorApiKey: valueOf("calculatorApiKey"),
      location: state.location,
    };
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

  async function apiFetch(path, options) {
    const headers = { ...(options && options.headers ? options.headers : {}) };
    if (options && options.body) headers["content-type"] = "application/json";
    const apiKey = apiKeyForPath(path);
    if (apiKey) headers["X-API-Key"] = apiKey;
    const response = await fetch(path, { ...options, headers });
    const text = await response.text();
    const body = parseResponseBody(text, response.headers.get("content-type") || "");
    if (!response.ok) {
      const message = errorMessageFromBody(body) || response.statusText || `HTTP ${response.status}`;
      const err = new Error(message);
      err.status = response.status;
      err.body = body;
      throw err;
    }
    return body;
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

  function isHoroscopeService(service) {
    return service.kind === "horoscope_daily" || service.kind === "horoscope_period";
  }

  function serviceNeedsBirthTime(service) {
    return service.kind === "natal_full" || isHoroscopeService(service);
  }

  function serviceNeedsLocation(service) {
    return true;
  }

  function serviceCanRun(service, input) {
    if (!input.birthDate) return "Date de naissance requise.";
    if (serviceNeedsLocation(service) && !input.location) return "Resoudre le lieu avant execution.";
    if (serviceNeedsBirthTime(service) && !input.birthTime) return "Heure de naissance requise pour ce service.";
    if (!GENERAL_AUDIENCES.has(input.audience)) return "Audience invalide.";
    return "";
  }

  function buildGatewayNatalRequest(input) {
    const birth = {
      date: input.birthDate,
      timezone: input.timezone,
      location: {
        latitude: input.location.latitude,
        longitude: input.location.longitude,
        label: input.location.label,
      },
    };
    if (input.birthTime) birth.time = input.birthTime;
    return {
      context: {
        request_id: `service-test-ui-${Date.now()}`,
        idempotency_key: newClientId(),
        target_language_code: input.language,
        audience_level: normalizeGatewayAudience(input.audience),
      },
      birth,
    };
  }

  function buildCalculatorNatalPayload(input) {
    return {
      request_contract_version: "astro_engine_request_v1",
      request_id: `service-test-ui-chart-${Date.now()}`,
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
        },
      },
      projection: {
        level: "compact",
      },
    };
  }

  function buildGatewayHoroscopeDailyRequest(input, chartCalculationId, service) {
    const payload = {
      date: input.targetDate,
      timezone: input.timezone,
      target_language: input.language,
      chart_calculation_id: String(chartCalculationId),
      audience_level: normalizeHoroscopeAudience(input.audience),
    };
    if (service.tier === "premium") {
      payload.location = {
        latitude: input.location.latitude,
        longitude: input.location.longitude,
        label: input.location.label,
      };
      payload.detail_level = "premium_rich";
    }
    return payload;
  }

  function buildGatewayHoroscopePeriodRequest(input, chartCalculationId) {
    return {
      anchor_date: input.targetDate,
      timezone: input.timezone,
      target_language: input.language,
      target_language_code: supportedTargetLanguageCode(input.language),
      chart_calculation_id: String(chartCalculationId),
      audience_level: normalizeHoroscopeAudience(input.audience),
    };
  }

  function supportedTargetLanguageCode(value) {
    return ["fr", "en", "es", "de"].includes(value) ? value : null;
  }

  function normalizeGatewayAudience(value) {
    if (value === "general") return "general";
    if (value === "beginner" || value === "intermediate" || value === "expert") return value;
    return "general";
  }

  function normalizeHoroscopeAudience(value) {
    return value === "expert" ? "expert" : "general";
  }

  async function ensureNatalChartCalculationId(input) {
    const fingerprint = JSON.stringify({
      birthDate: input.birthDate,
      birthTime: input.birthTime,
      timezone: input.timezone,
      location: input.location,
    });
    if (state.natalChartByFingerprint && state.natalChartByFingerprint.fingerprint === fingerprint) {
      return state.natalChartByFingerprint.chartCalculationId;
    }
    const response = await apiFetch("/api/calculator/v1/calculations/natal", {
      method: "POST",
      body: JSON.stringify(buildCalculatorNatalPayload(input)),
    });
    const chartCalculationId = response && response.calculation_result && response.calculation_result.chart_calculation_id;
    if (!chartCalculationId) {
      throw new Error("Le calcul natal ne contient pas chart_calculation_id.");
    }
    state.natalChartByFingerprint = { fingerprint, chartCalculationId };
    return chartCalculationId;
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
    state.location = {
      latitude: Number(data.latitude),
      longitude: Number(data.longitude),
      label: data.label,
      countryCode: data.country_code || null,
    };
    state.natalChartByFingerprint = null;
    if (result) {
      result.textContent = `${state.location.label} - lat ${state.location.latitude}, lon ${state.location.longitude}`;
    }
    renderServices();
  }

  async function loadHealth() {
    await Promise.all([
      setHealth("gatewayStatus", "/api/gateway/health/ready"),
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

  async function loadDiagnosticServices() {
    const count = document.getElementById("diagnosticCount");
    if (count) count.textContent = "Chargement diagnostic async interne...";
    try {
      const data = await apiFetch("/api/llm/v1/services", { method: "GET" });
      state.diagnosticServices = (data.services || []).filter((service) => ["active", "beta"].includes(service.availability));
      if (count) count.textContent = `${state.diagnosticServices.length} service(s) async interne(s) actifs/beta`;
    } catch (err) {
      state.diagnosticServices = [];
      if (count) count.textContent = `Diagnostic async indisponible: ${err.message}`;
    }
    renderServices();
  }

  function renderServices() {
    const grid = document.getElementById("serviceGrid");
    const count = document.getElementById("serviceCount");
    const template = document.getElementById("serviceCardTemplate");
    if (!grid || !template) return;
    grid.textContent = "";
    if (count) count.textContent = `${state.gatewayServices.length} service(s) publics V2`;

    state.gatewayServices.forEach((service) => {
      const input = readForm();
      const blockedReason = serviceCanRun(service, input);
      const card = template.content.firstElementChild.cloneNode(true);
      card.dataset.serviceCode = service.service_code;
      card.querySelector("h3").textContent = service.label_fr;
      card.querySelector(".service-description").textContent = service.description_fr;
      const availability = card.querySelector(".availability");
      availability.textContent = service.availability;
      card.querySelector(".service-meta").innerHTML = buildMeta(service).map((item) => `<span class="meta-pill">${escapeHtml(item)}</span>`).join("");
      const button = card.querySelector(".run-service");
      button.disabled = Boolean(blockedReason);
      button.textContent = blockedReason || "Executer";
      button.addEventListener("click", () => runService(service, card));
      attachResultControls(card);
      grid.appendChild(card);
    });
  }

  function buildMeta(service) {
    const relatedAsync = state.diagnosticServices.find((item) => item.api_surface && item.api_surface.recommended_entrypoint === service.endpoint.replace("/api/gateway", ""));
    return [
      service.service_code,
      service.endpoint.replace("/api/gateway", ""),
      service.kind,
      `tier:${service.tier}`,
      relatedAsync ? `async:${relatedAsync.service_code}` : "async:unmapped",
    ];
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

    const button = card.querySelector(".run-service");
    button.disabled = true;
    button.textContent = "Execution...";
    const started = Date.now();
    showProgress(card, "Preparation...");

    try {
      let requestBody;
      if (service.kind === "natal_simplified" || service.kind === "natal_full") {
        requestBody = buildGatewayNatalRequest(input);
      } else {
        showProgress(card, "Calcul natal de reference...");
        const chartCalculationId = await ensureNatalChartCalculationId(input);
        if (service.kind === "horoscope_daily") {
          requestBody = buildGatewayHoroscopeDailyRequest(input, chartCalculationId, service);
        } else {
          requestBody = buildGatewayHoroscopePeriodRequest(input, chartCalculationId);
        }
      }
      showProgress(card, `POST ${service.endpoint.replace("/api/gateway", "")}`);
      const response = await apiFetch(service.endpoint, {
        method: "POST",
        body: JSON.stringify(requestBody),
      });
      renderResult(card, response, Date.now() - started, service);
    } catch (err) {
      showError(card, err.message, err.body || null);
    } finally {
      button.disabled = false;
      button.textContent = "Executer";
    }
  }

  function showProgress(card, message) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    region.hidden = false;
    reading.hidden = false;
    card.querySelector(".json-view").hidden = true;
    reading.innerHTML = `<div class="reading-block"><h4>${escapeHtml(message)}</h4><p>Veuillez patienter.</p></div>`;
    card.querySelector(".tech-details").textContent = message;
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

  function renderResult(card, response, elapsedMs, service) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    const json = card.querySelector(".json-view");
    const tech = card.querySelector(".tech-details");
    const normalized = normalizeGatewayReadingSections(response);
    region.hidden = false;
    json.textContent = JSON.stringify(response, null, 2);
    tech.innerHTML = [
      `Endpoint: ${escapeHtml(service.endpoint.replace("/api/gateway", ""))}`,
      `Service: ${escapeHtml(service.service_code)}`,
      `Duree UI: ${Math.round(elapsedMs / 1000)} s`,
      `Produit: ${escapeHtml(readField(response, ["metadata", "product_code"]) || "")}`,
      `Variant: ${escapeHtml(readField(response, ["metadata", "variant"]) || "")}`,
    ].join("<br>");
    if (!normalized.length) {
      reading.innerHTML = `<div class="reading-block"><h4>Aucun rendu simplifie disponible</h4><p>Utiliser l'onglet JSON brut pour inspecter la reponse.</p></div>`;
      return;
    }
    reading.innerHTML = normalized.map(renderSection).join("");
  }

  function normalizeGatewayReadingSections(payload) {
    if (!payload || typeof payload !== "object") return [];
    const reading = extractReadingPayload(payload);
    const sections = [];

    if (payload.metadata) {
      sections.push({
        title: "Metadata",
        paragraphs: [
          `product_code: ${payload.metadata.product_code || ""}`,
          `variant: ${payload.metadata.variant || ""}`,
          `tier: ${payload.metadata.tier || ""}`,
        ].filter(Boolean),
      });
    }

    if (payload.quality) {
      sections.push({
        title: "Quality",
        paragraphs: [
          payload.quality.calculator_contract_version,
          payload.quality.llm_contract_version,
          payload.quality.reading_completeness,
        ].filter(Boolean),
      });
    }

    return sections.concat(normalizeReadingSections(reading));
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

    if (payload.status && TERMINAL_READING_STATUSES.has(payload.status) && payload.error) {
      sections.push({
        title: payload.status === "safety_rejected" ? "Generation rejetee" : "Erreur",
        paragraphs: [payload.error.message || payload.error.code].filter(Boolean),
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
      const meta = [item.slot_label, item.day_label, item.date].filter((value) => value && value !== title);
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

  function readField(obj, path) {
    return path.reduce((value, key) => (value && Object.prototype.hasOwnProperty.call(value, key) ? value[key] : null), obj);
  }

  function escapeHtml(value) {
    return String(value || "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#039;");
  }

  function newClientId() {
    if (window.crypto && typeof window.crypto.randomUUID === "function") {
      return window.crypto.randomUUID();
    }
    return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
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
      loadDiagnosticServices().catch((err) => {
        const count = document.getElementById("diagnosticCount");
        if (count) count.textContent = err.message;
      });
    });
    ["birthDate", "birthTime", "timezone", "city", "country", "targetDate", "language", "audience"].forEach((id) => {
      const element = document.getElementById(id);
      if (!element) return;
      element.addEventListener("change", () => {
        if (["birthDate", "birthTime", "timezone"].includes(id)) {
          state.natalChartByFingerprint = null;
        }
        if (["city", "country"].includes(id)) {
          state.location = null;
          state.natalChartByFingerprint = null;
          const result = document.getElementById("locationResult");
          if (result) result.textContent = "Lieu modifie, resolution a relancer.";
        }
        renderServices();
      });
    });
    renderServices();
    loadHealth().catch(() => {});
    loadDiagnosticServices().catch(() => {});
  }

  window.AstralServiceTestUi = {
    buildGatewayNatalRequest,
    buildGatewayHoroscopeDailyRequest,
    buildGatewayHoroscopePeriodRequest,
    buildCalculatorNatalPayload,
    isHoroscopeService,
    normalizeGatewayAudience,
    normalizeHoroscopeAudience,
    normalizeReadingSections,
    normalizeGatewayReadingSections,
    parseResponseBody,
    serviceCanRun,
    supportedTargetLanguageCode,
    newClientId,
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      if (document.getElementById("serviceGrid")) boot();
    });
  } else if (document.getElementById("serviceGrid")) {
    boot();
  }
}());
