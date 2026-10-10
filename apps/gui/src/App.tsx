import { useEffect } from 'react';
import { useDbStore } from './store/useDbStore';
import { useWebSocket } from './features/telemetry/hooks/useWebSocket';
import { HeartPanel } from './features/telemetry/components/HeartPanel';
import { JujuCanvas } from './features/graph/components/JujuCanvas';
import { ValidationGate } from './features/sandbox/components/ValidationGate';
import { DbManager } from './features/database/components/DbManager';
import { SettingsPanel } from './features/settings/components/SettingsPanel';
import { AppShell } from './components/layout/AppShell';
import { Search } from 'lucide-react';
import './styles/App.css';

export default function App() {
  const { sendCommand } = useWebSocket();
  const nodes = useDbStore((state) => state.nodes);
  const links = useDbStore((state) => state.links);
  const searchQuery = useDbStore((state) => state.searchQuery);
  const setSearchQuery = useDbStore((state) => state.setSearchQuery);
  const searchResults = useDbStore((state) => state.searchResults);
  const setSelectedNode = useDbStore((state) => state.setSelectedNode);
  const activeView = useDbStore((state) => state.activeView);

  // Initialize beautiful mock database nodes at startup
  useEffect(() => {
    // Only populate if empty to prevent duplicate triggers
    if (nodes.length > 0) return;

    const mockNodes = [
      // IDENTITY CLUSTER
      { id: '1', name: 'Cluaizdb Core identity', label: 'OrgNeuron', cluster: 'IDENTITY', isRoot: true },
      { id: '2', name: 'Aryan Owner Node', label: 'BossNeuron', cluster: 'IDENTITY' },
      { id: '3', name: 'Engineering Core', label: 'DeptNeuron', cluster: 'IDENTITY' },
      // WORKFORCE CLUSTER
      { id: '4', name: 'Database Manager Agent', label: 'AgentNeuron', cluster: 'WORKFORCE' },
      { id: '5', name: 'Read/Write Synapse', label: 'SkillNeuron', cluster: 'WORKFORCE' },
      { id: '6', name: 'LMDB Driver Interface', label: 'ToolNeuron', cluster: 'WORKFORCE' },
      // KNOWLEDGE CLUSTER
      { id: '7', name: 'cluaizdb_manual.pdf', label: 'PageNeuron', cluster: 'KNOWLEDGE' },
      { id: '8', name: 'Hot state caching rules', label: 'PageNeuron', cluster: 'KNOWLEDGE' },
      // MEMORY CLUSTER
      { id: '9', name: 'Active Admin Session', label: 'SessionNode', cluster: 'MEMORY' },
      // REFLEX CLUSTER
      { id: '10', name: 'Write-Ahead Log Trigger', label: 'TriggerNeuron', cluster: 'REFLEX' },
      { id: '11', name: 'Deep Archer Sim Gate', label: 'DecisionGate', cluster: 'REFLEX' },
    ];

    const mockLinks = [
      { source: '1', target: '2' },
      { source: '1', target: '3' },
      { source: '3', target: '4' },
      { source: '4', target: '5' },
      { source: '4', target: '6' },
      { source: '1', target: '7' },
      { source: '7', target: '8' },
      { source: '1', target: '9' },
      { source: '5', target: '10' },
      { source: '10', target: '11' },
    ];

    useDbStore.setState({ nodes: mockNodes, links: mockLinks });
  }, [nodes]);

  const handleResultClick = (node: any) => {
    setSelectedNode(node);
    setSearchQuery('');
  };

  const renderActiveView = () => {
    switch (activeView) {
      case 'dashboard':
        return (
          <div className="flex flex-col gap-6 animate-slideUp">
            
            {/* Dashboard Sub-Header (Stats & Search Bar) */}
            <div className="flex flex-col md:flex-row items-center justify-between border-b-2 border-cyber-border/80 pb-4 gap-4">
              
              {/* Stats */}
              <div className="flex gap-4 text-[10px] font-mono uppercase text-cyber-text/60">
                <div className="flex items-center gap-1.5 px-3 py-1 bg-cyber-panel border-2 border-cyber-border">
                  <span className="w-1.5 h-1.5 rounded-full bg-cyber-neonGreen animate-pulse" />
                  <span>Neuron Nodes: {nodes.length}</span>
                </div>
                <div className="flex items-center gap-1.5 px-3 py-1 bg-cyber-panel border-2 border-cyber-border">
                  <span className="w-1.5 h-1.5 rounded-full bg-cyber-neonBlue animate-pulse" />
                  <span>Synaptic Links: {links.length}</span>
                </div>
              </div>

              {/* Search */}
              <div className="relative w-full md:w-72">
                <div className="flex items-center gap-2 px-3 py-1.5 bg-cyber-panel border-2 border-cyber-border">
                  <Search className="w-3.5 h-3.5 text-cyber-text/40" />
                  <input
                    type="text"
                    placeholder="Search database genome..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="flex-1 bg-transparent text-xs font-mono text-white outline-none placeholder-cyber-text/30"
                  />
                </div>

                {/* Search Dropdown */}
                {searchResults.length > 0 && (
                  <div className="absolute right-0 left-0 mt-2 bg-cyber-panel border-2 border-cyber-border shadow-[4px_4px_0px_0px_#000] overflow-hidden z-50">
                    {searchResults.map((n) => (
                      <button
                        key={n.id}
                        onClick={() => handleResultClick(n)}
                        className="w-full px-4 py-2.5 hover:bg-white/5 border-b border-cyber-border/40 last:border-0 text-left flex items-center justify-between text-xs transition-colors group"
                      >
                        <span className="font-bold text-white/80 group-hover:text-white">{n.name}</span>
                        <span className="font-mono text-[9px] uppercase text-cyber-neonBlue">{n.label}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>

            </div>

            {/* Bento Grid Layout */}
            <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-stretch">
              
              {/* Row 1, Col 1-4: Heart Telemetry */}
              <div className="lg:col-span-4 h-full">
                <HeartPanel sendCommand={sendCommand} />
              </div>

              {/* Row 1, Col 5-12: Infinite Juju Graph Canvas */}
              <div className="lg:col-span-8 h-full flex flex-col justify-between">
                <JujuCanvas />
              </div>

              {/* Row 2, Col 1-6: Security Gate */}
              <div className="lg:col-span-6 h-full">
                <ValidationGate />
              </div>

              {/* Row 2, Col 7-12: Engine Manager */}
              <div className="lg:col-span-6 h-full">
                <DbManager />
              </div>

            </div>

          </div>
        );
      
      case 'graph':
        return (
          <div className="w-full animate-slideUp">
            <JujuCanvas />
          </div>
        );

      case 'telemetry':
        return (
          <div className="w-full animate-slideUp">
            <HeartPanel sendCommand={sendCommand} />
          </div>
        );

      case 'database':
        return (
          <div className="w-full animate-slideUp">
            <DbManager />
          </div>
        );

      case 'sandbox':
        return (
          <div className="w-full animate-slideUp">
            <ValidationGate />
          </div>
        );

      case 'settings':
        return <SettingsPanel />;

      default:
        return null;
    }
  };

  return (
    <AppShell>
      {renderActiveView()}
    </AppShell>
  );
}
