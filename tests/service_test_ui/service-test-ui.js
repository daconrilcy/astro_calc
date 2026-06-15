(function () {
  "use strict";

  const TERMINAL_READING_STATUSES = new Set(["completed", "failed", "safety_rejected", "cancelled", "expired"]);
  const GENERAL_AUDIENCES = new Set(["general", "beginner", "intermediate", "expert"]);
  const MODES = { complete: "complete", degraded: "degraded" };
  const FRAME_DEFINITIONS = {
    natal: {
      title: "Theme natal",
      description: "Services natals executes en sequence Free, Basic puis Premium.",
      modeLabel: "Mode degrade",
      supportsBatch: true,
    },
    horoscope: {
      title: "Horoscope",
      description: "Sous-cadres daily et period, avec orchestration sequentielle par tier.",
      modeLabel: "Mode degrade",
      supportsBatch: false,
    },
    other: {
      title: "Autres interpretations",
      description: "Perimetre reserve pour les futurs services.",
      placeholder: true,
      modeLabel: "Mode degrade",
      supportsBatch: false,
    },
  };
  const SUBFRAME_DESCRIPTIONS = {
    daily: "Horoscope quotidien, toujours ordonne Free, Basic, Premium.",
    period: "Horoscope de periode, toujours ordonne Free, Basic, Premium.",
  };
  const PUBLIC_SERVICES = [
    {
      service_code: "natal_simplified_free",
      label_fr: "Natal simplifie Free",
      description_fr: "Parcours public V2 simplifie sans heure obligatoire.",
      tier: "free",
      kind: "natal_simplified",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/simplified/free",
      availability: "current",
      sort_order: 10,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "natal_simplified_basic",
      label_fr: "Natal simplifie Basic",
      description_fr: "Parcours public V2 simplifie avec tier Basic.",
      tier: "basic",
      kind: "natal_simplified",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/simplified/basic",
      availability: "current",
      sort_order: 11,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "natal_simplified_premium",
      label_fr: "Natal simplifie Premium",
      description_fr: "Parcours public V2 simplifie avec tier Premium.",
      tier: "premium",
      kind: "natal_simplified",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/simplified/premium",
      availability: "current",
      sort_order: 12,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "natal_full_free",
      label_fr: "Natal full Free",
      description_fr: "Parcours public V2 full natal.",
      tier: "free",
      kind: "natal_full",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/full/free",
      availability: "current",
      sort_order: 20,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "natal_full_basic",
      label_fr: "Natal full Basic",
      description_fr: "Parcours public V2 full natal avec tier Basic.",
      tier: "basic",
      kind: "natal_full",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/full/basic",
      availability: "current",
      sort_order: 21,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "natal_full_premium",
      label_fr: "Natal full Premium",
      description_fr: "Parcours public V2 full natal avec tier Premium.",
      tier: "premium",
      kind: "natal_full",
      frame: "natal",
      subframe: "natal",
      endpoint: "/api/gateway/v2/natal/full/premium",
      availability: "current",
      sort_order: 22,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_free_daily",
      label_fr: "Horoscope daily Free",
      description_fr: "Gateway V2 quotidien compact.",
      tier: "free",
      kind: "horoscope_daily",
      frame: "horoscope",
      subframe: "daily",
      endpoint: "/api/gateway/v2/horoscope/daily/free",
      availability: "current",
      sort_order: 30,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_basic_daily_natal_3_slots",
      label_fr: "Horoscope daily Basic",
      description_fr: "Gateway V2 quotidien en trois slots.",
      tier: "basic",
      kind: "horoscope_daily",
      frame: "horoscope",
      subframe: "daily",
      endpoint: "/api/gateway/v2/horoscope/daily/basic",
      availability: "current",
      sort_order: 31,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_premium_daily_local_2h_slots",
      label_fr: "Horoscope daily Premium",
      description_fr: "Gateway V2 premium local sur 12 creneaux.",
      tier: "premium",
      kind: "horoscope_daily",
      frame: "horoscope",
      subframe: "daily",
      endpoint: "/api/gateway/v2/horoscope/daily/premium",
      availability: "current",
      sort_order: 32,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_free_next_7_days_natal",
      label_fr: "Horoscope period Free",
      description_fr: "Gateway V2 periode des 7 prochains jours.",
      tier: "free",
      kind: "horoscope_period",
      frame: "horoscope",
      subframe: "period",
      endpoint: "/api/gateway/v2/horoscope/period/free",
      availability: "current",
      sort_order: 40,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_basic_next_7_days_natal",
      label_fr: "Horoscope period Basic",
      description_fr: "Gateway V2 periode Basic.",
      tier: "basic",
      kind: "horoscope_period",
      frame: "horoscope",
      subframe: "period",
      endpoint: "/api/gateway/v2/horoscope/period/basic",
      availability: "current",
      sort_order: 41,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
    {
      service_code: "horoscope_premium_next_7_days_natal",
      label_fr: "Horoscope period Premium",
      description_fr: "Gateway V2 periode Premium.",
      tier: "premium",
      kind: "horoscope_period",
      frame: "horoscope",
      subframe: "period",
      endpoint: "/api/gateway/v2/horoscope/period/premium",
      availability: "current",
      sort_order: 42,
      capabilities: { copyText: true, promptView: true, auditModal: true, batchRun: true },
    },
  ];

  const state = {
    location: null,
    gatewayServices: PUBLIC_SERVICES.slice().sort((a, b) => a.sort_order - b.sort_order),
    diagnosticServices: [],
    natalChartByFingerprint: null,
    results: {},
    locationAuto: true,
    locationResolveToken: 0,
    timers: {},
    isBatchRunning: false,
    modes: {
      natal: MODES.complete,
      horoscope_daily: MODES.complete,
      horoscope_period: MODES.complete,
    },
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

  function isAutoLocationEnabled() {
    return state.locationAuto;
  }

  function modeForService(service) {
    if (!service) return MODES.complete;
    if (service.kind === "horoscope_daily") return state.modes.horoscope_daily;
    if (service.kind === "horoscope_period") return state.modes.horoscope_period;
    return state.modes.natal;
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
      autoLocation: state.locationAuto,
    };
  }

  function apiKeyForPath(path) {
    if (path.startsWith("/api/calculator/")) return valueOf("calculatorApiKey") || valueOf("llmApiKey");
    if (path.startsWith("/api/llm/")) return valueOf("llmApiKey");
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
    if (body.error && typeof body.error === "object") return body.error.message || body.error.code || "";
    if (typeof body.message === "string") return body.message;
    return "";
  }

  function isHoroscopeService(service) {
    return service.kind === "horoscope_daily" || service.kind === "horoscope_period";
  }

  function serviceNeedsLocation() {
    return true;
  }

  function serviceNeedsBirthTime(service) {
    const mode = modeForService(service);
    if (service.kind === "natal_simplified") return false;
    if (service.kind === "natal_full") return mode === MODES.complete;
    if (isHoroscopeService(service)) return mode === MODES.complete;
    return false;
  }

  function serviceCanRun(service, input) {
    if (!input.birthDate) return "Date de naissance requise.";
    if (serviceNeedsLocation(service) && !input.location) {
      return input.autoLocation ? "Localisation automatique en attente ou echouee." : "Resoudre le lieu avant execution.";
    }
    if (!GENERAL_AUDIENCES.has(input.audience)) return "Audience invalide.";

    if (service.kind === "natal_full" && !input.birthTime && modeForService(service) === MODES.complete) {
      return "Heure de naissance requise en mode complet.";
    }

    if (isHoroscopeService(service) && !input.birthTime) {
      if (modeForService(service) === MODES.degraded) {
        return "Mode degrade visible, mais le backend horoscope exige encore l'heure.";
      }
      return "Heure de naissance requise pour ce service.";
    }

    if (serviceNeedsBirthTime(service) && !input.birthTime) return "Heure de naissance requise pour ce service.";
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
    if (!chartCalculationId) throw new Error("Le calcul natal ne contient pas chart_calculation_id.");
    state.natalChartByFingerprint = { fingerprint, chartCalculationId };
    return chartCalculationId;
  }

  async function resolveLocation(options) {
    const silent = Boolean(options && options.silent);
    const input = readForm();
    const result = document.getElementById("locationResult");
    const token = Date.now();
    state.locationResolveToken = token;
    if (!input.city || !input.country) {
      state.location = null;
      if (result) result.textContent = "Ville et pays requis.";
      renderServices();
      return null;
    }
    if (result && !silent) result.textContent = "Resolution du lieu...";
    const params = new URLSearchParams({ city: input.city, country: input.country });
    const data = await apiFetch(`/api/geocode?${params.toString()}`, { method: "GET" });
    if (state.locationResolveToken !== token) return state.location;
    state.location = {
      latitude: Number(data.latitude),
      longitude: Number(data.longitude),
      label: data.label,
      countryCode: data.country_code || null,
    };
    state.natalChartByFingerprint = null;
    if (result) result.textContent = `${state.location.label} - lat ${state.location.latitude}, lon ${state.location.longitude}`;
    renderServices();
    return state.location;
  }

  function scheduleAutoResolve() {
    if (!state.locationAuto) return;
    window.clearTimeout(state.autoResolveHandle);
    state.autoResolveHandle = window.setTimeout(() => {
      resolveLocation({ silent: false }).catch((err) => {
        const result = document.getElementById("locationResult");
        if (result) result.textContent = err.message;
        renderServices();
      });
    }, 250);
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

  function buildServiceGroups(services) {
    const frames = [
      { key: "natal", ...FRAME_DEFINITIONS.natal, subframes: [] },
      { key: "horoscope", ...FRAME_DEFINITIONS.horoscope, subframes: [] },
      { key: "other", ...FRAME_DEFINITIONS.other, subframes: [] },
    ];
    const map = {};
    frames.forEach((frame) => { map[frame.key] = frame; });

    services.forEach((service) => {
      const frame = map[service.frame];
      if (!frame) return;
      let subframe = frame.subframes.find((item) => item.key === service.subframe);
      if (!subframe) {
        subframe = {
          key: service.subframe,
          title: service.subframe === "natal" ? "Tous services natals" : titleCase(service.subframe),
          description: SUBFRAME_DESCRIPTIONS[service.subframe] || "",
          services: [],
        };
        frame.subframes.push(subframe);
      }
      subframe.services.push(service);
    });

    frames.forEach((frame) => {
      frame.subframes.forEach((subframe) => {
        subframe.services.sort((a, b) => a.sort_order - b.sort_order);
      });
    });
    return frames;
  }

  function renderServices() {
    const container = document.getElementById("serviceSections");
    const count = document.getElementById("serviceCount");
    const frameTemplate = document.getElementById("serviceFrameTemplate");
    const subframeTemplate = document.getElementById("serviceSubframeTemplate");
    const cardTemplate = document.getElementById("serviceCardTemplate");
    if (!container || !frameTemplate || !subframeTemplate || !cardTemplate) return;
    container.textContent = "";
    if (count) count.textContent = `${state.gatewayServices.length} service(s) publics V2`;
    const groups = buildServiceGroups(state.gatewayServices);

    groups.forEach((frame) => {
      if (frame.placeholder) {
        container.appendChild(renderPlaceholderFrame(frame));
        return;
      }

      const frameNode = frameTemplate.content.firstElementChild.cloneNode(true);
      frameNode.dataset.frameKey = frame.key;
      frameNode.querySelector(".frame-title").textContent = frame.title;
      frameNode.querySelector(".frame-description").textContent = frame.description;
      const modeSwitch = frameNode.querySelector(".mode-switch");
      const modeLabel = frameNode.querySelector(".mode-label");
      if (modeSwitch && modeLabel) {
        const frameMode = frame.key === "natal"
          ? state.modes.natal
          : frame.key === "horoscope"
            ? combineHoroscopeMode()
            : MODES.complete;
        modeSwitch.checked = frameMode === MODES.degraded;
        modeLabel.textContent = frameMode === MODES.degraded ? "Mode degrade" : "Mode complet";
        modeSwitch.addEventListener("change", () => {
          const mode = modeSwitch.checked ? MODES.degraded : MODES.complete;
          if (frame.key === "natal") state.modes.natal = mode;
          if (frame.key === "horoscope") {
            state.modes.horoscope_daily = mode;
            state.modes.horoscope_period = mode;
          }
          renderServices();
        });
      }

      const runGroup = frameNode.querySelector(".run-group");
      if (runGroup) {
        if (frame.supportsBatch) {
          runGroup.disabled = state.isBatchRunning;
          runGroup.textContent = state.isBatchRunning ? "Batch en cours..." : "Tout executer";
          runGroup.addEventListener("click", () => runBatch(frame.subframes.flatMap((sub) => sub.services)));
        } else {
          runGroup.hidden = true;
        }
      }

      const subframesHost = frameNode.querySelector(".frame-subsections");
      frame.subframes.forEach((subframe) => {
        const subNode = subframeTemplate.content.firstElementChild.cloneNode(true);
        subNode.dataset.subframeKey = subframe.key;
        subNode.querySelector(".subframe-title").textContent = subframe.title;
        subNode.querySelector(".subframe-description").textContent = subframe.description;
        const subgroupButton = subNode.querySelector(".run-subgroup");
        const showSubgroupBatch = !(frame.key === "natal" && subframe.key === "natal");
        if (showSubgroupBatch) {
          subgroupButton.disabled = state.isBatchRunning;
          subgroupButton.textContent = state.isBatchRunning ? "Batch en cours..." : "Tout executer";
          subgroupButton.addEventListener("click", () => runBatch(subframe.services));
        } else {
          subgroupButton.hidden = true;
        }
        const grid = subNode.querySelector(".service-grid");

        subframe.services.forEach((service) => {
          const input = readForm();
          const blockedReason = serviceCanRun(service, input);
          const card = cardTemplate.content.firstElementChild.cloneNode(true);
          card.dataset.serviceCode = service.service_code;
          card.querySelector("h3").textContent = service.label_fr;
          card.querySelector(".service-description").textContent = service.description_fr;
          const availability = card.querySelector(".availability");
          availability.textContent = service.availability;
          if (service.availability === "beta") availability.classList.add("beta");
          card.querySelector(".service-meta").innerHTML = buildMeta(service).map((item) => `<span class="meta-pill">${escapeHtml(item)}</span>`).join("");
          attachCardControls(card, service, blockedReason);
          renderCardState(card, service, blockedReason);
          grid.appendChild(card);
        });

        subframesHost.appendChild(subNode);
      });

      container.appendChild(frameNode);
    });
  }

  function renderPlaceholderFrame(frame) {
    const section = document.createElement("section");
    section.className = "frame placeholder-frame";
    section.innerHTML = [
      `<div class="frame-header">`,
      `<div><h2>${escapeHtml(frame.title)}</h2><p>${escapeHtml(frame.description)}</p></div>`,
      `<span class="placeholder-badge">A venir</span>`,
      `</div>`,
      `<div class="placeholder-body">`,
      `<p class="placeholder-copy">Le cadre existe pour preparer l'extension de l'interface, sans service branche pour l'instant.</p>`,
      `</div>`,
    ].join("");
    return section;
  }

  function combineHoroscopeMode() {
    return state.modes.horoscope_daily === MODES.degraded && state.modes.horoscope_period === MODES.degraded
      ? MODES.degraded
      : MODES.complete;
  }

  function buildMeta(service) {
    const relatedAsync = state.diagnosticServices.find((item) => item.api_surface && item.api_surface.recommended_entrypoint === service.endpoint.replace("/api/gateway", ""));
    return [
      service.service_code,
      service.endpoint.replace("/api/gateway", ""),
      service.kind,
      `tier:${service.tier}`,
      `mode:${modeForService(service)}`,
      relatedAsync ? `async:${relatedAsync.service_code}` : "async:unmapped",
    ];
  }

  function attachCardControls(card, service) {
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
      const result = state.results[service.service_code];
      try {
        await navigator.clipboard.writeText(result && result.jsonText ? result.jsonText : "");
        flashButtonLabel(button, "Copie");
      } catch (err) {
        flashButtonLabel(button, "Copie impossible");
      }
    });

    card.querySelector(".copy-text").addEventListener("click", async () => {
      const button = card.querySelector(".copy-text");
      const payload = composeCopiedReading(service.label_fr, getReadingTextForService(service.service_code));
      try {
        await navigator.clipboard.writeText(payload);
        flashButtonLabel(button, "Texte copie");
      } catch (err) {
        flashButtonLabel(button, "Copie impossible");
      }
    });

    card.querySelector(".show-prompt").addEventListener("click", () => openPromptModal(service));
    card.querySelector(".show-usage").addEventListener("click", () => openUsageModal(service));
    card.querySelector(".run-service").addEventListener("click", () => runService(service));
  }

  function renderCardState(card, service, blockedReason) {
    const button = card.querySelector(".run-service");
    const note = card.querySelector(".service-note");
    const result = state.results[service.service_code];
    const isRunning = result && result.status === "running";
    button.disabled = state.isBatchRunning || isRunning || Boolean(blockedReason);
    button.textContent = isRunning ? "Execution..." : "Executer";
    if (note) note.textContent = blockedReason || "";
    card.querySelector(".copy-text").disabled = !result || !result.readingText;
    card.querySelector(".show-prompt").disabled = !result;
    card.querySelector(".show-usage").disabled = !result;

    renderProgress(card, service, result);
    renderResult(card, service, result);
  }

  function renderProgress(card, service, result) {
    const status = card.querySelector(".progress-status");
    const timer = card.querySelector(".progress-timer");
    const steps = card.querySelector(".progress-steps");
    const mode = modeForService(service);
    const defaultCopy = service.kind === "horoscope_daily" || service.kind === "horoscope_period"
      ? mode === MODES.degraded
        ? "Mode degrade visible, backend encore contraint."
        : "Execution unitaire ou batch en attente."
      : "Execution unitaire ou batch en attente.";
    status.textContent = result ? result.progressLabel : "En attente";
    timer.textContent = formatMs(result ? result.elapsedMs : 0);
    const entries = result && result.steps.length ? result.steps : [{ label: defaultCopy, state: "idle" }];
    steps.innerHTML = entries.map((step) => {
      const classes = [
        step.state === "active" ? "is-active" : "",
        step.state === "error" ? "is-error" : "",
      ].filter(Boolean).join(" ");
      return `<li class="${classes}">${escapeHtml(step.label)}${step.meta ? ` - ${escapeHtml(step.meta)}` : ""}</li>`;
    }).join("");
  }

  function renderResult(card, service, result) {
    const region = card.querySelector(".result-region");
    const reading = card.querySelector(".reading-view");
    const json = card.querySelector(".json-view");
    const tech = card.querySelector(".tech-details");
    if (!result) {
      region.hidden = true;
      reading.innerHTML = "";
      json.textContent = "";
      tech.textContent = "";
      tech.hidden = true;
      return;
    }

    region.hidden = false;
    json.textContent = result.jsonText || "";
    tech.innerHTML = buildTechDetailsHtml(result, service);
    tech.hidden = true;

    if (result.errorMessage) {
      reading.innerHTML = `<div class="reading-block"><h4 class="error-text">Erreur</h4><p>${escapeHtml(result.errorMessage)}</p></div>`;
      return;
    }

    if (!result.sections || !result.sections.length) {
      reading.innerHTML = `<div class="reading-block"><h4>Aucun rendu simplifie disponible</h4><p>Utiliser l'onglet JSON brut pour inspecter la reponse.</p></div>`;
      return;
    }

    reading.innerHTML = result.sections.map(renderSection).join("");
  }

  function buildTechDetailsHtml(result, service) {
    const lines = [
      `Endpoint: ${escapeHtml(service.endpoint.replace("/api/gateway", ""))}`,
      `Service: ${escapeHtml(service.service_code)}`,
      `Mode UI: ${escapeHtml(modeForService(service))}`,
      `Duree UI: ${escapeHtml(formatMs(result.elapsedMs))}`,
      `Produit: ${escapeHtml(result.productCode || "")}`,
      `Variant: ${escapeHtml(result.variant || "")}`,
    ];
    if (result.runId) lines.push(`Run audit: ${escapeHtml(result.runId)}`);
    if (result.audit && result.audit.status) lines.push(`Audit status: ${escapeHtml(result.audit.status)}`);
    lines.push(promptAvailabilityLabel(result));
    return lines.join("<br>");
  }

  function promptAvailabilityLabel(result) {
    const promptPayload = result && result.promptPayload ? result.promptPayload : null;
    const count = promptPayload && Array.isArray(promptPayload.traces) ? promptPayload.traces.length : 0;
    if (count) return `Prompt: disponible (${count} trace(s))`;
    if (promptPayload && promptPayload.kind === "unavailable") return "Prompt: audit indisponible";
    if (result && result.audit && result.audit.error) return "Prompt: audit indisponible";
    return "Prompt: non expose par le backend";
  }

  async function runBatch(services) {
    if (state.isBatchRunning) return;
    state.isBatchRunning = true;
    renderServices();
    try {
      for (const service of services.slice().sort((a, b) => a.sort_order - b.sort_order)) {
        const input = readForm();
        const blockedReason = serviceCanRun(service, input);
        if (blockedReason) {
          state.results[service.service_code] = buildErrorResult(service.service_code, blockedReason);
          renderServices();
          continue;
        }
        await runService(service, { batch: true });
      }
    } finally {
      state.isBatchRunning = false;
      renderServices();
    }
  }

  async function runService(service, options) {
    const input = readForm();
    const blockedReason = serviceCanRun(service, input);
    if (blockedReason) {
      state.results[service.service_code] = buildErrorResult(service.service_code, blockedReason);
      renderServices();
      return;
    }

    const started = Date.now();
    const result = {
      serviceCode: service.service_code,
      status: "running",
      startedAt: started,
      elapsedMs: 0,
      progressLabel: "Preparation",
      steps: [],
      response: null,
      jsonText: "",
      sections: [],
      readingText: "",
      productCode: "",
      variant: "",
      runId: null,
      audit: null,
      promptPayload: null,
      promptAvailable: null,
      errorMessage: "",
    };
    state.results[service.service_code] = result;
    startTimer(service.service_code);
    addProgressStep(service.service_code, "Preparation de la requete", "active");
    renderServices();

    try {
      let requestBody;
      if (service.kind === "natal_simplified" || service.kind === "natal_full") {
        requestBody = buildGatewayNatalRequest(input);
      } else {
        addProgressStep(service.service_code, "Calcul natal de reference", "active");
        const chartCalculationId = await ensureNatalChartCalculationId(input);
        markLatestStep(service.service_code, "done", `chart:${chartCalculationId}`);
        if (service.kind === "horoscope_daily") {
          requestBody = buildGatewayHoroscopeDailyRequest(input, chartCalculationId, service);
        } else {
          requestBody = buildGatewayHoroscopePeriodRequest(input, chartCalculationId);
        }
      }

      markLatestStep(service.service_code, "done");
      addProgressStep(service.service_code, `Appel ${service.endpoint.replace("/api/gateway", "")}`, "active");
      const response = await apiFetch(service.endpoint, {
        method: "POST",
        body: JSON.stringify(requestBody),
      });
      markLatestStep(service.service_code, "done");
      addProgressStep(service.service_code, "Traitement de la reponse", "active");
      await finalizeServiceResult(service, response, Date.now() - started);
      markLatestStep(service.service_code, "done");
      addProgressStep(service.service_code, "Rendu UI termine", "done");
    } catch (err) {
      markLatestStep(service.service_code, "error", err.message);
      state.results[service.service_code] = buildErrorResult(service.service_code, err.message, err.body || null, Date.now() - started, state.results[service.service_code].steps);
    } finally {
      stopTimer(service.service_code);
      const next = state.results[service.service_code];
      if (next) {
        next.status = next.errorMessage ? "failed" : "completed";
        next.progressLabel = next.errorMessage ? "Echec" : "Termine";
      }
      renderServices();
      if (!options || !options.batch) {
        return;
      }
    }
  }

  async function finalizeServiceResult(service, response, elapsedMs) {
    const normalized = normalizeGatewayReadingSections(response);
    const runId = extractRunId(response);
    const audit = runId ? await fetchRunAudit(runId) : null;
    const promptPayload = extractPromptPayload(response, audit);
    const readingText = joinSectionsForCopy(normalized);
    const current = state.results[service.service_code];
    state.results[service.service_code] = {
      ...current,
      status: "completed",
      elapsedMs,
      progressLabel: audit ? "Reponse + audit charges" : "Reponse chargee",
      response,
      jsonText: JSON.stringify(response, null, 2),
      sections: normalized,
      readingText,
      productCode: readField(response, ["metadata", "product_code"]) || "",
      variant: readField(response, ["metadata", "variant"]) || "",
      runId,
      audit,
      promptPayload,
      promptAvailable: Boolean(promptPayload && Array.isArray(promptPayload.traces) && promptPayload.traces.length),
      errorMessage: "",
      steps: mergeUiAndAuditSteps(current.steps, audit),
    };
  }

  async function fetchRunAudit(runId) {
    try {
      return await apiFetch(`/api/llm/v1/runs/${encodeURIComponent(runId)}`, { method: "GET" });
    } catch (err) {
      return { error: err.message, steps: [] };
    }
  }

  function extractRunId(payload) {
    return firstStringAtPaths(payload, [
      ["run_id"],
      ["metadata", "run_id"],
      ["reading", "run_id"],
      ["result", "run_id"],
    ]);
  }

  function extractPromptPayload(response, audit) {
    const traces = normalizePromptTraceList(audit);
    if (traces.length) {
      return {
        kind: "prompt_traces",
        traces,
        source: "audit",
        auditStatus: audit && audit.status ? audit.status : null,
        auditError: audit && audit.error ? String(audit.error) : "",
      };
    }
    const direct = firstValueAtPaths(response, [
      ["prompt"],
      ["prompt_text"],
      ["prompt_trace"],
      ["metadata", "prompt"],
      ["metadata", "prompt_trace"],
      ["debug", "prompt"],
    ]);
    if (direct !== null && direct !== undefined && direct !== "") {
      const legacyTraces = normalizeLegacyPromptPayload(direct);
      return {
        kind: "legacy_prompt",
        traces: legacyTraces.length ? legacyTraces : [normalizeLegacyPromptTrace(direct)],
        source: "response",
        auditStatus: audit && audit.status ? audit.status : null,
        auditError: audit && audit.error ? String(audit.error) : "",
      };
    }
    if (audit && audit.prompt) {
      const legacyTraces = normalizeLegacyPromptPayload(audit.prompt);
      return {
        kind: "legacy_prompt",
        traces: legacyTraces.length ? legacyTraces : [normalizeLegacyPromptTrace(audit.prompt)],
        source: "audit_prompt",
        auditStatus: audit && audit.status ? audit.status : null,
        auditError: audit && audit.error ? String(audit.error) : "",
      };
    }
    if (audit && audit.error) {
      return {
        kind: "unavailable",
        traces: [],
        source: "audit_error",
        auditStatus: audit.status || null,
        auditError: String(audit.error),
      };
    }
    return null;
  }

  function normalizePromptTraceList(audit) {
    if (!audit || !Array.isArray(audit.prompt_traces) || !audit.prompt_traces.length) return [];
    return audit.prompt_traces
      .map((trace, index) => normalizePromptTrace(trace, index))
      .filter((trace) => trace && (hasPromptText(trace.compiledPrompt) || trace.messagesJson !== null));
  }

  function normalizePromptTrace(trace, index) {
    if (!trace || typeof trace !== "object") return null;
    return {
      index,
      kind: "prompt_trace",
      chapterCode: nonEmptyString(trace.chapter_code),
      stepType: nonEmptyString(trace.step_type),
      attempt: nonEmptyString(trace.attempt),
      promptFamily: nonEmptyString(trace.prompt_family),
      promptVersion: nonEmptyString(trace.prompt_version),
      messageCount: typeof trace.message_count === "number" ? trace.message_count : null,
      compiledPrompt: typeof trace.compiled_prompt === "string" ? trace.compiled_prompt : "",
      messagesJson: trace.messages_json === undefined ? null : trace.messages_json,
      createdAt: nonEmptyString(trace.created_at),
      source: "audit",
    };
  }

  function normalizeLegacyPromptTrace(value) {
    return {
      index: 0,
      kind: "legacy_prompt",
      chapterCode: "",
      stepType: "legacy_prompt",
      attempt: "",
      promptFamily: "",
      promptVersion: "",
      messageCount: null,
      compiledPrompt: typeof value === "string" ? value : "",
      messagesJson: typeof value === "string" ? null : value,
      createdAt: "",
      source: "legacy",
    };
  }

  function normalizeLegacyPromptPayload(value) {
    if (Array.isArray(value)) {
      return value
        .map((item, index) => normalizeLegacyPromptObject(item, index))
        .filter(Boolean);
    }
    const single = normalizeLegacyPromptObject(value, 0);
    return single ? [single] : [];
  }

  function normalizeLegacyPromptObject(value, index) {
    if (!value || typeof value !== "object" || Array.isArray(value)) return null;
    if (!Object.prototype.hasOwnProperty.call(value, "compiled_prompt")
      && !Object.prototype.hasOwnProperty.call(value, "messages_json")
      && !Object.prototype.hasOwnProperty.call(value, "step_type")
      && !Object.prototype.hasOwnProperty.call(value, "attempt")) {
      return null;
    }
    const normalized = normalizePromptTrace(value, index);
    if (!normalized) return null;
    return {
      ...normalized,
      source: "legacy",
    };
  }

  function mergeUiAndAuditSteps(existingSteps, audit) {
    const uiSteps = (existingSteps || []).map((step) => ({ ...step }));
    if (!audit || !Array.isArray(audit.steps) || !audit.steps.length) return uiSteps;
    const auditSteps = audit.steps.map((step) => ({
      label: step.step_type || "backend_step",
      state: step.status === "failed" ? "error" : "done",
      meta: compactAuditMeta(step),
    }));
    return uiSteps.concat(auditSteps);
  }

  function compactAuditMeta(step) {
    const parts = [];
    if (step.chapter_code) parts.push(step.chapter_code);
    if (step.provider || step.model) parts.push([step.provider, step.model].filter(Boolean).join("/"));
    if (typeof step.latency_ms === "number") parts.push(formatMs(step.latency_ms));
    const tokenLabel = compactTokenLabel(step && step.token_usage, step && step.input_tokens, step && step.output_tokens);
    if (tokenLabel) {
      parts.push(tokenLabel);
    }
    return parts.join(" | ");
  }

  function compactTokenLabel(tokenUsage, legacyInput, legacyOutput) {
    if (tokenUsage && tokenUsage.summary) {
      const summary = tokenUsage.summary;
      return [
        `in ${summary.input_tokens || 0}`,
        `out ${summary.output_tokens || 0}`,
        `cache ${summary.cache_tokens || 0}`,
        `reason ${summary.reasoning_tokens || 0}`,
      ].join(" / ");
    }
    if (typeof legacyInput === "number" || typeof legacyOutput === "number") {
      return `${legacyInput || 0}/${legacyOutput || 0} tok`;
    }
    return "";
  }

  function buildErrorResult(serviceCode, message, body, elapsedMs, previousSteps) {
    return {
      serviceCode,
      status: "failed",
      startedAt: Date.now(),
      elapsedMs: elapsedMs || 0,
      progressLabel: "Echec",
      steps: previousSteps || [{ label: message, state: "error" }],
      response: body || null,
      jsonText: body ? JSON.stringify(body, null, 2) : "",
      sections: [],
      readingText: "",
      productCode: "",
      variant: "",
      runId: null,
      audit: null,
      promptPayload: null,
      promptAvailable: false,
      errorMessage: message,
    };
  }

  function addProgressStep(serviceCode, label, stateValue, meta) {
    const result = state.results[serviceCode];
    if (!result) return;
    result.progressLabel = label;
    result.steps.push({ label, state: stateValue, meta: meta || "" });
  }

  function markLatestStep(serviceCode, stateValue, meta) {
    const result = state.results[serviceCode];
    if (!result || !result.steps.length) return;
    const step = result.steps[result.steps.length - 1];
    step.state = stateValue;
    if (meta) step.meta = meta;
  }

  function startTimer(serviceCode) {
    stopTimer(serviceCode);
    state.timers[serviceCode] = window.setInterval(() => {
      const result = state.results[serviceCode];
      if (!result) return;
      result.elapsedMs = Date.now() - result.startedAt;
      const card = document.querySelector(`[data-service-code="${serviceCode}"]`);
      if (card) renderProgress(card, state.gatewayServices.find((item) => item.service_code === serviceCode), result);
    }, 100);
  }

  function stopTimer(serviceCode) {
    if (!state.timers[serviceCode]) return;
    window.clearInterval(state.timers[serviceCode]);
    delete state.timers[serviceCode];
  }

  function openPromptModal(service) {
    const result = state.results[service.service_code];
    const promptPayload = result && result.promptPayload ? result.promptPayload : null;
    const traceCount = promptPayload && Array.isArray(promptPayload.traces) ? promptPayload.traces.length : 0;
    const content = traceCount
      ? renderPromptPayload(promptPayload)
      : `<div class="empty-copy">${escapeHtml(promptUnavailableMessage(result))}</div>`;
    openModal(`Prompt - ${service.label_fr}`, service.service_code, content);
  }

  function renderPromptPayload(payload) {
    if (!payload || !Array.isArray(payload.traces) || !payload.traces.length) return "";
    const intro = payload.traces.length === 1
      ? `<p class="prompt-summary">1 prompt audite pour ce run.</p>`
      : `<p class="prompt-summary">${payload.traces.length} prompts audites pour ce run.</p>`;
    const traces = payload.traces.map((trace, index) => renderPromptTrace(trace, index)).join("");
    return `<div class="prompt-trace-list">${intro}${traces}</div>`;
  }

  function renderPromptTrace(trace, index) {
    const title = trace.stepType || `prompt_${index + 1}`;
    const badges = [
      trace.chapterCode ? `chapitre ${trace.chapterCode}` : "",
      trace.attempt ? `attempt ${trace.attempt}` : "",
      trace.createdAt ? formatPromptTimestamp(trace.createdAt) : "",
    ].filter(Boolean);
    const metaLines = [
      trace.promptFamily ? `Famille: ${trace.promptFamily}` : "",
      trace.promptVersion ? `Version: ${trace.promptVersion}` : "",
      trace.messageCount !== null ? `Messages: ${trace.messageCount}` : "",
      hasPromptText(trace.compiledPrompt) ? "" : "Prompt compile indisponible",
    ].filter(Boolean);
    const compiledPrompt = hasPromptText(trace.compiledPrompt)
      ? `<div class="prompt-trace-section"><h5>Compiled prompt</h5><pre class="prompt-box">${escapeHtml(trace.compiledPrompt)}</pre></div>`
      : "";
    const messages = trace.messagesJson !== null
      ? `<div class="prompt-trace-section"><h5>Messages JSON</h5><pre class="prompt-box">${escapeHtml(stringifyPrompt(trace.messagesJson))}</pre></div>`
      : `<div class="prompt-trace-section"><h5>Messages JSON</h5><div class="empty-copy">Messages indisponibles pour cette trace.</div></div>`;
    return [
      `<section class="prompt-trace-card">`,
      `<div class="prompt-trace-header"><div><h4>${escapeHtml(title)}</h4>${badges.length ? `<div class="prompt-trace-badges">${badges.map((part) => `<span class="meta-pill">${escapeHtml(part)}</span>`).join("")}</div>` : ""}</div></div>`,
      metaLines.length ? `<div class="prompt-trace-meta">${metaLines.map(escapeHtml).join("<br>")}</div>` : "",
      compiledPrompt,
      messages,
      `</section>`,
    ].join("");
  }

  function openUsageModal(service) {
    const result = state.results[service.service_code];
    const usage = summarizeUsage(result);
    const auditTable = usage.steps.length
      ? [
        `<table class="audit-table">`,
        `<thead><tr><th>Step</th><th>Statut</th><th>Provider</th><th>Input</th><th>Output</th><th>Cache</th><th>Reasoning</th><th>Latence</th></tr></thead>`,
        `<tbody>`,
        usage.steps.map((step) => `<tr><td>${escapeHtml(step.label)}</td><td>${escapeHtml(step.status)}</td><td>${escapeHtml(step.provider)}</td><td>${escapeHtml(step.inputTokens)}</td><td>${escapeHtml(step.outputTokens)}</td><td>${escapeHtml(step.cacheTokens)}</td><td>${escapeHtml(step.reasoningTokens)}</td><td>${escapeHtml(step.latency)}</td></tr>`).join(""),
        `</tbody></table>`,
      ].join("")
      : `<div class="empty-copy">Aucun detail backend disponible. Les couts restent indisponibles tant que le service de comptage n'est pas expose.</div>`;
    const content = [
      `<div class="usage-grid">`,
      `<article class="usage-card"><span>Input tokens</span><strong>${escapeHtml(String(usage.inputTokens))}</strong></article>`,
      `<article class="usage-card"><span>Output tokens</span><strong>${escapeHtml(String(usage.outputTokens))}</strong></article>`,
      `<article class="usage-card"><span>Cache tokens</span><strong>${escapeHtml(String(usage.cacheTokens))}</strong></article>`,
      `<article class="usage-card"><span>Reasoning tokens</span><strong>${escapeHtml(String(usage.reasoningTokens))}</strong></article>`,
      `<article class="usage-card"><span>Cout estime</span><strong>${escapeHtml(usage.costLabel)}</strong></article>`,
      `</div>`,
      auditTable,
    ].join("");
    openModal(`Tokens / couts - ${service.label_fr}`, service.service_code, content);
  }

  function summarizeUsage(result) {
    const audit = result && result.audit && !result.audit.error ? result.audit : null;
    const rootUsage = audit && audit.token_usage ? audit.token_usage : null;
    const steps = audit && Array.isArray(audit.steps) ? audit.steps.map((step) => ({
      label: step.step_type || "-",
      status: step.status || "-",
      provider: [step.provider, step.model].filter(Boolean).join("/") || "-",
      inputTokens: readStepTokenSummary(step, "input_tokens", step.input_tokens),
      outputTokens: readStepTokenSummary(step, "output_tokens", step.output_tokens),
      cacheTokens: readStepTokenSummary(step, "cache_tokens", null),
      reasoningTokens: readStepTokenSummary(step, "reasoning_tokens", null),
      latency: typeof step.latency_ms === "number" ? formatMs(step.latency_ms) : "-",
    })) : [];
    return {
      inputTokens: readRootTokenSummary(rootUsage, audit, "input_tokens", "token_input"),
      outputTokens: readRootTokenSummary(rootUsage, audit, "output_tokens", "token_output"),
      cacheTokens: rootUsage && rootUsage.summary ? valueOrUnavailable(rootUsage.summary.cache_tokens) : "indisponible",
      reasoningTokens: rootUsage && rootUsage.summary ? valueOrUnavailable(rootUsage.summary.reasoning_tokens) : "indisponible",
      costLabel: rootUsage && rootUsage.cost && rootUsage.cost.estimated_total !== undefined && rootUsage.cost.estimated_total !== null
        ? `${Number(rootUsage.cost.estimated_total).toFixed(6)} ${rootUsage.cost.currency || "USD"}`
        : "indisponible",
      steps,
    };
  }

  function readStepTokenSummary(step, key, legacyValue) {
    const summaryValue = readTokenSummary(step && step.token_usage, key, legacyValue);
    if (summaryValue !== "indisponible") return summaryValue;
    if (isLocalAuditStep(step)) return "step local (sans tokens provider)";
    return summaryValue;
  }

  function readTokenSummary(tokenUsage, key, legacyValue) {
    if (tokenUsage && tokenUsage.summary && tokenUsage.summary[key] !== undefined && tokenUsage.summary[key] !== null) {
      return String(tokenUsage.summary[key]);
    }
    if (legacyValue !== undefined && legacyValue !== null) return String(legacyValue);
    return "indisponible";
  }

  function readRootTokenSummary(rootUsage, audit, usageKey, legacyKey) {
    if (rootUsage && rootUsage.summary && rootUsage.summary[usageKey] !== undefined && rootUsage.summary[usageKey] !== null) {
      return String(rootUsage.summary[usageKey]);
    }
    if (audit && audit[legacyKey] !== undefined && audit[legacyKey] !== null) {
      return String(audit[legacyKey]);
    }
    return "indisponible";
  }

  function valueOrUnavailable(value) {
    return value === undefined || value === null ? "indisponible" : String(value);
  }

  function isLocalAuditStep(step) {
    if (!step || typeof step !== "object") return false;
    const hasDetailedUsage = Boolean(step.token_usage && step.token_usage.summary);
    const hasLegacyUsage = step.input_tokens !== undefined && step.input_tokens !== null
      || step.output_tokens !== undefined && step.output_tokens !== null;
    if (hasDetailedUsage || hasLegacyUsage) return false;
    return String(step.status || "").toLowerCase() === "repaired";
  }

  function openModal(title, subtitle, content) {
    const root = document.getElementById("modalRoot");
    const modalTitle = document.getElementById("modalTitle");
    const modalSubtitle = document.getElementById("modalSubtitle");
    const modalContent = document.getElementById("modalContent");
    if (!root || !modalTitle || !modalSubtitle || !modalContent) return;
    modalTitle.textContent = title;
    modalSubtitle.textContent = subtitle || "";
    modalContent.innerHTML = content;
    root.hidden = false;
  }

  function closeModal() {
    const root = document.getElementById("modalRoot");
    if (root) root.hidden = true;
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
    addObjectTextSection(sections, "Reperes", payload.watch_summary);

    addArraySections(sections, "Creneau", payload.slots, ["text", "advice", "watch_point"]);
    addArraySections(sections, "Meilleur moment", payload.best_slots, ["reason"]);
    addArraySections(sections, "Moment de vigilance", payload.watch_slots, ["reason"]);
    addArraySections(sections, "Timeline", payload.timeline, ["text", "advice", "watch_point"]);
    addArraySections(sections, "Jour cle", payload.key_days, ["reason"]);
    addArraySections(sections, "Meilleur jour", payload.best_days, ["reason"]);
    addArraySections(sections, "Jours sensibles", payload.watch_days, ["reason"]);
    addArraySections(sections, "Timeline quotidienne", payload.daily_timeline, ["text", "advice"]);
    addArraySections(sections, "Domaine", payload.domain_sections, ["text"]);
    addArraySections(sections, "Creneaux utiles", payload.best_windows, ["reason", "advice"]);
    addArraySections(sections, "Fenetre de vigilance", payload.watch_windows, ["reason", "advice"]);
    addObjectAdvice(sections, payload.strategy, "Strategie");

    return sections.filter((section) => section.paragraphs.length > 0 || section.items);
  }

  function addStringSection(sections, title, value) {
    if (typeof value === "string" && value.trim()) sections.push({ title, paragraphs: [value] });
  }

  function addObjectTextSection(sections, title, value) {
    if (value && typeof value === "object" && value.text) sections.push({ title, paragraphs: [value.text] });
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

  function firstValueAtPaths(obj, paths) {
    for (const path of paths) {
      const value = readField(obj, path);
      if (value !== null && value !== undefined && value !== "") return value;
    }
    return null;
  }

  function firstStringAtPaths(obj, paths) {
    const value = firstValueAtPaths(obj, paths);
    return typeof value === "string" && value.trim() ? value.trim() : null;
  }

  function composeCopiedReading(serviceName, readingText) {
    const stamp = new Date().toISOString();
    return [`Service: ${serviceName}`, `Date: ${stamp}`, "", readingText || ""].join("\n");
  }

  function joinSectionsForCopy(sections) {
    return (sections || []).map((section) => {
      const lines = [section.title];
      if (Array.isArray(section.meta) && section.meta.length) lines.push(section.meta.join(" - "));
      (section.paragraphs || []).forEach((paragraph) => lines.push(paragraph));
      return lines.join("\n");
    }).join("\n\n").trim();
  }

  function getReadingTextForService(serviceCode) {
    const result = state.results[serviceCode];
    return result && result.readingText ? result.readingText : "";
  }

  function stringifyPrompt(value) {
    return typeof value === "string" ? value : JSON.stringify(value, null, 2);
  }

  function hasPromptText(value) {
    return typeof value === "string" && value.trim().length > 0;
  }

  function nonEmptyString(value) {
    return typeof value === "string" && value.trim() ? value.trim() : "";
  }

  function formatPromptTimestamp(value) {
    if (!value) return "";
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toISOString();
  }

  function promptUnavailableMessage(result) {
    const promptPayload = result && result.promptPayload ? result.promptPayload : null;
    if (promptPayload && promptPayload.kind === "unavailable" && promptPayload.auditError) {
      return `Audit prompt indisponible: ${promptPayload.auditError}`;
    }
    if (result && result.audit && result.audit.error) {
      return `Audit prompt indisponible: ${result.audit.error}`;
    }
    if (result && result.runId && !result.audit) {
      return "Audit du run indisponible pour ce service.";
    }
    return "Prompt non expose par le backend pour ce service.";
  }

  function formatMs(value) {
    const ms = Number(value) || 0;
    return `${(ms / 1000).toFixed(1)} s`;
  }

  function titleCase(value) {
    return String(value || "").replace(/(^|\s|-)\S/g, (match) => match.toUpperCase());
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
    if (window.crypto && typeof window.crypto.randomUUID === "function") return window.crypto.randomUUID();
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
    const locationAuto = document.getElementById("locationAuto");
    const resolveButton = document.getElementById("resolveLocation");
    if (locationAuto) {
      locationAuto.checked = state.locationAuto;
      locationAuto.addEventListener("change", () => {
        state.locationAuto = locationAuto.checked;
        resolveButton.disabled = state.locationAuto;
        if (state.locationAuto) {
          scheduleAutoResolve();
        }
        renderServices();
      });
    }
    if (resolveButton) {
      resolveButton.disabled = state.locationAuto;
      resolveButton.addEventListener("click", () => {
        resolveLocation().catch((err) => {
          const result = document.getElementById("locationResult");
          if (result) result.textContent = err.message;
        });
      });
    }
    document.getElementById("reloadServices").addEventListener("click", () => {
      loadDiagnosticServices().catch((err) => {
        const count = document.getElementById("diagnosticCount");
        if (count) count.textContent = err.message;
      });
    });
    document.getElementById("closeModal").addEventListener("click", closeModal);
    document.getElementById("modalRoot").addEventListener("click", (event) => {
      const target = event.target;
      if (target && target.dataset && target.dataset.closeModal === "true") closeModal();
    });
    ["birthDate", "birthTime", "timezone", "city", "country", "targetDate", "language", "audience"].forEach((id) => {
      const element = document.getElementById(id);
      if (!element) return;
      element.addEventListener("change", () => {
        if (["birthDate", "birthTime", "timezone"].includes(id)) state.natalChartByFingerprint = null;
        if (["city", "country"].includes(id)) {
          state.location = null;
          state.natalChartByFingerprint = null;
          const result = document.getElementById("locationResult");
          if (result) result.textContent = state.locationAuto ? "Lieu modifie, resolution automatique..." : "Lieu modifie, resolution a relancer.";
          scheduleAutoResolve();
        }
        renderServices();
      });
      element.addEventListener("input", () => {
        if (["city", "country"].includes(id)) {
          state.location = null;
          const result = document.getElementById("locationResult");
          if (result) result.textContent = state.locationAuto ? "Lieu modifie, resolution automatique..." : "Lieu modifie, resolution a relancer.";
          scheduleAutoResolve();
          renderServices();
        }
      });
    });
    renderServices();
    loadHealth().catch(() => {});
    loadDiagnosticServices().catch(() => {});
    if (state.locationAuto) scheduleAutoResolve();
  }

  window.AstralServiceTestUi = {
    PUBLIC_SERVICES,
    MODES,
    buildGatewayNatalRequest,
    buildGatewayHoroscopeDailyRequest,
    buildGatewayHoroscopePeriodRequest,
    buildCalculatorNatalPayload,
    buildServiceGroups,
    composeCopiedReading,
    combineHoroscopeMode,
    extractRunId,
    extractPromptPayload,
    isAutoLocationEnabled,
    isHoroscopeService,
    joinSectionsForCopy,
    normalizePromptTraceList,
    normalizeGatewayAudience,
    normalizeHoroscopeAudience,
    normalizeReadingSections,
    normalizeGatewayReadingSections,
    parseResponseBody,
    serviceCanRun,
    supportedTargetLanguageCode,
    summarizeUsage,
    newClientId,
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      if (document.getElementById("serviceSections")) boot();
    });
  } else if (document.getElementById("serviceSections")) {
    boot();
  }
}());
