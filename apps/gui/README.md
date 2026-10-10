# 🏛️ CLUAIZ CLUAIZD: THE VISUAL MATRIX (FRONTEND MONOREPO)

This is the self-contained, isolated frontend monorepo for the **Cluaizdb Nervous System Database (CLUAIZD)**. It powers the cognitive visual console, featuring **Cluaizdb-HEART** (real-time biomarker telemetry) and **Cluaizdb-JUJU** (3D infinite synaptic mesh graph).

The codebase is engineered around strict **DRY (Don't Repeat Yourself)** principles, a decoupled **Feature-Driven Architecture**, and a **Pixelated Voxel/Minecraft Neobrutalist design standard** matching the master `cluaiz.com` layout.

---

## 🛰️ THE VISION: MODULAR Independent TELEMETRY

Our absolute mission is to provide an identical, high-performance visual dashboard across all execution wrappers (Web, Desktop, Mobile) from a single shared codebase.

### 1. 🌍 Core UI & Logic Isolation

No shell wrappers contain any duplicate components, hooks, or stores. All operational UI code lives in the parent `src/` directory and is imported directly by platform wrappers.

### 2. 🧱 Neobrutalist Minecraft Voxel Standard

- **Sharp Corners:** Zero border-radius is enforced across all custom components (`border-radius: 0px !important`).
- **Flat Voxel Shadows:** Hard flat drop shadows (`shadow-[4px_4px_0px_0px_var(--shadow)]`).
- **Translate Hover Animations:** Components shift slightly on hover (`hover:-translate-x-0.5 hover:-translate-y-0.5`).
- **Dynamic Colors:** Fully controlled via CSS variables supporting light/dark system dynamics.

---

## 📂 Deep Feature-Driven Folder Structure

```text
apps/gui/
├── package.json                # Monorepo root (workspaces: ["apps/*"])
├── package-lock.json
│
├── src/                        # The Shared Core (@cluaizdb/gui)
│   ├── index.ts                # Entrypoint (exports App component)
│   ├── App.tsx                 # Master Bento Grid Dashboard Layout
│   ├── App.css                 # Main dashboard layout styles
│   │
│   ├── components/
│   │   └── ui/                 # Reusable Atomic UI Primitives (DRY Moat)
│   │       ├── Card.tsx        # Minecraft/Voxel styled Card container
│   │       └── Button.tsx      # Cyber-themed responsive Button
│   │
│   ├── features/               # Domain-Driven Functional Modules
│   │   ├── telemetry/          # Cluaizdb-HEART Telemetry Stream (BPM, BP, SpO2)
│   │   │   ├── components/HeartPanel.tsx
│   │   │   └── hooks/useWebSocket.ts
│   │   │
│   │   ├── database/           # Active Engine Management & Console Logs
│   │   │   └── components/DbManager.tsx
│   │   │
│   │   ├── sandbox/            # Deep Archer Volatile Sandbox State
│   │   │   └── components/ValidationGate.tsx
│   │   │
│   │   └── graph/              # Cluaizdb-JUJU 3D Synaptic Mesh Renderer
│   │       └── Geist/JujuCanvas.tsx
│   │
│   ├── store/                  # Unified State Management
│   │   └── useDbStore.ts       # Global Zustand store
│   │
│   ├── types/                  # Global Type Declarations
│   │   └── index.ts
│   │
│   └── styles/                 # Tailwind Config & Global Resets
│       └── index.css           # Tailwind + Custom Scrollbar from cluaiz.com
│
└── apps/                       # Thin Platform Wrappers (Shells)
    ├── shell-web/              # Vite Web Application Shell
    │   ├── package.json        # Declares workspace dependency "@cluaizdb/gui"
    │   ├── tsconfig.json       # Includes shared parent "../../src"
    │   ├── vite.config.ts      # Bypasses type definitions mapping
    │   └── src/main.tsx        # Boots shared App and CSS
    │
    └── shell-desktop/          # Vite + Tauri Desktop Application Shell
        ├── package.json        # Declares workspace dependency "@cluaizdb/gui"
        ├── src-tauri/          # Desktop compilation assets
        ├── tsconfig.json       # Includes shared parent "../../src"
        └── src/main.tsx        # Boots shared App and CSS
```

---

## 🏛️ Reusable Component Standards (DRY Law)

To keep code clean and prevent system drift, never create inline custom containers or style buttons manually. Use the UI primitives under `src/components/ui/`:

### 1. Card (`src/components/ui/Card.tsx`)

A voxel-card container with a flat black shadow that animates to a neon blue shadow on hover.

```tsx
import { Card } from "../../components/ui/Card";

<Card className="your-custom-styles">{children}</Card>;
```

### 2. Button (`src/components/ui/Button.tsx`)

Supports multiple cyber variants (`primary`, `green`, `pink`, `secondary`) with translation animations.

```tsx
import { Button } from "../../components/ui/Button";

<Button variant="green" onClick={handleEvent}>
	Adrenaline Shot
</Button>;
```

---

## 🚀 Development & Build Workflows

All npm commands must be run from this directory (`apps/gui/`):

### 1. Install Workspace Dependencies

```bash
npm install
```

### 2. Build the Web Application Shell

```bash
npm run build:web
```

### 3. Build the Desktop Tauri Application Shell

```bash
npm run build:desktop
```

---

**Cluaizdb Technology © 2026** — _CLUAIZD Visual Matrix_
