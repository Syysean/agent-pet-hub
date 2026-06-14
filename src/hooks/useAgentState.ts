import { useState, useEffect, useRef, useCallback } from "react";
import { PetState } from "@agent-pet-hub/protocol";
import { PetStateSnapshot } from "@/types/pet";
import { listen } from "@tauri-apps/api/event";

const defaultSnapshot: PetStateSnapshot = {
  petState: "idle" as const,
  previousState: undefined,
  position: { x: 0, y: 0 },
  activeAgent: null,
  sessionId: undefined,
  errorCount: 0,
};

export interface UseAgentStateReturn {
  petState: PetStateSnapshot;
  previousState: PetState | undefined;
  setPetState: (state: PetState) => void;
  updatePosition: (pos: { x: number; y: number }) => void;
  setActiveAgent: (agent: string | null) => void;
  setSessionId: (id: string | undefined) => void;
  incrementError: () => void;
}

export function useAgentState(): UseAgentStateReturn {
  const [snapshot, setSnapshot] = useState<PetStateSnapshot>(defaultSnapshot);
  const [previousState, setPreviousState] = useState<PetState>();
  const snapshotRef = useRef(snapshot);
  snapshotRef.current = snapshot;

  const setPetState = useCallback((state: PetState) => {
    setPreviousState(snapshotRef.current.petState);
    setSnapshot(prev => ({
      ...prev,
      petState: state,
      previousState: prev.petState,
    }));
  }, []);

  useEffect(() => {
    let unlisten1: (() => void) | undefined;
    let unlisten2: (() => void) | undefined;

    const setup = async () => {
      try {
        unlisten1 = await listen<PetState>("pet:state_changed", (event) => {
          // 仅记录状态值，不记录完整 payload（可能含 raw 等敏感字段）
          console.log("[pet] state_changed:", event.payload);
          setPetState(event.payload);
        });
      } catch (e) {
        console.error("[pet] Failed to listen pet:state_changed:", e);
      }
      try {
        unlisten2 = await listen("pet:event", (event) => {
          // 记录事件类型和源，过滤掉 raw 字段防止泄露
          const { raw, ...safePayload } = event.payload as Record<string, unknown>;
          console.log("[pet] event:", safePayload);
        });
      } catch (e) {
        console.error("[pet] Failed to listen pet:event:", e);
      }
    };

    setup().catch(console.error);
    return () => { unlisten1?.(); unlisten2?.(); };
  }, [setPetState]);

  const updatePosition = useCallback((pos: { x: number; y: number }) => {
    setSnapshot(prev => ({ ...prev, position: pos }));
  }, []);

  const setActiveAgent = useCallback((agent: string | null) => {
    setSnapshot(prev => ({ ...prev, activeAgent: agent }));
  }, []);

  const setSessionId = useCallback((id: string | undefined) => {
    setSnapshot(prev => ({ ...prev, sessionId: id }));
  }, []);

  const incrementError = useCallback(() => {
    setSnapshot(prev => ({ ...prev, errorCount: prev.errorCount + 1 }));
  }, []);

  return {
    petState: snapshot,
    previousState,
    setPetState,
    updatePosition,
    setActiveAgent,
    setSessionId,
    incrementError,
  };
}