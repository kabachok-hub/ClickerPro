import os
import sys
import cv2
import numpy as np
import mss
from win_input import ensure_desktop_access

def imread_unicode(path, flags=cv2.IMREAD_UNCHANGED):
    """Safely loads image files with Cyrillic/Unicode paths on Windows."""
    try:
        with open(path, "rb") as f:
            bytes_data = np.frombuffer(f.read(), np.uint8)
            return cv2.imdecode(bytes_data, flags)
    except Exception as e:
        print(f"Error loading {path}: {e}")
        return None

def imwrite_unicode(path, img):
    """Safely writes image files with Cyrillic/Unicode paths on Windows."""
    try:
        ext = os.path.splitext(path)[1]
        ok, buf = cv2.imencode(ext, img)
        if ok:
            with open(path, "wb") as f:
                f.write(buf)
            return True
    except Exception as e:
        print(f"Error saving {path}: {e}")
    return False

class PopupDetector:
    def __init__(self, assets_dir=None):
        if assets_dir is None:
            if getattr(sys, 'frozen', False):
                base_dir = getattr(sys, '_MEIPASS', os.path.dirname(sys.executable))
                assets_dir = os.path.join(base_dir, "assets")
                if not os.path.exists(assets_dir):
                    assets_dir = os.path.join(os.path.dirname(sys.executable), "assets")
            else:
                base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
                assets_dir = os.path.join(base_dir, "assets")
        
        self.assets_dir = assets_dir
        self.templates = {}
        self.load_embedded_templates()
        self.sct = None
        
    def get_sct(self):
        if self.sct is None:
            ensure_desktop_access()
            self.sct = mss.MSS()
        return self.sct

    def capture_screen(self, region=None):
        """
        Captures screen. If region is None, captures primary monitor.
        Region format: (x1, y1, x2, y2)
        Returns BGR numpy array and (offset_x, offset_y).
        """
        ensure_desktop_access()
        sct = self.get_sct()
        
        if region is not None:
            x1, y1, x2, y2 = region
            rx1 = min(x1, x2)
            ry1 = min(y1, y2)
            rw = max(1, abs(x2 - x1))
            rh = max(1, abs(y2 - y1))
            monitor = {"top": ry1, "left": rx1, "width": rw, "height": rh}
            offset_x, offset_y = rx1, ry1
        else:
            mon = sct.monitors[1]
            monitor = mon
            offset_x, offset_y = mon["left"], mon["top"]

        img_raw = np.array(sct.grab(monitor))
        img_bgr = img_raw[:, :, :3]
        return img_bgr, offset_x, offset_y

    def load_embedded_templates(self):
        roblox_path = os.path.join(self.assets_dir, "roblox_button.png")
        if os.path.exists(roblox_path):
            tmpl = imread_unicode(roblox_path, cv2.IMREAD_UNCHANGED)
            if tmpl is not None:
                th, tw = tmpl.shape[:2]
                bgr = tmpl[:, :, :3]
                mask = tmpl[:, :, 3] if tmpl.shape[2] == 4 else None
                self.templates["roblox_button"] = {
                    "bgr": bgr,
                    "mask": mask,
                    "w": tw,
                    "h": th,
                }

    def add_custom_template(self, name, path):
        tmpl = imread_unicode(path, cv2.IMREAD_UNCHANGED)
        if tmpl is not None:
            th, tw = tmpl.shape[:2]
            bgr = tmpl[:, :, :3]
            mask = tmpl[:, :, 3] if tmpl.shape[2] == 4 else None
            self.templates[name] = {
                "bgr": bgr,
                "mask": mask,
                "w": tw,
                "h": th,
            }
            return True
        return False

    def match_template_coarse_fine(self, screen_bgr, tmpl_data, min_conf=0.70):
        """
        Ultra-fast coarse-to-fine template matching with alpha mask support.
        Speed: 20-40ms on full 1080p screen.
        """
        bgr = tmpl_data["bgr"]
        mask = tmpl_data["mask"]
        tw = tmpl_data["w"]
        th = tmpl_data["h"]
        sh, sw = screen_bgr.shape[:2]
        
        if sw < tw or sh < th:
            return None

        # 1. Coarse Pass at 0.5x scale
        scale = 0.5
        small_screen = cv2.resize(screen_bgr, (0, 0), fx=scale, fy=scale, interpolation=cv2.INTER_AREA)
        small_tmpl = cv2.resize(bgr, (0, 0), fx=scale, fy=scale, interpolation=cv2.INTER_AREA)
        small_mask = cv2.resize(mask, (0, 0), fx=scale, fy=scale, interpolation=cv2.INTER_NEAREST) if mask is not None else None

        if small_mask is not None:
            res = cv2.matchTemplate(small_screen, small_tmpl, cv2.TM_CCORR_NORMED, mask=small_mask)
        else:
            res = cv2.matchTemplate(small_screen, small_tmpl, cv2.TM_CCOEFF_NORMED)
            
        _, max_val, _, max_loc = cv2.minMaxLoc(res)
        
        if max_val < (min_conf - 0.15):
            return None

        # 2. Fine Pass around the best coarse candidate
        coarse_x = int(max_loc[0] / scale)
        coarse_y = int(max_loc[1] / scale)
        
        pad = 24
        x1 = max(0, coarse_x - pad)
        y1 = max(0, coarse_y - pad)
        x2 = min(sw, coarse_x + tw + pad)
        y2 = min(sh, coarse_y + th + pad)
        
        sub_screen = screen_bgr[y1:y2, x1:x2]
        if sub_screen.shape[0] < th or sub_screen.shape[1] < tw:
            return None

        if mask is not None:
            fine_res = cv2.matchTemplate(sub_screen, bgr, cv2.TM_CCORR_NORMED, mask=mask)
        else:
            fine_res = cv2.matchTemplate(sub_screen, bgr, cv2.TM_CCOEFF_NORMED)
            
        _, fine_val, _, fine_loc = cv2.minMaxLoc(fine_res)
        
        if fine_val >= min_conf:
            cx = x1 + fine_loc[0] + tw // 2
            cy = y1 + fine_loc[1] + th // 2
            return {
                "center": (cx, cy),
                "confidence": float(fine_val),
                "box": (x1 + fine_loc[0], y1 + fine_loc[1], tw, th),
                "method": "template",
            }
        return None

    def detect_circle_by_color(self, screen_bgr, color_filter="purple", min_size=35, max_size=180):
        """
        Universal Circular Contour & Badge Detector.
        Speed: 5-10ms.
        Finds round button shapes with interior white badge icons (hands, x2, coins).
        """
        sh, sw = screen_bgr.shape[:2]
        hsv = cv2.cvtColor(screen_bgr, cv2.COLOR_BGR2HSV)
        
        ranges = {
            "purple": [(125, 75, 75), (165, 255, 255)],
            "gold": [(18, 110, 110), (35, 255, 255)],
            "orange": [(8, 130, 130), (22, 255, 255)],
            "red1": [(0, 120, 90), (10, 255, 255)],
            "red2": [(170, 120, 90), (180, 255, 255)],
            "green": [(35, 90, 90), (85, 255, 255)],
            "cyan": [(85, 110, 90), (105, 255, 255)],
        }
        
        if color_filter == "red":
            m1 = cv2.inRange(hsv, np.array(ranges["red1"][0]), np.array(ranges["red1"][1]))
            m2 = cv2.inRange(hsv, np.array(ranges["red2"][0]), np.array(ranges["red2"][1]))
            mask = cv2.bitwise_or(m1, m2)
        elif color_filter in ranges:
            r = ranges[color_filter]
            mask = cv2.inRange(hsv, np.array(r[0]), np.array(r[1]))
        elif color_filter == "all_circles":
            # Any vibrant circle: purple, gold, orange, red, green, cyan
            masks = []
            for k in ["purple", "gold", "orange", "green", "cyan"]:
                r = ranges[k]
                masks.append(cv2.inRange(hsv, np.array(r[0]), np.array(r[1])))
            r1 = ranges["red1"]
            r2 = ranges["red2"]
            masks.append(cv2.bitwise_or(cv2.inRange(hsv, np.array(r1[0]), np.array(r1[1])), cv2.inRange(hsv, np.array(r2[0]), np.array(r2[1]))))
            mask = masks[0]
            for m in masks[1:]:
                mask = cv2.bitwise_or(mask, m)
        else:
            mask = cv2.inRange(hsv, np.array([0, 80, 70]), np.array([180, 255, 255]))

        contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        candidates = []
        min_area = (min_size * min_size) * 0.45
        max_area = (max_size * max_size) * 1.2
        
        for c in contours:
            area = cv2.contourArea(c)
            if area < min_area or area > max_area:
                continue
            peri = cv2.arcLength(c, True)
            if peri == 0:
                continue
            circularity = 4 * np.pi * area / (peri * peri)
            x, y, w, h = cv2.boundingRect(c)
            
            # Exclude extreme borders
            if y < 15 or (y + h) > (sh - 15) or x < 10 or (x + w) > (sw - 10):
                continue
                
            aspect = w / float(h)
            if 0.60 <= aspect <= 1.60 and circularity >= 0.45:
                # Count white pixels inside bounding box (interior hand icon / x2 / text)
                sub = screen_bgr[y:y+h, x:x+w]
                white_px = int(np.sum((sub[:,:,0] > 185) & (sub[:,:,1] > 185) & (sub[:,:,2] > 185)))
                
                # Real interactive button popup always contains a white icon / text badge!
                if white_px >= 18:
                    score = circularity * 100.0 + min(100.0, (white_px / (w * h)) * 300.0)
                    candidates.append({
                        "center": (x + w // 2, y + h // 2),
                        "confidence": float(circularity),
                        "box": (x, y, w, h),
                        "white_px": white_px,
                        "score": score,
                        "method": "circle_color",
                    })

        if candidates:
            candidates.sort(key=lambda item: item["score"], reverse=True)
            return candidates[0]
        return None

    def detect_popup(self, mode="roblox_auto", threshold=0.70, region=None, exclude_hud=True):
        """
        Unified detector:
        Captures gameplay screen and detects popup button.
        Automatically excludes static HUD menus (Rebirth, Shop, etc.) unless custom region is set.
        Returns result dict and screen_bgr.
        """
        if region is None and exclude_hud:
            mon = self.get_sct().monitors[1]
            sw = mon["width"]
            sh = mon["height"]
            left = mon["left"]
            top = mon["top"]
            # Exclude left menu (Rebirth/Shop: x < 280), right menu (x > sw - 140), topbar (y < 70), bottom hotbar (y > sh - 110)
            rx1 = left + max(280, int(sw * 0.15))
            rx2 = left + min(sw - 140, int(sw * 0.92))
            ry1 = top + max(70, int(sh * 0.07))
            ry2 = top + min(sh - 110, int(sh * 0.89))
            region = (rx1, ry1, rx2, ry2)

        screen_bgr, offset_x, offset_y = self.capture_screen(region)
        result = None
        
        # 1. Check embedded multiplier template first
        if mode in ("roblox_auto", "template", "all_circles") and "roblox_button" in self.templates:
            result = self.match_template_coarse_fine(screen_bgr, self.templates["roblox_button"], min_conf=threshold)
            
        # 2. If no template match or color mode, use HSV circle detector
        if result is None:
            color = "purple" if mode == "roblox_auto" else mode
            result = self.detect_circle_by_color(screen_bgr, color_filter=color)

        if result is not None:
            local_cx, local_cy = result["center"]
            screen_cx = local_cx + offset_x
            screen_cy = local_cy + offset_y
            result["screen_center"] = (screen_cx, screen_cy)
            return result, screen_bgr
        
        return None, screen_bgr
