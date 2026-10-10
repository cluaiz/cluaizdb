import { useEffect, useRef, useCallback } from 'react';
import { useDbStore, type TelemetryData } from '../../../store/useDbStore';

export const useWebSocket = (url: string = 'ws://localhost:7331/ws/telemetry') => {
  const wsRef = useRef<WebSocket | null>(null);
  const setTelemetry = useDbStore((state) => state.setTelemetry);
  const addLog = useDbStore((state) => state.addLog);

  const connect = useCallback(() => {
    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        addLog('success', 'Connected to Cluaizdb-HEART telemetry server');
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.heart_rate_bpm !== undefined) {
            const telemetry: TelemetryData = {
              heartRate: data.heart_rate_bpm,
              bloodPressureSystolic: data.blood_pressure_systolic,
              bloodPressureDiastolic: data.blood_pressure_diastolic,
              oxygenLevel: data.oxygen_level_spo2,
              metabolicRate: data.metabolic_rate,
            };
            setTelemetry(telemetry);
          }
        } catch (e) {
          console.error('Failed to parse websocket telemetry message', e);
        }
      };

      ws.onclose = () => {
        addLog('warn', 'Telemetry server connection closed. Retrying...');
        setTimeout(connect, 3000); // Auto-reconnect
      };

      ws.onerror = () => {
        addLog('error', 'Telemetry WebSocket encountered an error');
      };
    } catch (e) {
      console.error('WebSocket connection failed', e);
    }
  }, [url, setTelemetry, addLog]);

  useEffect(() => {
    connect();
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [connect]);

  const sendCommand = useCallback((command: 'adrenaline_shot' | 'induced_coma' | 'artificial_pacemaker', payload?: any) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      const msg = JSON.stringify({
        command,
        payload,
      });
      wsRef.current.send(msg);
      addLog('info', `Sent HEART Override Command: ${command.toUpperCase()}`);
      return true;
    }
    addLog('error', `Failed to send command ${command}: Server disconnected`);
    return false;
  }, [addLog]);

  return { sendCommand };
};
