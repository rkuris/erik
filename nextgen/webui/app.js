const state = {
    token: null,
    currentView: "status",
    statusCache: null,
    defaultsCache: null,
    probesCache: null,
};

const selectors = {
    loginScreen: document.getElementById("login-screen"),
    loginForm: document.getElementById("login-form"),
    loginError: document.getElementById("login-error"),
    appShell: document.getElementById("app-shell"),
    viewTitle: document.getElementById("view-title"),
    wifiPill: document.getElementById("wifi-pill"),
    statusDot: document.querySelector(".brand .status-dot"),
    navButtons: Array.from(document.querySelectorAll("aside nav button")),
    views: Array.from(document.querySelectorAll("main .view")),
    temperatureTable: document.querySelector("#temperature-table tbody"),
    relayState: document.getElementById("relay-state"),
    relayButtons: Array.from(document.querySelectorAll("[data-relay]")),
    scanNetworks: document.getElementById("scan-networks"),
    wifiResults: document.getElementById("wifi-scan-results"),
    wifiForm: document.getElementById("wifi-form"),
    wifiMessage: document.getElementById("wifi-message"),
    defaultsForm: document.getElementById("defaults-form"),
    defaultsMessage: document.getElementById("defaults-message"),
    probeList: document.getElementById("probe-list"),
    refreshStatus: document.getElementById("refresh-status"),
    reboot: document.getElementById("reboot"),
    factoryReset: document.getElementById("factory-reset"),
    adminMessage: document.getElementById("admin-message"),
    passwordForm: document.getElementById("password-form"),
    passwordMessage: document.getElementById("password-message"),
    logout: document.getElementById("logout"),
};

const STORAGE_KEYS = {
    token: "solar-heater-token",
    mock: "solar-heater-mock",
};

const ENDPOINTS = {
    login: "/api/login",
    logout: "/api/logout",
    status: "/api/status",
    relay: "/api/relay",
    wifiScan: "/api/wifi/scan",
    wifiSave: "/api/wifi",
    defaults: "/api/defaults",
    probes: "/api/probes",
    reboot: "/api/admin/reboot",
    factoryReset: "/api/admin/factory-reset",
    passwordChange: "/api/admin/password",
};

async function apiRequest(endpoint, options = {}) {
    const headers = options.headers ?? {};
    if (state.token) {
        headers["Authorization"] = `Bearer ${state.token}`;
    }

    const response = await fetch(endpoint, {
        ...options,
        headers: {
            "Content-Type": "application/json",
            ...headers,
        },
    });

    if (response.status === 401) {
        handleUnauthorized();
        throw new Error("unauthorized");
    }
    if (!response.ok) {
        const text = await response.text();
        throw new Error(text || response.statusText);
    }

    if (response.status === 204) {
        return null;
    }

    const contentType = response.headers.get("content-type") || "";
    if (contentType.includes("application/json")) {
        return response.json();
    }
    return response.text();
}

function getStoredValue(key) {
    try {
        return window.localStorage.getItem(key);
    } catch (err) {
        console.warn("Unable to read localStorage", err);
        return null;
    }
}

function setStoredValue(key, value) {
    try {
        window.localStorage.setItem(key, value);
    } catch (err) {
        console.warn("Unable to write localStorage", err);
    }
}

function removeStoredValue(key) {
    try {
        window.localStorage.removeItem(key);
    } catch (err) {
        console.warn("Unable to remove localStorage item", err);
    }
}

// Mock helpers to keep the UI usable before firmware endpoints exist
const MockApi = (() => {
    const mockToken = "mock-token";
    let mockPassword = "admin";
    const sampleStatus = () => ({
        wifi: {
            mode: "AP",
            ssid: "Solar-Heater",
            connected: false,
            rssi: null,
            ip: "192.168.4.1",
        },
        relay: {
            state: Math.random() > 0.5 ? "on" : "off",
            lastChange: new Date().toISOString(),
        },
        probes: [
            {
                id: "28-00000abcd123",
                name: "Pool Return",
                fahrenheit: +(70 + Math.random() * 5).toFixed(1),
                lastUpdated: new Date().toISOString(),
                enabled: true,
            },
            {
                id: "28-00000abcd456",
                name: "Roof",
                fahrenheit: +(100 + Math.random() * 5).toFixed(1),
                lastUpdated: new Date().toISOString(),
                enabled: true,
            },
        ],
        uptimeSeconds: Math.floor(performance.now() / 1000),
    });

    let defaults = {
        default_state: "off",
        hysteresis: 2,
        min_on_temp: 70,
    };

    return {
        async login({ username, password }) {
            if (username === "admin" && password === mockPassword) {
                return { token: mockToken };
            }
            throw new Error("Invalid credentials");
        },
        async logout() {
            return {};
        },
        async status() {
            return sampleStatus();
        },
        async relay(body) {
            return { state: body.state };
        },
        async wifiScan() {
            return {
                networks: [
                    { ssid: "Backyard", rssi: -55, secure: true },
                    { ssid: "Guest", rssi: -68, secure: false },
                ],
            };
        },
        async wifiSave(body) {
            defaults = { ...defaults, preferred_ssid: body.ssid };
            return { saved: true };
        },
        async defaultsGet() {
            return defaults;
        },
        async defaultsSave(body) {
            defaults = { ...defaults, ...body };
            return defaults;
        },
        async probes() {
            const status = await this.status();
            return status.probes;
        },
        async reboot() {
            return { rebooting: true };
        },
        async factoryReset() {
            defaults = {
                default_state: "off",
                hysteresis: 2,
                min_on_temp: 70,
            };
            mockPassword = "admin";
            return { reset: true };
        },
        async changePassword(body) {
            if (body.current_password !== mockPassword) {
                throw new Error("Current password incorrect");
            }
            mockPassword = body.new_password;
            return { changed: true };
        },
    };
})();

let useMock = window.location.protocol === "file:";
if (!useMock) {
    useMock = getStoredValue(STORAGE_KEYS.mock) === "1";
} else {
    setStoredValue(STORAGE_KEYS.mock, "1");
}

function persistPreferences() {
    setStoredValue(STORAGE_KEYS.mock, useMock ? "1" : "0");
}

function persistSession() {
    if (state.token) {
        setStoredValue(STORAGE_KEYS.token, state.token);
    } else {
        removeStoredValue(STORAGE_KEYS.token);
    }
    persistPreferences();
}

function showAppShell() {
    selectors.loginScreen.classList.add("hidden");
    selectors.appShell.classList.remove("hidden");
}

function clearSession() {
    state.token = null;
    state.statusCache = null;
    state.defaultsCache = null;
    state.probesCache = null;
    removeStoredValue(STORAGE_KEYS.token);
    persistPreferences();
    selectors.loginForm.reset();
    selectors.appShell.classList.add("hidden");
    selectors.loginScreen.classList.remove("hidden");
    selectors.wifiPill.textContent = "Wi-Fi: no data";
    setStatusIndicator("unknown");
    selectors.loginForm.username.focus();
    selectors.passwordMessage.hidden = true;
    selectors.passwordMessage.classList.remove("error");
    selectors.adminMessage.hidden = true;
}

function handleUnauthorized() {
    clearSession();
}

async function restoreSession() {
    try {
        await loadStatus();
    } catch (err) {
        console.warn("Session restore failed", err);
        clearSession();
    }
}

async function login(username, password) {
    try {
        const data = useMock
            ? await MockApi.login({ username, password })
            : await apiRequest(ENDPOINTS.login, {
                  method: "POST",
                  body: JSON.stringify({ username, password }),
              });
        state.token = data.token;
        persistSession();
        return true;
    } catch (err) {
        if (!useMock && err.message === "Failed to fetch") {
            useMock = true;
            persistPreferences();
            return login(username, password);
        }
        throw err;
    }
}

async function logout() {
    try {
        if (useMock) {
            await MockApi.logout();
        } else {
            await apiRequest(ENDPOINTS.logout, { method: "POST" });
        }
    } catch (err) {
        console.warn("Logout failed", err);
    }
    clearSession();
}

async function loadStatus() {
    try {
        const payload = useMock ? await MockApi.status() : await apiRequest(ENDPOINTS.status);
        state.statusCache = payload;
        renderStatus(payload);
    } catch (err) {
        console.warn("Unable to load status", err);
        renderStatus(null);
    }
}

function renderStatus(payload) {
    const wifi = payload?.wifi;
    if (!wifi) {
        selectors.wifiPill.textContent = "Wi-Fi: no data";
        setStatusIndicator("unknown");
    } else if (wifi.connected === true) {
        const rssi = wifi.rssi != null ? ` (${wifi.rssi} dBm)` : "";
        selectors.wifiPill.textContent = `Wi-Fi: ${wifi.ssid}${rssi}`;
        setStatusIndicator("connected");
    } else if (wifi.connected === false) {
        selectors.wifiPill.textContent = `Wi-Fi: ${wifi.mode ?? "offline"}`;
        setStatusIndicator("disconnected");
    } else {
        selectors.wifiPill.textContent = `Wi-Fi: ${wifi.mode ?? "unknown"}`;
        setStatusIndicator("unknown");
    }

    const relay = payload?.relay;
    selectors.relayState.textContent = relay?.state ? relay.state.toUpperCase() : "NO DATA";

    const probes = payload?.probes ?? [];
    if (!probes.length) {
        selectors.temperatureTable.innerHTML = '<tr class="placeholder"><td colspan="3">No data</td></tr>';
    } else {
        selectors.temperatureTable.innerHTML = probes
            .map((probe) => {
                const label = probe.name ?? probe.id ?? "Probe";
                const temperature = typeof probe.fahrenheit === "number"
                    ? `${probe.fahrenheit.toFixed(1)} °F`
                    : "No data";
                const updated = formatRelativeTime(probe.lastUpdated);
                return `
                <tr>
                    <td>${label}</td>
                    <td>${temperature}</td>
                    <td>${updated}</td>
                </tr>
            `;
            })
            .join("");
    }
}

function formatRelativeTime(timestamp) {
    if (!timestamp) return "No data";
    const parsed = Date.parse(timestamp);
    if (Number.isNaN(parsed)) return "No data";
    const delta = Math.floor((Date.now() - parsed) / 1000);
    if (delta < 60) return `${delta}s ago`;
    if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
    return `${Math.floor(delta / 3600)}h ago`;
}

function setStatusIndicator(state) {
    const dot = selectors.statusDot;
    if (!dot) return;
    dot.classList.remove("connected", "disconnected", "unknown");
    dot.classList.add(state);
}

async function setRelay(stateValue) {
    if (useMock) {
        await MockApi.relay({ state: stateValue });
    } else {
        await apiRequest(ENDPOINTS.relay, {
            method: "POST",
            body: JSON.stringify({ state: stateValue }),
        });
    }
    await loadStatus();
}

async function scanWifi() {
    const payload = useMock ? await MockApi.wifiScan() : await apiRequest(ENDPOINTS.wifiScan);
    const networks = payload.networks ?? [];
    selectors.wifiResults.innerHTML = networks.length
        ? networks
              .map((network) => {
                  const secure = network.secure ? "🔒" : "🔓";
                  return `
                <div class="item">
                    <span>${secure} ${network.ssid}</span>
                    <span>${network.rssi} dBm</span>
                </div>
            `;
              })
              .join("")
        : '<p class="empty">No networks</p>';
}

async function saveWifi(formData) {
    const body = {
        ssid: formData.get("ssid"),
        password: formData.get("password"),
    };
    if (useMock) {
        await MockApi.wifiSave(body);
    } else {
        await apiRequest(ENDPOINTS.wifiSave, {
            method: "POST",
            body: JSON.stringify(body),
        });
    }
    selectors.wifiMessage.textContent = "Wi-Fi credentials saved.";
    selectors.wifiMessage.hidden = false;
    await loadStatus();
}

async function loadDefaults() {
    try {
        const payload = useMock
            ? await MockApi.defaultsGet()
            : await apiRequest(ENDPOINTS.defaults);
        state.defaultsCache = payload;
        selectors.defaultsForm.default_state.value = payload.default_state ?? "off";
        selectors.defaultsForm.hysteresis.value = payload.hysteresis ?? 0;
        selectors.defaultsForm.min_on_temp.value = payload.min_on_temp ?? 0;
    } catch (err) {
        console.warn("Unable to load defaults", err);
        selectors.defaultsForm.reset();
    }
}

async function saveDefaults(formData) {
    const body = {
        default_state: formData.get("default_state"),
        hysteresis: Number(formData.get("hysteresis")),
        min_on_temp: Number(formData.get("min_on_temp")),
    };
    if (useMock) {
        await MockApi.defaultsSave(body);
    } else {
        await apiRequest(ENDPOINTS.defaults, {
            method: "POST",
            body: JSON.stringify(body),
        });
    }
    selectors.defaultsMessage.textContent = "Defaults saved.";
    selectors.defaultsMessage.hidden = false;
}

async function loadProbes() {
    try {
        const payload = useMock ? await MockApi.probes() : await apiRequest(ENDPOINTS.probes);
        state.probesCache = payload;
        selectors.probeList.innerHTML = payload.length
            ? payload
                  .map(
                      (probe) => `
                <div class="item">
                    <div>
                        <div>${probe.name ?? probe.id}</div>
                        <small>${probe.id}</small>
                    </div>
                    <span>${probe.enabled ? "Enabled" : "Disabled"}</span>
                </div>
            `,
                  )
                  .join("")
            : '<p class="empty">No probes</p>';
    } catch (err) {
        console.warn("Unable to load probes", err);
        selectors.probeList.innerHTML = '<p class="empty">No probes</p>';
    }
}

async function rebootController() {
    if (useMock) {
        await MockApi.reboot();
    } else {
        await apiRequest(ENDPOINTS.reboot, { method: "POST" });
    }
    selectors.adminMessage.textContent = "Rebooting controller...";
    selectors.adminMessage.hidden = false;
}

async function factoryReset() {
    if (!confirm("Factory reset will erase all settings. Continue?")) {
        return;
    }
    if (useMock) {
        await MockApi.factoryReset();
    } else {
        await apiRequest(ENDPOINTS.factoryReset, { method: "POST" });
    }
    selectors.adminMessage.textContent = "Factory reset complete.";
    selectors.adminMessage.hidden = false;
    await loadStatus();
}

async function updatePassword(formData) {
    const currentPassword = formData.get("current_password");
    const newPassword = formData.get("new_password");
    const confirmPassword = formData.get("confirm_password");

    selectors.passwordMessage.hidden = true;
    selectors.passwordMessage.classList.remove("error");

    if (newPassword !== confirmPassword) {
        selectors.passwordMessage.textContent = "New passwords do not match.";
        selectors.passwordMessage.classList.add("error");
        selectors.passwordMessage.hidden = false;
        return;
    }

    try {
        if (useMock) {
            await MockApi.changePassword({
                current_password: currentPassword,
                new_password: newPassword,
            });
        } else {
            await apiRequest(ENDPOINTS.passwordChange, {
                method: "POST",
                body: JSON.stringify({
                    current_password: currentPassword,
                    new_password: newPassword,
                }),
            });
        }
        selectors.passwordMessage.textContent = "Password updated successfully.";
        selectors.passwordMessage.hidden = false;
        selectors.passwordForm.reset();
    } catch (err) {
        selectors.passwordMessage.textContent = err.message || "Unable to update password.";
        selectors.passwordMessage.classList.add("error");
        selectors.passwordMessage.hidden = false;
    }
}

function showView(view) {
    state.currentView = view;
    selectors.navButtons.forEach((button) => {
        button.classList.toggle("active", button.dataset.view === view);
    });
    selectors.views.forEach((section) => {
        section.classList.toggle("hidden", section.id !== `${view}-view`);
    });
    selectors.viewTitle.textContent = view.charAt(0).toUpperCase() + view.slice(1);

    if (view === "status") {
        loadStatus();
    } else if (view === "wifi") {
        loadStatus();
    } else if (view === "controls") {
        loadDefaults();
    } else if (view === "probes") {
        loadProbes();
    }
}

function bindEvents() {
    selectors.loginForm.addEventListener("submit", async (event) => {
        event.preventDefault();
        selectors.loginError.hidden = true;
        const formData = new FormData(selectors.loginForm);
        try {
            await login(formData.get("username"), formData.get("password"));
            showAppShell();
            await loadStatus();
        } catch (err) {
            selectors.loginError.hidden = false;
        }
    });

    selectors.logout.addEventListener("click", async () => {
        await logout();
    });

    selectors.navButtons.forEach((button) => {
        button.addEventListener("click", () => showView(button.dataset.view));
    });

    selectors.relayButtons.forEach((button) => {
        button.addEventListener("click", () => setRelay(button.dataset.relay));
    });

    selectors.scanNetworks.addEventListener("click", scanWifi);

    selectors.wifiForm.addEventListener("submit", async (event) => {
        event.preventDefault();
        selectors.wifiMessage.hidden = true;
        await saveWifi(new FormData(selectors.wifiForm));
    });

    selectors.defaultsForm.addEventListener("submit", async (event) => {
        event.preventDefault();
        selectors.defaultsMessage.hidden = true;
        await saveDefaults(new FormData(selectors.defaultsForm));
    });

    selectors.refreshStatus.addEventListener("click", loadStatus);
    selectors.reboot.addEventListener("click", rebootController);
    selectors.factoryReset.addEventListener("click", factoryReset);
    selectors.passwordForm.addEventListener("submit", async (event) => {
        event.preventDefault();
        await updatePassword(new FormData(selectors.passwordForm));
    });
}

function init() {
    bindEvents();
    const savedToken = getStoredValue(STORAGE_KEYS.token);
    if (savedToken) {
        state.token = savedToken;
        showAppShell();
        restoreSession();
    } else {
        renderStatus(null);
        selectors.loginForm.username.focus();
    }
}

init();
