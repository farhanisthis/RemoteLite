import { useState } from "react";

interface TrustedDeviceView {
  id: string;
  name: string;
  platform: string;
  unattended: boolean;
  pairedAt: string;
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "pairing" | "devices" | "settings">("dashboard");
  const [unattendedEnabled, setUnattendedEnabled] = useState(false);
  const [devices] = useState<TrustedDeviceView[]>([
    {
      id: "dev-pixel-8",
      name: "Pixel 8 Pro (Primary)",
      platform: "Android",
      unattended: true,
      pairedAt: "2026-10-31",
    },
  ]);

  return (
    <div className="app-container">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-logo">OR</div>
          <div className="brand-title">OpenRemote</div>
        </div>
        <nav className="nav-menu">
          <button
            className={`nav-item ${activeTab === "dashboard" ? "active" : ""}`}
            onClick={() => setActiveTab("dashboard")}
          >
            Dashboard
          </button>
          <button
            className={`nav-item ${activeTab === "pairing" ? "active" : ""}`}
            onClick={() => setActiveTab("pairing")}
          >
            Pair New Device
          </button>
          <button
            className={`nav-item ${activeTab === "devices" ? "active" : ""}`}
            onClick={() => setActiveTab("devices")}
          >
            Trusted Devices
          </button>
          <button
            className={`nav-item ${activeTab === "settings" ? "active" : ""}`}
            onClick={() => setActiveTab("settings")}
          >
            Settings
          </button>
        </nav>
        <div className="host-status">
          <span className="status-dot online"></span>
          <span>Host Service: Online</span>
        </div>
      </aside>

      <main className="content">
        {activeTab === "dashboard" && (
          <section className="dashboard-section">
            <header className="page-header">
              <h1>This Computer</h1>
              <p className="subtitle">Ready to accept remote connections from authorized devices.</p>
            </header>

            <div className="cards-grid">
              <div className="card">
                <h3>Host Identity</h3>
                <div className="identity-badge">DESKTOP-WIN11-MAIN</div>
                <p className="caption">ID: 8a94b32c-7b01-49ea-b8fd-a0067b09c855</p>
              </div>

              <div className="card">
                <h3>Unattended Access</h3>
                <div className="toggle-row">
                  <span>Allow connections without desktop prompt</span>
                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={unattendedEnabled}
                      onChange={(e) => setUnattendedEnabled(e.target.checked)}
                    />
                    <span className="slider round"></span>
                  </label>
                </div>
                <p className="caption alert">
                  {unattendedEnabled
                    ? "Active: Only previously paired, trusted devices can connect."
                    : "Disabled: Each connection requires confirmation on this PC."}
                </p>
              </div>

              <div className="card">
                <h3>Active Sessions</h3>
                <div className="empty-state">No remote sessions active</div>
              </div>
            </div>
          </section>
        )}

        {activeTab === "pairing" && (
          <section className="pairing-section">
            <header className="page-header">
              <h1>Pair a New Android Device</h1>
              <p className="subtitle">Open OpenRemote on your phone and scan the code below.</p>
            </header>
            <div className="qr-container">
              <div className="qr-box">
                <div className="qr-mock">[ QR CODE INVITATION ]</div>
                <p className="qr-timer">Expires in 04:59</p>
              </div>
              <div className="pairing-instructions">
                <ol>
                  <li>Install OpenRemote on your Android smartphone.</li>
                  <li>Tap "Pair New Computer".</li>
                  <li>Point your camera at this screen.</li>
                  <li>Confirm the pairing request on both devices.</li>
                </ol>
              </div>
            </div>
          </section>
        )}

        {activeTab === "devices" && (
          <section className="devices-section">
            <header className="page-header">
              <h1>Trusted Devices</h1>
              <p className="subtitle">Devices authorized to access this PC.</p>
            </header>
            <div className="device-list">
              {devices.map((d) => (
                <div key={d.id} className="device-item">
                  <div className="device-info">
                    <h4>{d.name}</h4>
                    <span className="device-meta">
                      {d.platform} • Paired: {d.pairedAt} • {d.unattended ? "Unattended Allowed" : "Prompt Required"}
                    </span>
                  </div>
                  <button className="btn-danger">Revoke</button>
                </div>
              ))}
            </div>
          </section>
        )}

        {activeTab === "settings" && (
          <section className="settings-section">
            <header className="page-header">
              <h1>Host Settings</h1>
              <p className="subtitle">Network, display capture, and security policies.</p>
            </header>
            <div className="card">
              <h3>Capture & Encoding</h3>
              <p>Primary Capture API: Windows Graphics Capture (WGC)</p>
              <p>Fallback API: Desktop Duplication (DXGI)</p>
              <p>Video Codec: H.264 (Hardware MFT / NVENC / QSV)</p>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}
