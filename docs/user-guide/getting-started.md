# OpenRemote Getting Started Guide

Get your Windows PC and Android smartphone paired and ready for secure, low-latency remote control in under 10 minutes.

---

## 1. Prerequisites

- **Windows Host PC**: Windows 11 (64-bit) or Windows 10 (64-bit).
- **Android Smartphone / Tablet**: Android 8.0 (Oreo / API 26) or newer.
- **Network**: Local Wi-Fi network or active internet connection.

---

## 2. Windows PC Setup

1. **Install OpenRemote for Windows**:
   - Run `OpenRemote-Setup.exe`.
   - The OpenRemote background host service will install and launch automatically.
2. **Open the Dashboard**:
   - Open OpenRemote from your Start menu or system tray.
   - You will see your PC's Host ID and status: `Host Service: Online`.
3. **Generate Pairing Code**:
   - Click **Pair New Device** on the sidebar navigation.
   - A secure, single-use QR code invitation will appear on your monitor (valid for 5 minutes).

---

## 3. Android Setup

1. **Install OpenRemote on Android**:
   - Install the OpenRemote APK on your phone.
2. **Scan the QR Code**:
   - Open OpenRemote and tap **Pair New PC**.
   - Grant camera permission and point your phone's camera at the QR code displayed on your Windows screen.
   - The phone will automatically register cryptographic device credentials with your PC.
3. **Confirm Pairing**:
   - The device will show in your PC's **Trusted Devices** list.

---

## 4. Enabling Unattended Access (Optional)

By default, OpenRemote requires confirmation on the PC for every incoming connection.

To enable **Unattended Access** (connect anytime from your phone without touching your PC):
1. In the Windows OpenRemote dashboard, navigate to **Dashboard** or **Trusted Devices**.
2. Toggle the switch: **"Allow connections without desktop prompt"**.
3. You can now remotely connect, control your PC, and transfer files whenever the PC is powered on and online!

---

## 5. Controlling the Desktop

When connected from Android:
- **Touchpad Mode (Default)**: Drag your finger across the screen to move the mouse cursor. Tap once to Left-Click. Tap with two fingers to Right-Click.
- **Direct-Touch Mode**: Tap directly on UI elements to click at that exact coordinate.
- **Scrolling**: Drag with two fingers vertically to scroll web pages and documents.
- **Virtual Keyboard Toolbar**: Tap the keyboard icon in the floating action menu to access `Ctrl`, `Alt`, `Shift`, `Win`, arrow keys, function keys, and common shortcuts (`Ctrl+C`, `Ctrl+V`, `Ctrl+Alt+Del`).
- **File Transfer**: Use the file manager icon to upload files from your phone or download files from your PC.
- **Emergency Disconnect**: Hit the red power button on the Android toolbar or click **Disconnect** on the Windows tray icon to terminate the session instantly.
