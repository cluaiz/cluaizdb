import React from 'react';
import { useDbStore } from '../../../store/useDbStore';
import { Card } from '../../../components/ui/Card';
import { ShieldCheck, ShieldAlert, AlertTriangle, RefreshCw } from 'lucide-react';

export const ValidationGate: React.FC = () => {
  const sandboxStatus = useDbStore((state) => state.sandboxStatus);
  const selectedNode = useDbStore((state) => state.selectedNode);
  const logs = useDbStore((state) => state.logs);

  const getStatusDisplay = () => {
    switch (sandboxStatus) {
      case 'simulating':
        return {
          icon: <RefreshCw className="w-8 h-8 text-cyber-neonBlue animate-spin" />,
          title: 'SANDBOX SIMULATING',
          desc: 'Simulating neural balance changes inside volatile LMDB clone...',
          color: 'text-cyber-neonBlue border-cyber-neonBlue/40 bg-cyber-neonBlue/5',
        };
      case 'passed':
        return {
          icon: <ShieldCheck className="w-8 h-8 text-cyber-neonGreen" />,
          title: 'DEEP ARCHER VERIFIED',
          desc: 'No Tumor Logic passed. Structurally safe to commit to physical WAL.',
          color: 'text-cyber-neonGreen border-cyber-neonGreen/40 bg-cyber-neonGreen/5 glow-green',
        };
      case 'failed':
        return {
          icon: <ShieldAlert className="w-8 h-8 text-cyber-neonPink" />,
          title: 'DEEP ARCHER BLOCKED',
          desc: 'Tumor Logic triggered! Vector weights hit NaN or Inf boundaries. Transaction aborted.',
          color: 'text-cyber-neonPink border-cyber-neonPink/40 bg-cyber-neonPink/5 glow-pink',
        };
      case 'idle':
      default:
        return {
          icon: <AlertTriangle className="w-8 h-8 text-cyber-neonYellow" />,
          title: 'DEEP ARCHER GATE IDLE',
          desc: 'Waiting for dynamic sliders or behavioral graft triggers to validate.',
          color: 'text-cyber-neonYellow border-cyber-neonYellow/40 bg-cyber-neonYellow/5',
        };
    }
  };

  const status = getStatusDisplay();

  return (
    <Card className={`flex flex-col h-full transition-all duration-300 ${status.color}`}>
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-cyber-border/40 pb-2 mb-3">
        <span className="font-mono text-xs uppercase tracking-wider font-bold">
          Deep Archer Security Gate
        </span>
      </div>

      {/* Main status indicator */}
      <div className="flex flex-col items-center justify-center text-center p-4 flex-1">
        <div className="mb-2">{status.icon}</div>
        <h3 className="font-mono text-sm font-black tracking-wider uppercase mb-1">
          {status.title}
        </h3>
        <p className="text-xs text-cyber-text/80 max-w-xs leading-relaxed">
          {status.desc}
        </p>
      </div>

      {/* Details Box */}
      {selectedNode && (
        <div className="bg-cyber-bg/50 border-2 border-cyber-border/40 p-2.5 [border-radius:0px_!important] flex flex-col gap-1 text-[11px] font-mono mt-2 text-cyber-text/85">
          <div className="flex justify-between">
            <span className="text-cyber-text/50">Target Shard:</span>
            <span>cluaizdb.mdb</span>
          </div>
          <div className="flex justify-between">
            <span className="text-cyber-text/50">Selected Neuron:</span>
            <span className="truncate max-w-[150px]">{selectedNode.name || selectedNode.id}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-cyber-text/50">OAT/OET Weights:</span>
            <span className="text-cyber-neonGreen">Balanced [16-D]</span>
          </div>
        </div>
      )}

      {/* Live simulation logs */}
      <div className="mt-3 border-t border-cyber-border/40 pt-2 h-24 overflow-y-auto flex flex-col gap-1 pr-1">
        {logs.slice(0, 3).map((log, i) => (
          <div key={i} className="text-[10px] font-mono flex gap-1.5 leading-tight text-cyber-text/70">
            <span className="text-cyber-text/40">{log.time}</span>
            <span className={`uppercase font-bold ${
              log.type === 'error' ? 'text-cyber-neonPink' :
              log.type === 'warn' ? 'text-cyber-neonYellow' :
              log.type === 'success' ? 'text-cyber-neonGreen' : 'text-cyber-neonBlue'
            }`}>
              [{log.type}]
            </span>
            <span className="truncate">{log.message}</span>
          </div>
        ))}
      </div>
    </Card>
  );
};
