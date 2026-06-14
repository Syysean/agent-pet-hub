import { PetPNG } from "./components/PetPNG";
import { PetStatus } from "./components/PetStatus";
import { useAgentState } from "./hooks/useAgentState";
import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";

export default function App() {
  const { petState, previousState } = useAgentState();
  const [skinId, setSkinId] = useState<string>("shark");

  // 从配置获取皮肤 ID
  useEffect(() => {
    const loadSkinId = async () => {
      try {
        const settings = await invoke<Record<string, unknown>>("get_settings");
        const petConfig = settings.pet as Record<string, unknown> | undefined;
        const id = petConfig?.skinId as string | undefined;
        if (id) {
          setSkinId(id);
        }
      } catch (e) {
        console.warn("[App] Failed to load skin_id from settings, using default:", e);
      }
    };
    loadSkinId();
  }, []);

  const handleDragStart = (e: React.MouseEvent) => {
    if (e.button === 0) {
      invoke("start_drag").catch(console.error);
    }
  };

  return (
    <div
      className="pet-container"
      style={{ width: "100%", height: "100%", cursor: "grab" }}
    >
      {/* 全屏透明覆盖层 - 确保窗口任意位置都能拖拽 */}
      <div
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          zIndex: 9999,
          cursor: "grab",
        }}
        onMouseDown={handleDragStart}
      />
      <PetPNG
        petState={petState.petState}
        previousState={previousState}
        skinId={skinId}
      />
      <PetStatus petState={petState} />
    </div>
  );
}
