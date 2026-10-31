import { useState, useEffect, useCallback } from "react";
import QRCode from "qrcode";
import { invoke } from "@tauri-apps/api/core";

interface HostInfo {
  device_id: string;
  display_name: string;
  public_key: string;
  storage_path: string;
  registry_path: string;
  signaling_url: string;
  lan_ip: string;
  is_online: boolean;
  trusted_device_count: number;
  active_session_count: number;
}

interface DevicePermissions {
  remote_control: boolean;
  file_transfer: boolean;
  remote_reboot: boolean;
  clipboard_sync: boolean;
  audio_streaming: boolean;
}

interface TrustedDevice {
  device_id: string;
  display_name: string;
  platform: string;
  public_key: string;
  paired_at: string;
  last_seen_at?: string;
  unattended_access_enabled: boolean;
  permissions: DevicePermissions;
}

interface QrPairingPayload {
  server_url: string;
  invitation_token: string;
  host_public_key: string;
  host_device_id: string;
  host_display_name: string;
  expires_at: string;
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "pairing" | "devices" | "settings">("dashboard");
  const [hostInfo, setHostInfo] = useState<HostInfo | null>(null);
  const [devices, setDevices] = useState<TrustedDevice[]>([]);
  const [pairingPayload, setPairingPayload] = useState<QrPairingPayload | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState(false);
  const [copiedPayload, setCopiedPayload] = useState(false);
  const [simDeviceName, setSimDeviceName] = useState("Pixel 8 Pro");
  const [simUnattended, setSimUnattended] = useState(true);
  const [signalingInput, setSignalingInput] = useState("");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const showStatus = (msg: string) => {
    setStatusMessage(msg);
    setTimeout(() => setStatusMessage(null), 4000);
  };

  const loadHostInfo = useCallback(async () => {
    try {
      const info = await invoke<HostInfo>("get_host_info");
      setHostInfo(info);
      if (!signalingInput) {
        setSignalingInput(info.signaling_url);
      }
    } catch (e) {
      console.warn("Failed to load host info from IPC:", e);
    }
  }, [signalingInput]);

  const loadDevices = useCallback(async () => {
    try {
      const list = await invoke<TrustedDevice[]>("get_trusted_devices");
      setDevices(list);
    } catch (e) {
      console.warn("Failed to load devices from IPC:", e);
    }
  }, []);

  const generateQr = useCallback(async () => {
    try {
      const payload = await invoke<QrPairingPayload>("create_pairing_invitation", {
        serverUrl: signalingInput || undefined,
      });
      setPairingPayload(payload);
      const jsonStr = JSON.stringify(payload);
      const url = await QRCode.toDataURL(jsonStr, {
        width: 240,
        margin: 2,
        color: {
          dark: "#0f172a",
          light: "#ffffff",
        },
      });
      setQrDataUrl(url);
    } catch (e) {
      console.error("Failed to generate QR code:", e);
      showStatus("Error generating pairing QR code");
    }
  }, [signalingInput]);

  useEffect(() => {
    loadHostInfo();
    loadDevices();
    const interval = setInterval(() => {
      loadHostInfo();
      loadDevices();
    }, 3000);
    return () => clearInterval(interval);
  }, [loadHostInfo, loadDevices]);

  useEffect(() => {
    if (activeTab === "pairing" && !qrDataUrl) {
      generateQr();
    }
  }, [activeTab, qrDataUrl, generateQr]);

  const handleToggleUnattended = async (device: TrustedDevice) => {
    try {
      await invoke("set_unattended_access", {
        deviceId: device.device_id,
        enabled: !device.unattended_access_enabled,
      });
      await loadDevices();
      showStatus(`Updated unattended access for ${device.display_name}`);
    } catch (e) {
      showStatus(`Error updating unattended access: ${e}`);
    }
  };

  const handleRevokeDevice = async (deviceId: string, name: string) => {
    if (!window.confirm(`Revoke trust for "${name}"? Active connections will be immediately terminated.`)) {
      return;
    }
    try {
      await invoke("revoke_device", { deviceId });
      await loadDevices();
      await loadHostInfo();
      showStatus(`Revoked access for ${name}`);
    } catch (e) {
      showStatus(`Error revoking device: ${e}`);
    }
  };

  const handleSimulatePairing = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!simDeviceName.trim()) return;
    try {
      await invoke("add_simulated_device", {
        displayName: simDeviceName.trim(),
        platform: "android",
        unattended: simUnattended,
      });
      await loadDevices();
      await loadHostInfo();
      showStatus(`Paired test device: ${simDeviceName}`);
      setActiveTab("devices");
    } catch (e) {
      showStatus(`Error adding test device: ${e}`);
    }
  };

  const handleSaveSignalingUrl = async () => {
    try {
      await invoke("set_signaling_url", { url: signalingInput });
      await loadHostInfo();
      await generateQr();
      showStatus("Signaling server address updated!");
    } catch (e) {
      showStatus(`Error saving signaling URL: ${e}`);
    }
  };

  const copyToClipboard = (text: string, type: "id" | "payload") => {
    navigator.clipboard.writeText(text);
    if (type === "id") {
      setCopiedId(true);
      setTimeout(() => setCopiedId(false), 2000);
    } else {
      setCopiedPayload(true);
      setTimeout(() => setCopiedPayload(false), 2000);
    }
  };

  const displayName = hostInfo?.display_name || "Windows Host";
  const deviceId = hostInfo?.device_id || "Loading...";
  const publicKey = hostInfo?.public_key || "Loading...";
  const isOnline = hostInfo?.is_online ?? false;

  return (
    <div className="app-container">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-logo">OR</div>
          <div>
            <div className="brand-title">OpenRemote</div>
            <div className="brand-subtitle">v0.2.0 • Personal Edition</div>
          </div>
        </div>

        <nav className="nav-menu">
          <button
            className={`nav-item ${activeTab === "dashboard" ? "active" : ""}`}
            onClick={() => setActiveTab("dashboard")}
          >
            📊 Dashboard
          </button>
          <button
            className={`nav-item ${activeTab === "pairing" ? "active" : ""}`}
            onClick={() => setActiveTab("pairing")}
          >
            📱 Pair Android Device
          </button>
          <button
            className={`nav-item ${activeTab === "devices" ? "active" : ""}`}
            onClick={() => setActiveTab("devices")}
          >
            🔐 Trusted Devices ({devices.length})
          </button>
          <button
            className={`nav-item ${activeTab === "settings" ? "active" : ""}`}
            onClick={() => setActiveTab("settings")}
          >
            ⚙️ Settings & Network
          </button>
        </nav>

        <div className="host-status">
          <span className={`status-dot ${isOnline ? "online" : "offline"}`}></span>
          <div>
            <div>Coordination: {isOnline ? "Connected" : "Standby / Local"}</div>
            <div className="caption">LAN IP: {hostInfo?.lan_ip || "127.0.0.1"}</div>
          </div>
        </div>
      </aside>

      <main className="content">
        {statusMessage && (
          <div className="notification-banner">
            <span>ℹ️ {statusMessage}</span>
          </div>
        )}

        {activeTab === "dashboard" && (
          <section className="dashboard-section">
            <header className="page-header">
              <h1>This Computer</h1>
              <p className="subtitle">Ready to accept secure remote connections from authorized devices.</p>
            </header>

            <div className="cards-grid">
              <div className="card">
                <h3>Host Identity</h3>
                <div className="identity-badge">{displayName}</div>
                <div className="meta-row">
                  <span className="caption">Device ID:</span>
                  <button
                    className="btn-tiny"
                    onClick={() => copyToClipboard(deviceId, "id")}
                    title="Copy Device ID"
                  >
                    {copiedId ? "Copied! ✓" : "Copy"}
                  </button>
                </div>
                <div className="key-snippet">{deviceId}</div>

                <div className="meta-row" style={{ marginTop: "10px" }}>
                  <span className="caption">Ed25519 Public Key:</span>
                </div>
                <div className="key-snippet small">{publicKey.slice(0, 32)}...</div>
              </div>

              <div className="card">
                <h3>Device Authorization</h3>
                <p className="caption" style={{ marginBottom: "14px" }}>
                  Zero-trust security model: Only explicitly paired devices can ever connect. Unattended access requires per-device authorization.
                </p>
                <div className="stat-row">
                  <div>
                    <div className="stat-number">{devices.length}</div>
                    <div className="caption">Trusted Devices</div>
                  </div>
                  <div>
                    <div className="stat-number">
                      {devices.filter((d) => d.unattended_access_enabled).length}
                    </div>
                    <div className="caption">Unattended Enabled</div>
                  </div>
                </div>
                <div style={{ marginTop: "16px" }}>
                  <button className="btn-primary" onClick={() => setActiveTab("pairing")}>
                    + Pair New Device
                  </button>
                </div>
              </div>

              <div className="card">
                <h3>Active Remote Sessions</h3>
                {hostInfo?.active_session_count === 0 ? (
                  <div className="empty-state">
                    <p>No remote sessions active.</p>
                    <p className="caption">When an authorized Android phone connects, session details and disconnect controls appear here.</p>
                  </div>
                ) : (
                  <div className="session-active-card">
                    <div className="session-indicator">● Active Connection</div>
                  </div>
                )}
              </div>
            </div>
          </section>
        )}

        {activeTab === "pairing" && (
          <section className="pairing-section">
            <header className="page-header">
              <h1>Pair a New Android Device</h1>
              <p className="subtitle">
                Scan this cryptographically signed QR code with the OpenRemote Android app.
              </p>
            </header>

            <div className="qr-container">
              <div className="qr-box">
                {qrDataUrl ? (
                  <img src={qrDataUrl} alt="Pairing QR Code" className="qr-image" />
                ) : (
                  <div className="qr-placeholder">Generating Cryptographic QR Code...</div>
                )}
                {pairingPayload && (
                  <div className="qr-timer">
                    Valid for 10 minutes (expires: {new Date(pairingPayload.expires_at).toLocaleTimeString()})
                  </div>
                )}
                <div className="qr-actions">
                  <button className="btn-secondary" onClick={generateQr}>
                    🔄 Regenerate New Code
                  </button>
                  {pairingPayload && (
                    <button
                      className="btn-secondary"
                      onClick={() => copyToClipboard(JSON.stringify(pairingPayload), "payload")}
                    >
                      {copiedPayload ? "Copied! ✓" : "📋 Copy Payload"}
                    </button>
                  )}
                </div>
              </div>

              <div className="pairing-instructions">
                <h3>Pairing Steps:</h3>
                <ol>
                  <li>
                    Install <strong>openremote-android.apk</strong> on your Android phone or tablet.
                  </li>
                  <li>
                    Open the app and tap <strong>"Pair New Computer"</strong>.
                  </li>
                  <li>Point your Android camera at the QR code on this screen.</li>
                  <li>Confirm the pairing key comparison on both devices.</li>
                </ol>
                <div className="info-box">
                  <strong>Network Note:</strong>
                  <p>
                    QR invitation is broadcast for signaling hub at:
                    <br />
                    <code>{signalingInput || hostInfo?.signaling_url}</code>
                  </p>
                </div>
              </div>
            </div>
          </section>
        )}

        {activeTab === "devices" && (
          <section className="devices-section">
            <header className="page-header">
              <div className="header-split">
                <div>
                  <h1>Trusted Devices Registry</h1>
                  <p className="subtitle">
                    Stored in Windows DPAPI-protected registry ({hostInfo?.registry_path || "%ProgramData%"}).
                  </p>
                </div>
              </div>
            </header>

            {devices.length === 0 ? (
              <div className="empty-registry-card">
                <h3>No Devices Paired Yet</h3>
                <p className="caption" style={{ maxWidth: "480px", margin: "10px auto 20px auto" }}>
                  To authorize an Android phone or tablet, scan the QR code in the "Pair Android Device" tab.
                  Or add a test device below to simulate and verify unattended access controls.
                </p>
                <button className="btn-primary" onClick={() => setActiveTab("pairing")}>
                  Scan QR Code to Pair
                </button>
              </div>
            ) : (
              <div className="device-list">
                {devices.map((device) => (
                  <div key={device.device_id} className="device-item">
                    <div className="device-info">
                      <div className="device-title-row">
                        <h4>{device.display_name}</h4>
                        <span className="platform-tag">{device.platform}</span>
                      </div>
                      <div className="device-meta">
                        ID: <code>{device.device_id}</code>
                        <br />
                        Paired on: {new Date(device.paired_at).toLocaleString()}
                      </div>

                      <div className="permissions-bar">
                        <span className="perm-label">Permissions:</span>
                        <span className="perm-chip active">Remote Control</span>
                        <span className={`perm-chip ${device.permissions.file_transfer ? "active" : ""}`}>
                          Files
                        </span>
                        <span className={`perm-chip ${device.permissions.clipboard_sync ? "active" : ""}`}>
                          Clipboard
                        </span>
                        <span className={`perm-chip ${device.permissions.remote_reboot ? "active" : ""}`}>
                          Reboot
                        </span>
                      </div>
                    </div>

                    <div className="device-controls">
                      <div className="unattended-toggle">
                        <span className="toggle-label">Unattended:</span>
                        <label className="switch">
                          <input
                            type="checkbox"
                            checked={device.unattended_access_enabled}
                            onChange={() => handleToggleUnattended(device)}
                          />
                          <span className="slider round"></span>
                        </label>
                      </div>

                      <button
                        className="btn-danger"
                        onClick={() => handleRevokeDevice(device.device_id, device.display_name)}
                      >
                        Revoke Access
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <div className="card" style={{ marginTop: "30px" }}>
              <h3>Quick Test Device Simulator</h3>
              <p className="caption" style={{ marginBottom: "14px" }}>
                Add a simulated trusted Android device to test authorization, unattended access toggles, and instant revocation:
              </p>
              <form onSubmit={handleSimulatePairing} className="sim-form">
                <input
                  type="text"
                  className="text-input"
                  value={simDeviceName}
                  onChange={(e) => setSimDeviceName(e.target.value)}
                  placeholder="Device Name (e.g. Pixel 8 Pro)"
                />
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={simUnattended}
                    onChange={(e) => setSimUnattended(e.target.checked)}
                  />
                  <span>Enable Unattended Access</span>
                </label>
                <button type="submit" className="btn-secondary">
                  + Add Test Device
                </button>
              </form>
            </div>
          </section>
        )}

        {activeTab === "settings" && (
          <section className="settings-section">
            <header className="page-header">
              <h1>Host Settings & Network</h1>
              <p className="subtitle">
                System configuration, hardware video encoder, and signaling hub connection.
              </p>
            </header>

            <div className="cards-grid">
              <div className="card">
                <h3>Signaling Server Connection</h3>
                <p className="caption" style={{ marginBottom: "12px" }}>
                  Android devices connect through this signaling URL to negotiate peer-to-peer WebRTC streams.
                </p>
                <div className="input-group">
                  <input
                    type="text"
                    className="text-input"
                    value={signalingInput}
                    onChange={(e) => setSignalingInput(e.target.value)}
                    placeholder="ws://192.168.1.xxx:3000/ws"
                  />
                  <button className="btn-primary" onClick={handleSaveSignalingUrl}>
                    Save & Update QR
                  </button>
                </div>
                <p className="caption" style={{ marginTop: "8px" }}>
                  Detected Local LAN IP: <strong>{hostInfo?.lan_ip}</strong>
                </p>
              </div>

              <div className="card">
                <h3>Windows Desktop Capture & Video Codec</h3>
                <div className="settings-item">
                  <div className="setting-title">Primary Capture API</div>
                  <div className="setting-val">Windows Graphics Capture (WGC) • 30/60 FPS</div>
                </div>
                <div className="settings-item">
                  <div className="setting-title">Fallback Capture API</div>
                  <div className="setting-val">Desktop Duplication API (DXGI Direct3D 11)</div>
                </div>
                <div className="settings-item">
                  <div className="setting-title">Video Encoder</div>
                  <div className="setting-val">Hardware H.264 (Windows Media Foundation MFT)</div>
                </div>
                <div className="settings-item">
                  <div className="setting-title">Input Injection</div>
                  <div className="setting-val">Win32 SendInput (Rate-limited, Bounds-checked)</div>
                </div>
              </div>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}
