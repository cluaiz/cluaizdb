import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  ChevronDown, 
  PanelLeftClose, 
  PanelLeftOpen, 
  Activity, 
  Database, 
  Shield, 
  LayoutDashboard,
  Sliders,
  Sparkles,
  Search
} from 'lucide-react';
import { useDbStore } from '../../store/useDbStore';

export function SecondarySidebar() {
  const activeView = useDbStore((state) => state.activeView);
  const setActiveView = useDbStore((state) => state.setActiveView);
  const activeSettingsSubView = useDbStore((state) => state.activeSettingsSubView);
  const setActiveSettingsSubView = useDbStore((state) => state.setActiveSettingsSubView);
  const isSecondaryOpen = useDbStore((state) => state.isSecondaryOpen);
  const toggleSecondary = useDbStore((state) => state.toggleSecondary);
  const setSearchModalOpen = useDbStore((state) => state.setSearchModalOpen);

  // Switcher state
  const activeOrg = useDbStore((state) => state.activeOrg);
  const setActiveOrg = useDbStore((state) => state.setActiveOrg);
  const activeUser = useDbStore((state) => state.activeUser);
  const setActiveUser = useDbStore((state) => state.setActiveUser);

  const [orgOpen, setOrgOpen] = useState(false);
  const [userOpen, setUserOpen] = useState(false);

  const orgs = ['Cluaizdb Core Shard', 'Research Shard Delta', 'Neural Labs', 'Global Ops Matrix'];
  const users = ['Aryan (Owner)', 'Neuron Operator', 'Guest Analyst', 'Root System Admin'];

  const getSubnavItems = () => {
    if (activeView === 'settings') {
      return [
        {
          group: 'SETTINGS & TUNING',
          items: [
            { label: 'Theme Settings', id: 'settings', icon: Sliders, badge: 'Config' }
          ]
        }
      ];
    } else {
      return [
        {
          group: 'COMMAND CENTER',
          items: [
            { label: 'System Overview', id: 'dashboard', icon: LayoutDashboard, badge: 'Matrix' },
            { label: 'Real-time Telemetry', id: 'telemetry', icon: Activity, badge: 'Live' },
          ]
        },
        {
          group: 'SYSTEM ENGINE',
          items: [
            { label: 'Database Manager', id: 'database', icon: Database, badge: 'Engine' },
            { label: 'Deep Archer Gate', id: 'sandbox', icon: Shield, badge: 'Secure' },
          ]
        }
      ];
    }
  };

  return (
    <motion.div
      initial={false}
      animate={{ width: isSecondaryOpen ? 240 : 64 }}
      transition={{ duration: 0.3, ease: 'easeInOut' }}
      className="h-full bg-cyber-panel/40 border-r-2 border-cyber-border flex flex-col select-none relative z-40 overflow-visible backdrop-blur-md"
    >
      {/* Header with Collapsible Toggle */}
      <div className="h-16 flex items-center justify-between border-b-2 border-cyber-border/80 px-4">
        {isSecondaryOpen && (
          <div className="flex items-center gap-2">
            <span className="text-sm leading-none">🪼</span>
            <h2 className="text-xs font-mono font-black tracking-widest text-cyber-heading uppercase truncate">
              {activeView === 'settings' ? 'SYSTEM CONFIG' : 'COGNITIVE OS'}
            </h2>
          </div>
        )}
        <button
          onClick={toggleSecondary}
          className={`p-1.5 border-2 border-cyber-border bg-cyber-bg hover:bg-white/5 text-cyber-text transition-all cursor-pointer hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)] active:shadow-[1px_1px_0px_0px_var(--cyber-accent-glow)] ${
            !isSecondaryOpen ? 'mx-auto' : ''
          }`}
          style={{ borderRadius: 0 }}
        >
          {isSecondaryOpen ? <PanelLeftClose className="w-3.5 h-3.5" /> : <PanelLeftOpen className="w-3.5 h-3.5" />}
        </button>
      </div>

      {isSecondaryOpen ? (
        <div className="flex-1 flex flex-col p-4 gap-6 overflow-y-auto [&::-webkit-scrollbar]:hidden">
          
          {/* Quick Search Command Palette Trigger */}
          <button
            onClick={() => setSearchModalOpen(true)}
            className="w-full flex items-center gap-2 px-3 py-2 bg-cyber-bg border-2 border-cyber-border text-left font-mono text-[10px] text-cyber-text/50 hover:border-cyber-accent/60 hover:text-cyber-heading transition-all cursor-pointer hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)] active:shadow-[1px_1px_0px_0px_var(--cyber-accent-glow)]"
            style={{ borderRadius: 0 }}
          >
            <Search className="w-3.5 h-3.5 text-cyber-accent shrink-0" />
            <span className="truncate flex-grow">Search...</span>
            <span className="text-[8px] opacity-65 shrink-0 bg-cyber-border px-1 border border-cyber-border">CTRL+K</span>
          </button>

          {/* Sub Navigation Items */}
          <div className="flex-1 flex flex-col gap-6 mt-2">
            {getSubnavItems().map((group, idx) => (
              <div key={idx} className="flex flex-col gap-2">
                <h3 className="text-[10px] font-mono font-black tracking-wider text-cyber-text/40 uppercase">
                  {group.group}
                </h3>
                <div className="flex flex-col gap-2">
                  {group.items.map((item, itemIdx) => {
                    const Icon = item.icon;
                    const isSettings = item.id === 'settings';
                    const isActive = isSettings 
                      ? (activeView === 'settings' && activeSettingsSubView === (item as any).subView)
                      : (activeView === item.id);
                    
                    const handleClick = () => {
                      if (isSettings) {
                        setActiveView('settings');
                        setActiveSettingsSubView((item as any).subView);
                      } else {
                        setActiveView(item.id as any);
                      }
                    };

                    return (
                      <button
                        key={itemIdx}
                        onClick={handleClick}
                        className={`w-full flex items-center gap-3 px-3 py-2 border-2 text-xs transition-all cursor-pointer ${
                          isActive
                            ? 'bg-cyber-accent/15 border-cyber-accent text-cyber-accent shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)]'
                            : 'border-cyber-border/40 bg-cyber-bg/25 text-cyber-text/80 hover:bg-white/5 hover:border-cyber-border hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)]'
                        }`}
                        style={{ borderRadius: 0 }}
                      >
                        <Icon className={`w-4 h-4 shrink-0 ${isActive ? 'text-cyber-accent' : 'text-cyber-text/50'}`} />
                        <span className="truncate flex-1 font-mono text-left">{item.label}</span>
                        {item.badge && (
                          <span className={`text-[9px] font-mono uppercase px-1 py-0.5 border shrink-0 ${
                            isActive 
                              ? 'bg-cyber-accent/20 border-cyber-accent text-cyber-accent' 
                              : 'bg-cyber-border/80 border-cyber-border text-cyber-heading'
                          }`}>
                            {item.badge}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>

        </div>
      ) : (
        /* Collapsed View */
        <div className="flex-1 flex flex-col items-center py-6 gap-6 overflow-y-auto">
          {/* Mini Search Icon */}
          <button
            onClick={() => setSearchModalOpen(true)}
            title="Search database (Ctrl+K)"
            className="w-8 h-8 bg-cyber-bg border-2 border-cyber-border flex items-center justify-center text-cyber-text/60 hover:border-cyber-accent/60 hover:text-cyber-heading transition-all cursor-pointer hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)] active:shadow-[1px_1px_0px_0px_var(--cyber-accent-glow)]"
            style={{ borderRadius: 0 }}
          >
            <Search className="w-4 h-4 text-cyber-accent" />
          </button>

          {/* Mini Icons */}
          {getSubnavItems().map((group) => 
            group.items.map((item, itemIdx) => {
              const Icon = item.icon;
              const isSettings = item.id === 'settings';
              const isActive = isSettings 
                ? (activeView === 'settings' && activeSettingsSubView === (item as any).subView)
                : (activeView === item.id);

              const handleClick = () => {
                if (isSettings) {
                  setActiveView('settings');
                  setActiveSettingsSubView((item as any).subView);
                } else {
                  setActiveView(item.id as any);
                }
              };

              return (
                <button
                  key={itemIdx}
                  onClick={handleClick}
                  title={`${group.group}: ${item.label}`}
                  className={`w-9 h-9 border-2 flex items-center justify-center transition-all cursor-pointer ${
                    isActive
                      ? 'bg-cyber-accent/15 border-cyber-accent text-cyber-accent hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)]'
                      : 'border-cyber-border/40 bg-cyber-bg/25 text-cyber-text/60 hover:bg-white/5 hover:border-cyber-border hover:shadow-[2px_2px_0px_0px_var(--cyber-accent-glow)]'
                  }`}
                  style={{ borderRadius: 0 }}
                >
                  <Icon className="w-4 h-4" />
                </button>
              );
            })
          )}
        </div>
      )}
    </motion.div>
  );
}
