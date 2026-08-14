import { useEffect } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useSettingsStore } from "@/store/settings";
import MainMenu from "@/pages/MainMenu";
import SettingsPage from "@/pages/Settings";
import TestPage from "@/pages/Test";
import ResultPage from "@/pages/Result";

export default function App() {
  const load = useSettingsStore((s) => s.load);
  const loaded = useSettingsStore((s) => s.loaded);

  // 启动时主动加载一次配置
  useEffect(() => {
    if (!loaded) {
      load();
    }
  }, [loaded, load]);

  return (
    <Routes>
      <Route path="/" element={<MainMenu />} />
      <Route path="/settings" element={<SettingsPage />} />
      <Route path="/test" element={<TestPage />} />
      <Route path="/result" element={<ResultPage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
