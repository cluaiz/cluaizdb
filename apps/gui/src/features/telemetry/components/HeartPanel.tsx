import React, { useState } from 'react';
import { useDbStore } from '../../../store/useDbStore';
import { Card } from '../../../components/ui/Card';
import { Button } from '../../../components/ui/Button';
import { Heart, ShieldAlert, Zap } from 'lucide-react';

interface HeartPanelProps {
  sendCommand: (command: any, payload?: any) => boolean;
}

export const HeartPanel: React.FC<HeartPanelProps> = ({ sendCommand }) => {
  const telemetry = useDbStore((state) => state.telemetry);
  const telemetryHistory = useDbStore((state) => state.telemetryHistory);
  const addLog = useDbStore((state) => state.addLog);
  const [pulseLimit, setPulseLimit] = useState(100);

  const handleAdrenaline = () => {
    sendCommand('adrenaline_shot');
    addLog('info', 'Adrenaline Injection: re-hydrating compressed storage tiers');
  };

  const handlePacemaker = (val: number) => {
    setPulseLimit(val);
    sendCommand('artificial_pacemaker', { pulse_limit: val * 1024 });
  };

  const handleComa = () => {
    if (window.confirm('WARNING: Force database suspension and WAL commit?')) {
      sendCommand('induced_coma');
      addLog('warn', 'Induced Coma: Emergency checkpoint committed');
    }
  };

  return (
    <Card className="flex flex-col h-full glow-green">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-cyber-border pb-2 mb-4">
        <div className="flex items-center gap-2">
          <Heart className="w-5 h-5 text-cyber-neonGreen animate-pulse" />
          <span className="font-mono text-xs uppercase tracking-wider text-cyber-neonGreen font-bold">
            Cluaizdb-HEART Engine Telemetry
          </span>
        </div>
        <div className="w-2 h-2 rounded-full bg-cyber-neonGreen animate-ping" />
      </div>

      {/* Grid containing metrics */}
      <div className="grid grid-cols-2 gap-3 mb-4">
        {/* Heart Rate */}
        <div className="bg-cyber-bg/40 border-2 border-cyber-border p-3 [border-radius:0px_!important] flex flex-col">
          <span className="text-[10px] uppercase font-mono tracking-wider text-cyber-text/60">Heart Rate (BPM)</span>
          <span className="text-2xl font-black font-mono text-cyber-neonGreen">{telemetry.heartRate}</span>
          <span className="text-[9px] text-cyber-text/40 font-mono mt-1">Throughput rate</span>
        </div>

        {/* Blood Pressure */}
        <div className="bg-cyber-bg/40 border-2 border-cyber-border p-3 [border-radius:0px_!important] flex flex-col">
          <span className="text-[10px] uppercase font-mono tracking-wider text-cyber-text/60">Blood Pressure (BP)</span>
          <span className="text-2xl font-black font-mono text-cyber-neonBlue">
            {telemetry.bloodPressureSystolic}/{telemetry.bloodPressureDiastolic}
          </span>
          <span className="text-[9px] text-cyber-text/40 font-mono mt-1">PCIe Bus / VRAM Saturation</span>
        </div>

        {/* Oxygen level */}
        <div className="bg-cyber-bg/40 border-2 border-cyber-border p-3 [border-radius:0px_!important] flex flex-col">
          <span className="text-[10px] uppercase font-mono tracking-wider text-cyber-text/60">Oxygen (SpO2)</span>
          <span className="text-2xl font-black font-mono text-cyber-neonYellow">
            {telemetry.oxygenLevel.toFixed(1)}%
          </span>
          <span className="text-[9px] text-cyber-text/40 font-mono mt-1">Context/KV-Cache available</span>
        </div>

        {/* Metabolic Rate */}
        <div className="bg-cyber-bg/40 border-2 border-cyber-border p-3 [border-radius:0px_!important] flex flex-col">
          <span className="text-[10px] uppercase font-mono tracking-wider text-cyber-text/60">Metabolic Rate</span>
          <span className="text-2xl font-black font-mono text-cyber-neonPink">
            {telemetry.metabolicRate.toFixed(2)}x
          </span>
          <span className="text-[9px] text-cyber-text/40 font-mono mt-1">GC / Compression factor</span>
        </div>
      </div>

      {/* Mini ECG History chart */}
      <div className="flex-1 bg-cyber-bg/50 border-2 border-cyber-border [border-radius:0px_!important] p-2 h-20 mb-4 flex items-end gap-[2px]">
        {telemetryHistory.map((pt, i) => {
          const height = Math.max(10, Math.min(100, ((pt.rate - 50) / 130) * 100));
          return (
            <div
              key={i}
              className="flex-1 bg-cyber-neonGreen/80 hover:bg-cyber-neonGreen rounded-t transition-all duration-300"
              style={{ height: `${height}%` }}
              title={`BPM: ${pt.rate} at ${pt.time}`}
            />
          );
        })}
        {telemetryHistory.length === 0 && (
          <div className="w-full text-center text-xs text-cyber-text/40 font-mono py-6">
            Waiting for heartbeat spikes...
          </div>
        )}
      </div>

      {/* Manual Controls */}
      <div className="border-t border-cyber-border pt-3 flex flex-col gap-3">
        <span className="text-[10px] font-mono uppercase text-cyber-text/60 tracking-wider">
          Biometric Control Overrides
        </span>

        {/* Buttons */}
        <div className="flex gap-2">
          {/* Adrenaline */}
          <Button onClick={handleAdrenaline} variant="green" className="flex-1">
            <Zap className="w-3.5 h-3.5" />
            Adrenaline Shot
          </Button>

          {/* Induced Coma */}
          <Button onClick={handleComa} variant="pink" className="flex-1">
            <ShieldAlert className="w-3.5 h-3.5" />
            Induced Coma
          </Button>
        </div>

        {/* Pacemaker Slider */}
        <div className="flex flex-col gap-1">
          <div className="flex items-center justify-between text-[10px] font-mono text-cyber-text/80">
            <span>Artificial Pacemaker (Pulse Limit)</span>
            <span className="text-cyber-neonBlue font-bold">{pulseLimit} MB</span>
          </div>
          <input
            type="range"
            min="10"
            max="1000"
            value={pulseLimit}
            onChange={(e) => handlePacemaker(Number(e.target.value))}
            className="w-full h-1 bg-cyber-border rounded-lg appearance-none cursor-pointer accent-cyber-neonBlue"
          />
        </div>
      </div>
    </Card>
  );
};
