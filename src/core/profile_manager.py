import os
import json
from typing import Dict, Any, List

DEFAULT_PROFILES_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "profiles")

class ProfileManager:
    def __init__(self, profiles_dir: str = DEFAULT_PROFILES_DIR):
        self.profiles_dir = profiles_dir
        os.makedirs(self.profiles_dir, exist_ok=True)
        self._ensure_default_profiles()

    def list_profiles(self) -> List[str]:
        if not os.path.exists(self.profiles_dir):
            return []
        files = [f[:-5] for f in os.listdir(self.profiles_dir) if f.endswith(".json")]
        return sorted(files)

    def save_profile(self, name: str, data: Dict[str, Any]) -> str:
        safe_name = "".join(c for c in name if c.isalnum() or c in ("-", "_", " ")).strip()
        if not safe_name:
            safe_name = "profile"
        filepath = os.path.join(self.profiles_dir, f"{safe_name}.json")
        with open(filepath, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        return filepath

    def load_profile(self, name: str) -> Dict[str, Any]:
        filepath = os.path.join(self.profiles_dir, f"{name}.json")
        if not os.path.exists(filepath):
            return {}
        with open(filepath, "r", encoding="utf-8") as f:
            return json.load(f)

    def delete_profile(self, name: str) -> bool:
        filepath = os.path.join(self.profiles_dir, f"{name}.json")
        if os.path.exists(filepath):
            os.remove(filepath)
            return True
        return False

    def _ensure_default_profiles(self):
        presets = {
            "Быстрый_CPS_100": {
                "tab": "mouse",
                "mouse": {
                    "button": "left",
                    "click_type": "single",
                    "mode": "cps",
                    "cps": 100.0,
                    "interval_ms": 10.0,
                    "random_delay_ms": 1.0,
                    "location_type": "cursor",
                    "stop_condition": "infinite"
                }
            },
            "Minecraft_PVP_16_CPS": {
                "tab": "mouse",
                "mouse": {
                    "button": "left",
                    "click_type": "single",
                    "mode": "cps",
                    "cps": 16.0,
                    "interval_ms": 62.5,
                    "random_delay_ms": 5.0,
                    "location_type": "cursor",
                    "jitter_radius": 0,
                    "stop_condition": "infinite"
                }
            },
            "AFK_Анти-АФК_Скролл": {
                "tab": "scroll",
                "scroll": {
                    "direction": "down",
                    "ticks": 1,
                    "interval_ms": 500.0,
                    "random_delay_ms": 50.0,
                    "smooth": True,
                    "location_type": "cursor",
                    "stop_condition": "infinite"
                }
            },
            "Скилл_Ротация_1_2_3_E": {
                "tab": "keyboard",
                "keyboard": {
                    "mode": "sequence",
                    "sequence_steps": [
                        {"key": "1", "delay_ms": 150},
                        {"key": "2", "delay_ms": 150},
                        {"key": "3", "delay_ms": 150},
                        {"key": "e", "delay_ms": 200}
                    ],
                    "interval_ms": 500.0,
                    "random_delay_ms": 20.0,
                    "stop_condition": "infinite"
                }
            }
        }
        for name, data in presets.items():
            path = os.path.join(self.profiles_dir, f"{name}.json")
            if not os.path.exists(path):
                try:
                    with open(path, "w", encoding="utf-8") as f:
                        json.dump(data, f, indent=2, ensure_ascii=False)
                except Exception:
                    pass
