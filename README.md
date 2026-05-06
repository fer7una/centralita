# Centralita

Scaffold base de `Tauri 2 + React + TypeScript + Vite` para una app de escritorio portable orientada a organizar y ejecutar proyectos locales.

## Requisitos

- Node.js 20+
- npm 11+
- Rust con toolchain `stable-msvc`
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime

## Arranque en desarrollo

```powershell
npm install
npm run tauri:dev
```

Esto levanta Vite en `http://localhost:1420` y abre la ventana nativa de Tauri.

## Build del frontend

```powershell
npm run build
```

## Estructura relevante

- `src/`: frontend React + TypeScript
- `src/shared/api/tauri/`: contrato IPC tipado del frontend
- `src-tauri/`: backend Rust y configuracion de Tauri
- `docs/ARCHITECTURE.md`: mapa de capas y reglas de dependencias
- `vite.config.ts`: configuracion de Vite adaptada a Tauri

## Scripts

- `npm run dev`: desarrollo web con Vite
- `npm run build`: compilacion del frontend
- `npm run tauri:dev`: desarrollo de la app de escritorio con Tauri
- `npm run tauri -- build`: build de escritorio con Tauri
