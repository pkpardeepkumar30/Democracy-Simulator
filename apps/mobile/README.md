# Expo mobile client

This client consumes the same Rust API as the Docker web game.

For a physical phone, `localhost` refers to the phone itself. Set the backend address before starting Expo:

```bash
# Windows PowerShell example
$env:EXPO_PUBLIC_API_URL="http://192.168.1.50:8080"
npm install
npm start
```

Replace `192.168.1.50` with the LAN address of the computer running the Docker backend. Open the QR code using Expo Go during development. Production Android and iOS packages can later be generated with EAS Build.
