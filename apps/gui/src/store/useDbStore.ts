import { create } from 'zustand';

export interface TelemetryData {
  heartRate: number;
  bloodPressureSystolic: number;
  bloodPressureDiastolic: number;
  oxygenLevel: number;
  metabolicRate: number;
}

export type AccentColorType = 'blue' | 'green' | 'pink' | 'yellow' | 'purple' | 'orange' | 'red' | 'teal';

export interface SettingsConfig {
  theme: 'dark' | 'light' | 'oled' | 'hacker';
  fontSize: number; // 14 to 18px matching ChatsSettings
  fontFamily: string; // Dynamic Google Fonts matching ChatsSettings
  hoverSidebar: boolean;
  accentColor: AccentColorType; // Unified theme color selection
}

export interface DatabaseState {
  // Telemetry
  telemetry: TelemetryData;
  telemetryHistory: { time: string; rate: number }[];
  setTelemetry: (data: TelemetryData) => void;

  // Nodes & Links for JujuCanvas Graph
  nodes: any[];
  links: any[];
  selectedNode: any | null;
  hoveredNode: any | null;
  searchQuery: string;
  searchResults: any[];
  setSelectedNode: (node: any | null) => void;
  setHoveredNode: (node: any | null) => void;
  setSearchQuery: (query: string) => void;

  // Database settings & mock connection types
  activeDbEngine: 'LMDB' | 'SQLite' | 'Postgres' | 'MongoDB' | 'PDF_Doc_Parser';
  setActiveDbEngine: (engine: 'LMDB' | 'SQLite' | 'Postgres' | 'MongoDB' | 'PDF_Doc_Parser') => void;

  // Logs & Commands status
  logs: { time: string; type: 'info' | 'warn' | 'error' | 'success'; message: string }[];
  addLog: (type: 'info' | 'warn' | 'error' | 'success', message: string) => void;
  clearLogs: () => void;

  // Deep Archer Gate Sandbox status
  sandboxStatus: 'idle' | 'simulating' | 'passed' | 'failed';
  setSandboxStatus: (status: 'idle' | 'simulating' | 'passed' | 'failed') => void;

  // Navigation & Multi-Tenant State
  activeView: 'dashboard' | 'settings' | 'telemetry' | 'database' | 'sandbox' | 'graph';
  setActiveView: (view: 'dashboard' | 'settings' | 'telemetry' | 'database' | 'sandbox' | 'graph') => void;
  activeSettingsSubView: 'engine' | 'visuals';
  setActiveSettingsSubView: (subView: 'engine' | 'visuals') => void;
  
  activeOrg: string;
  setActiveOrg: (org: string) => void;
  
  activeUser: string;
  setActiveUser: (user: string) => void;

  isSidebarOpen: boolean; // Mobile Drawer Open
  toggleSidebar: () => void;
  setSidebarOpen: (isOpen: boolean) => void;

  isSidebarVisible: boolean; // Desktop Sidebar visibility toggle (Ctrl + B)
  toggleSidebarVisible: () => void;
  setSidebarVisible: (isVisible: boolean) => void;

  isSearchModalOpen: boolean; // Global Search/Command Palette Modal (Ctrl + K)
  setSearchModalOpen: (isOpen: boolean) => void;

  isSecondaryOpen: boolean; // Desktop subbar expanded
  toggleSecondary: () => void;

  // App Custom Settings
  settings: SettingsConfig;
  updateSettings: (settings: Partial<SettingsConfig>) => void;
  
  // Cache TTL value (Internal store value, decoupled from settings UI)
  ttlValue: number;
  setTtlValue: (ttl: number) => void;
}

export const useDbStore = create<DatabaseState>((set) => ({
  telemetry: {
    heartRate: 72,
    bloodPressureSystolic: 120,
    bloodPressureDiastolic: 80,
    oxygenLevel: 98.5,
    metabolicRate: 1.0,
  },
  telemetryHistory: [],
  setTelemetry: (telemetry) => set((state) => {
    const time = new Date().toLocaleTimeString();
    const history = [...state.telemetryHistory, { time, rate: telemetry.heartRate }].slice(-20);
    return { telemetry, telemetryHistory: history };
  }),

  nodes: [],
  links: [],
  selectedNode: null,
  hoveredNode: null,
  searchQuery: '',
  searchResults: [],
  setSelectedNode: (node) => set({ selectedNode: node }),
  setHoveredNode: (node) => set({ hoveredNode: node }),
  setSearchQuery: (query) => set((state) => {
    if (!query.trim()) {
      return { searchQuery: query, searchResults: [] };
    }
    const matches = state.nodes.filter((n) =>
      n.name?.toLowerCase().includes(query.toLowerCase()) ||
      n.label?.toLowerCase().includes(query.toLowerCase())
    );
    return { searchQuery: query, searchResults: matches };
  }),

  activeDbEngine: 'LMDB',
  setActiveDbEngine: (engine) => set({ activeDbEngine: engine }),

  logs: [
    { time: new Date().toLocaleTimeString(), type: 'info', message: 'Cluaizdb-HEART engine boot initialized' },
    { time: new Date().toLocaleTimeString(), type: 'success', message: 'LMDB memory mapped page registers loaded' }
  ],
  addLog: (type, message) => set((state) => ({
    logs: [{ time: new Date().toLocaleTimeString(), type, message }, ...state.logs].slice(0, 50)
  })),
  clearLogs: () => set({ logs: [] }),

  sandboxStatus: 'idle',
  setSandboxStatus: (status) => set({ sandboxStatus: status }),

  activeView: 'dashboard',
  setActiveView: (view) => set({ activeView: view }),
  activeSettingsSubView: 'engine',
  setActiveSettingsSubView: (subView) => set({ activeSettingsSubView: subView }),
  
  activeOrg: 'Cluaizdb Core Shard',
  setActiveOrg: (org) => set({ activeOrg: org }),
  
  activeUser: 'Aryan (Owner)',
  setActiveUser: (user) => set({ activeUser: user }),

  isSidebarOpen: false,
  toggleSidebar: () => set((state) => ({ isSidebarOpen: !state.isSidebarOpen })),
  setSidebarOpen: (isOpen) => set({ isSidebarOpen: isOpen }),

  isSidebarVisible: true,
  toggleSidebarVisible: () => set((state) => ({ isSidebarVisible: !state.isSidebarVisible })),
  setSidebarVisible: (isVisible) => set({ isSidebarVisible: isVisible }),

  isSearchModalOpen: false,
  setSearchModalOpen: (isOpen) => set({ isSearchModalOpen: isOpen }),

  isSecondaryOpen: true,
  toggleSecondary: () => set((state) => ({ isSecondaryOpen: !state.isSecondaryOpen })),

  // Custom Settings Default
  settings: {
    theme: 'dark',
    fontSize: 14,
    fontFamily: 'Inter',
    hoverSidebar: true,
    accentColor: 'blue',
  },
  updateSettings: (newSettings) => set((state) => ({
    settings: { ...state.settings, ...newSettings }
  })),

  // Internal Cache TTL values
  ttlValue: 300,
  setTtlValue: (ttlValue) => set({ ttlValue }),
}));
