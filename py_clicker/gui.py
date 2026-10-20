import os
import sys
import cv2
import threading
from PIL import Image
import customtkinter as ctk

from win_input import is_admin, restart_as_admin
from engine import ClickerEngine
from detector import imwrite_unicode

class ClickerAppGUI(ctk.CTk):
    def __init__(self):
        super().__init__()

        # Appearance & Window
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")
        self.title("⚡ Автокликер Pro | Авто-охотник")
        self.geometry("820x680")
        self.minsize(780, 620)

        # Set App Icon if exists
        if getattr(sys, 'frozen', False):
            base_dir = getattr(sys, '_MEIPASS', os.path.dirname(sys.executable))
            icon_path = os.path.join(base_dir, "assets", "icon.png")
            if not os.path.exists(icon_path):
                icon_path = os.path.join(os.path.dirname(sys.executable), "assets", "icon.png")
        else:
            base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
            icon_path = os.path.join(base_dir, "assets", "icon.png")

        if os.path.exists(icon_path):
            try:
                self.iconphoto(False, Image.open(icon_path))
            except Exception:
                pass

        # Backend Engine
        self.engine = ClickerEngine()
        self.engine.on_state_changed = self.on_engine_state_changed
        self.engine.on_target_found = self.on_target_found
        self.engine.on_log = self.on_log_msg

        # Variables
        self.mode_var = ctk.StringVar(value="🟣 Roblox x2 (Кружок с рукой)")
        self.sens_var = ctk.DoubleVar(value=70.0)
        self.interval_var = ctk.IntVar(value=35)
        self.hold_var = ctk.IntVar(value=45)
        self.return_cursor_var = ctk.BooleanVar(value=True)
        self.sound_var = ctk.BooleanVar(value=True)

        self.mouse_btn_var = ctk.StringVar(value="Левая (Left)")
        self.mouse_type_var = ctk.StringVar(value="Одиночный")
        self.mouse_interval_var = ctk.IntVar(value=100)
        self.topmost_var = ctk.BooleanVar(value=True)
        self.attributes("-topmost", True)

        self.preview_image_ctk = None

        self.setup_ui()
        self.protocol("WM_DELETE_WINDOW", self.on_closing)

    def setup_ui(self):
        # 1. Header Frame
        header = ctk.CTkFrame(self, corner_radius=10, fg_color="#181a1f")
        header.pack(fill="x", padx=16, pady=(14, 8))

        title_lbl = ctk.CTkLabel(
            header,
            text="⚡ АВТОКЛИКЕР PRO",
            font=ctk.CTkFont(size=20, weight="bold"),
            text_color="#ffffff"
        )
        title_lbl.pack(side="left", padx=16, pady=12)

        subtitle_lbl = ctk.CTkLabel(
            header,
            text="Умный поиск всплывающих кнопок в Roblox & DirectX играх",
            font=ctk.CTkFont(size=12),
            text_color="#8b949e"
        )
        subtitle_lbl.pack(side="left", padx=6, pady=12)

        # Admin Badge
        admin_text = "🛡️ Администратор" if is_admin() else "⚠️ Обычный режим"
        admin_color = "#238636" if is_admin() else "#d29922"
        self.admin_lbl = ctk.CTkLabel(
            header,
            text=admin_text,
            font=ctk.CTkFont(size=11, weight="bold"),
            text_color="#ffffff",
            fg_color=admin_color,
            corner_radius=6,
            padx=10,
            pady=4
        )
        self.admin_lbl.pack(side="right", padx=16, pady=12)

        # Topmost Checkbox
        self.topmost_cb = ctk.CTkCheckBox(
            header,
            text="📌 Поверх всех окон",
            variable=self.topmost_var,
            command=self.toggle_topmost,
            font=ctk.CTkFont(size=12)
        )
        self.topmost_cb.pack(side="right", padx=8, pady=12)

        # 2. Tabs
        self.tabs = ctk.CTkTabview(self, corner_radius=10, fg_color="#181a1f")
        self.tabs.pack(fill="both", expand=True, padx=16, pady=6)

        self.tab_hunter = self.tabs.add("🎯 Авто-охотник")
        self.tab_mouse = self.tabs.add("🖱️ Обычный кликер")
        self.tab_settings = self.tabs.add("⚙️ Настройки")

        self.setup_hunter_tab()
        self.setup_mouse_tab()
        self.setup_settings_tab()

        # 3. Bottom Log / Status Bar
        log_frame = ctk.CTkFrame(self, corner_radius=8, fg_color="#101216", height=90)
        log_frame.pack(fill="x", padx=16, pady=(6, 12))

        log_title = ctk.CTkLabel(
            log_frame,
            text="📋 Журнал событий:",
            font=ctk.CTkFont(size=11, weight="bold"),
            text_color="#8b949e"
        )
        log_title.pack(anchor="w", padx=10, pady=(6, 0))

        self.log_textbox = ctk.CTkTextbox(
            log_frame,
            height=60,
            font=ctk.CTkFont(family="Consolas", size=11),
            fg_color="#0d1117",
            text_color="#58a6ff",
            activate_scrollbars=True
        )
        self.log_textbox.pack(fill="both", expand=True, padx=8, pady=(2, 6))
        self.log_textbox.insert("end", "🟢 Автокликер Pro готов к работе. Нажмите [F6] для охоты за кружками.\n")

    # =========================================================================
    # TAB 1: AUTO-HUNTER
    # =========================================================================
    def setup_hunter_tab(self):
        container = self.tab_hunter

        # Big Action Button & Status
        action_box = ctk.CTkFrame(container, fg_color="#21262d", corner_radius=8)
        action_box.pack(fill="x", padx=8, pady=8)

        self.hunter_btn = ctk.CTkButton(
            action_box,
            text="🚀 ЗАПУСТИТЬ ОХОТНИКА (F6)",
            font=ctk.CTkFont(size=16, weight="bold"),
            height=46,
            fg_color="#238636",
            hover_color="#2ea043",
            command=self.toggle_hunter_btn
        )
        self.hunter_btn.pack(side="left", fill="x", expand=True, padx=12, pady=10)

        self.test_btn = ctk.CTkButton(
            action_box,
            text="🔍 Проверить экран прямо сейчас",
            font=ctk.CTkFont(size=13, weight="bold"),
            height=46,
            fg_color="#1f6feb",
            hover_color="#388bfd",
            command=self.run_test_scan
        )
        self.test_btn.pack(side="right", padx=12, pady=10)

        # Settings Grid
        grid_frame = ctk.CTkFrame(container, fg_color="transparent")
        grid_frame.pack(fill="both", expand=True, padx=4, pady=4)

        left_col = ctk.CTkFrame(grid_frame, fg_color="#161b22", corner_radius=8)
        left_col.pack(side="left", fill="both", expand=True, padx=(0, 6), pady=0)

        right_col = ctk.CTkFrame(grid_frame, fg_color="#161b22", corner_radius=8)
        right_col.pack(side="right", fill="both", expand=True, padx=(6, 0), pady=0)

        # --- Left Column: Target & Sensitivity ---
        ctk.CTkLabel(left_col, text="🎯 Цель и чувствительность", font=ctk.CTkFont(size=14, weight="bold")).pack(anchor="w", padx=12, pady=(10, 4))

        ctk.CTkLabel(left_col, text="Что искать на экране:").pack(anchor="w", padx=12, pady=(4, 0))
        self.mode_menu = ctk.CTkOptionMenu(
            left_col,
            values=[
                "🟣 Roblox x2 (Кружок с рукой)",
                "🟡 Золотые кружки / Монеты",
                "🟠 Оранжевые кружки",
                "🔴 Красные кружки",
                "🟢 Зеленые кружки",
                "🔵 Голубые кружки",
                "🌈 Любой яркий кружок",
            ],
            variable=self.mode_var,
            command=self.on_mode_changed,
            height=32
        )
        self.mode_menu.pack(fill="x", padx=12, pady=(2, 10))

        # Sensitivity Slider
        self.sens_lbl = ctk.CTkLabel(left_col, text=f"Порог уверенности: {int(self.sens_var.get())}%")
        self.sens_lbl.pack(anchor="w", padx=12, pady=(2, 0))
        self.sens_slider = ctk.CTkSlider(
            left_col,
            from_=50.0,
            to=95.0,
            number_of_steps=45,
            variable=self.sens_var,
            command=lambda val: self.sens_lbl.configure(text=f"Порог уверенности: {int(val)}%")
        )
        self.sens_slider.pack(fill="x", padx=12, pady=(2, 10))

        # Scan Interval
        self.int_lbl = ctk.CTkLabel(left_col, text=f"Частота сканирования: {self.interval_var.get()} мс (~{int(1000/max(1,self.interval_var.get()))} кадров/сек)")
        self.int_lbl.pack(anchor="w", padx=12, pady=(2, 0))
        self.int_slider = ctk.CTkSlider(
            left_col,
            from_=15,
            to=250,
            number_of_steps=47,
            variable=self.interval_var,
            command=lambda val: self.int_lbl.configure(text=f"Частота сканирования: {int(val)} мс (~{int(1000/max(1,int(val)))} кадров/сек)")
        )
        self.int_slider.pack(fill="x", padx=12, pady=(2, 10))

        # Hold Duration
        self.hold_lbl = ctk.CTkLabel(left_col, text=f"Удержание клика в игре: {self.hold_var.get()} мс")
        self.hold_lbl.pack(anchor="w", padx=12, pady=(2, 0))
        self.hold_slider = ctk.CTkSlider(
            left_col,
            from_=25,
            to=150,
            number_of_steps=25,
            variable=self.hold_var,
            command=lambda val: self.hold_lbl.configure(text=f"Удержание клика в игре: {int(val)} мс")
        )
        self.hold_slider.pack(fill="x", padx=12, pady=(2, 10))

        # Checkboxes
        ctk.CTkCheckBox(left_col, text="Возвращать курсор на место после клика", variable=self.return_cursor_var).pack(anchor="w", padx=12, pady=6)
        ctk.CTkCheckBox(left_col, text="Звуковые сигналы (Бип при клике и переключении)", variable=self.sound_var).pack(anchor="w", padx=12, pady=6)

        # --- Right Column: Diagnostics & Preview ---
        ctk.CTkLabel(right_col, text="📊 Диагностика и превью", font=ctk.CTkFont(size=14, weight="bold")).pack(anchor="w", padx=12, pady=(10, 4))

        self.stats_lbl = ctk.CTkLabel(
            right_col,
            text="🎯 Кликнуто целей: 0\n📍 Последняя цель: (X: —, Y: —)",
            font=ctk.CTkFont(size=13),
            justify="left"
        )
        self.stats_lbl.pack(anchor="w", padx=12, pady=(4, 8))

        # Preview Frame for Test Scan
        self.preview_frame = ctk.CTkFrame(right_col, fg_color="#0d1117", corner_radius=6, height=130)
        self.preview_frame.pack(fill="both", expand=True, padx=12, pady=(2, 8))

        self.preview_lbl = ctk.CTkLabel(
            self.preview_frame,
            text="Нажмите [🔍 Проверить экран],\nчтобы увидеть найденную кнопку.",
            text_color="#8b949e",
            font=ctk.CTkFont(size=12)
        )
        self.preview_lbl.pack(expand=True, pady=10)

        self.test_status_lbl = ctk.CTkLabel(
            right_col,
            text="ℹ️ Нажмите кнопку проверки или F6 в игре.",
            font=ctk.CTkFont(size=11),
            text_color="#8b949e",
            wraplength=340,
            justify="left"
        )
        self.test_status_lbl.pack(anchor="w", padx=12, pady=(0, 8))

    # =========================================================================
    # TAB 2: NORMAL MOUSE CLICKER
    # =========================================================================
    def setup_mouse_tab(self):
        container = self.tab_mouse

        box = ctk.CTkFrame(container, fg_color="#161b22", corner_radius=8)
        box.pack(fill="both", expand=True, padx=12, pady=12)

        ctk.CTkLabel(box, text="🖱️ Настройки обычного автокликера", font=ctk.CTkFont(size=16, weight="bold")).pack(anchor="w", padx=16, pady=(16, 8))

        # Button row
        row1 = ctk.CTkFrame(box, fg_color="transparent")
        row1.pack(fill="x", padx=16, pady=8)
        ctk.CTkLabel(row1, text="Кнопка мыши:").pack(side="left", padx=(0, 10))
        ctk.CTkOptionMenu(
            row1,
            values=["Левая (Left)", "Правая (Right)", "Средняя (Middle)"],
            variable=self.mouse_btn_var,
            width=180
        ).pack(side="left")

        # Type row
        row2 = ctk.CTkFrame(box, fg_color="transparent")
        row2.pack(fill="x", padx=16, pady=8)
        ctk.CTkLabel(row2, text="Тип клика:   ").pack(side="left", padx=(0, 10))
        ctk.CTkOptionMenu(
            row2,
            values=["Одиночный", "Двойной"],
            variable=self.mouse_type_var,
            width=180
        ).pack(side="left")

        # Interval slider
        row3 = ctk.CTkFrame(box, fg_color="transparent")
        row3.pack(fill="x", padx=16, pady=8)
        self.mouse_int_lbl = ctk.CTkLabel(row3, text=f"Интервал: {self.mouse_interval_var.get()} мс ({int(1000/max(1,self.mouse_interval_var.get()))} CPS)")
        self.mouse_int_lbl.pack(anchor="w")
        ctk.CTkSlider(
            box,
            from_=5,
            to=500,
            number_of_steps=99,
            variable=self.mouse_interval_var,
            command=lambda val: self.mouse_int_lbl.configure(text=f"Интервал: {int(val)} мс ({int(1000/max(1,int(val)))} CPS)")
        ).pack(fill="x", padx=16, pady=(2, 12))

        self.mouse_toggle_btn = ctk.CTkButton(
            box,
            text="▶ ЗАПУСТИТЬ КЛИКЕР (F7)",
            font=ctk.CTkFont(size=15, weight="bold"),
            height=44,
            fg_color="#238636",
            hover_color="#2ea043",
            command=self.toggle_mouse_btn
        )
        self.mouse_toggle_btn.pack(fill="x", padx=16, pady=16)

    # =========================================================================
    # TAB 3: SETTINGS
    # =========================================================================
    def setup_settings_tab(self):
        container = self.tab_settings

        box = ctk.CTkFrame(container, fg_color="#161b22", corner_radius=8)
        box.pack(fill="both", expand=True, padx=12, pady=12)

        ctk.CTkLabel(box, text="⚙️ Горячие клавиши и параметры", font=ctk.CTkFont(size=16, weight="bold")).pack(anchor="w", padx=16, pady=(16, 8))

        ctk.CTkLabel(box, text="• F6 — Старт / Стоп Авто-охотника за всплывающими кружками\n• F7 — Старт / Стоп Обычного автокликера мыши\n• F12 — Экстренная аварийная остановка (Panic Key)", justify="left", font=ctk.CTkFont(size=13)).pack(anchor="w", padx=16, pady=8)

        if not is_admin():
            admin_card = ctk.CTkFrame(box, fg_color="#302217", corner_radius=6)
            admin_card.pack(fill="x", padx=16, pady=16)
            ctk.CTkLabel(
                admin_card,
                text="⚠️ Приложение запущено без прав Администратора.\nНекоторые игры с античитом блокируют клики от обычных программ.",
                text_color="#e3b341",
                justify="left"
            ).pack(side="left", padx=12, pady=12)

            ctk.CTkButton(
                admin_card,
                text="Перезапустить как Админ",
                fg_color="#d29922",
                hover_color="#bb8009",
                command=restart_as_admin
            ).pack(side="right", padx=12, pady=12)

    # =========================================================================
    # ACTIONS & LOGIC
    # =========================================================================
    def toggle_topmost(self):
        self.attributes("-topmost", self.topmost_var.get())
    def on_mode_changed(self, choice):
        mapping = {
            "🟣 Roblox x2 (Кружок с рукой)": "roblox_auto",
            "🟡 Золотые кружки / Монеты": "gold",
            "🟠 Оранжевые кружки": "orange",
            "🔴 Красные кружки": "red",
            "🟢 Зеленые кружки": "green",
            "🔵 Голубые кружки": "cyan",
            "🌈 Любой яркий кружок": "all_circles",
        }
        self.engine.hunter_mode = mapping.get(choice, "roblox_auto")
        self.engine.log(f"Режим охотника изменен на: {self.engine.hunter_mode}")

    def apply_config_to_engine(self):
        self.engine.hunter_threshold = self.sens_var.get() / 100.0
        self.engine.hunter_interval_ms = self.interval_var.get()
        self.engine.hunter_hold_ms = self.hold_var.get()
        self.engine.hunter_return_cursor = self.return_cursor_var.get()
        self.engine.hunter_sound = self.sound_var.get()

        btn_map = {"Левая (Left)": "left", "Правая (Right)": "right", "Средняя (Middle)": "middle"}
        self.engine.mouse_button = btn_map.get(self.mouse_btn_var.get(), "left")
        self.engine.mouse_click_type = "double" if self.mouse_type_var.get() == "Двойной" else "single"
        self.engine.mouse_interval_ms = self.mouse_interval_var.get()

    def toggle_hunter_btn(self):
        self.apply_config_to_engine()
        self.engine.toggle_hunter()

    def toggle_mouse_btn(self):
        self.apply_config_to_engine()
        self.engine.toggle_mouse()

    def on_engine_state_changed(self):
        # Update UI according to active state
        self.after(0, self._sync_ui_state)

    def _sync_ui_state(self):
        if self.engine.is_hunter_active:
            self.hunter_btn.configure(
                text="⏹️ ОСТАНОВИТЬ ОХОТНИКА (F6)",
                fg_color="#da3633",
                hover_color="#f85149"
            )
        else:
            self.hunter_btn.configure(
                text="🚀 ЗАПУСТИТЬ ОХОТНИКА (F6)",
                fg_color="#238636",
                hover_color="#2ea043"
            )

        if self.engine.is_mouse_active:
            self.mouse_toggle_btn.configure(
                text="⏹️ ОСТАНОВИТЬ КЛИКЕР (F7)",
                fg_color="#da3633",
                hover_color="#f85149"
            )
        else:
            self.mouse_toggle_btn.configure(
                text="▶ ЗАПУСТИТЬ КЛИКЕР (F7)",
                fg_color="#238636",
                hover_color="#2ea043"
            )

    def on_target_found(self, tx, ty, method, conf):
        self.after(0, lambda: self._update_stats_ui(tx, ty, method, conf))

    def _update_stats_ui(self, tx, ty, method, conf):
        self.stats_lbl.configure(
            text=f"🎯 Кликнуто целей: {self.engine.total_hunter_clicks}\n📍 Последняя цель: (X: {tx}, Y: {ty})\nМетод: {method} ({int(conf*100)}%)"
        )

    def on_log_msg(self, msg):
        self.after(0, lambda: self._append_log(msg))

    def _append_log(self, msg):
        self.log_textbox.insert("end", f"{msg}\n")
        self.log_textbox.see("end")

    def run_test_scan(self):
        self.apply_config_to_engine()
        self.test_status_lbl.configure(text="⏳ Сканирование экрана...", text_color="#58a6ff")
        threading.Thread(target=self._test_scan_worker, daemon=True).start()

    def _test_scan_worker(self):
        res, screen_bgr = self.engine.detector.detect_popup(
            mode=self.engine.hunter_mode,
            threshold=self.engine.hunter_threshold,
            region=self.engine.hunter_region
        )
        self.after(0, lambda: self._display_test_result(res, screen_bgr))

    def _display_test_result(self, res, screen_bgr):
        if res:
            tx, ty = res["screen_center"]
            conf = res["confidence"]
            method = res["method"]
            x, y, w, h = res["box"]

            self.test_status_lbl.configure(
                text=f"🟢 КРУЖОК НАЙДЕН!\nКоординаты: (X: {tx}, Y: {ty}) | Метод: {method} | Уверенность: {int(conf*100)}%",
                text_color="#3fb950"
            )
            self._append_log(f"🔍 [Тест] Кружок успешно найден в точке ({tx}, {ty})!")

            # Crop target for preview
            pad = 12
            y1 = max(0, y - pad)
            y2 = min(screen_bgr.shape[0], y + h + pad)
            x1 = max(0, x - pad)
            x2 = min(screen_bgr.shape[1], x + w + pad)
            crop = screen_bgr[y1:y2, x1:x2]

            # Draw green circle overlay
            cv2.circle(crop, (tx - x1, ty - y1), 8, (0, 255, 0), 2)
            crop_rgb = cv2.cvtColor(crop, cv2.COLOR_BGR2RGB)
            pil_img = Image.fromarray(crop_rgb)

            # Resize to fit preview box
            pil_img.thumbnail((120, 100))
            self.preview_image_ctk = ctk.CTkImage(light_image=pil_img, dark_image=pil_img, size=pil_img.size)
            self.preview_lbl.configure(image=self.preview_image_ctk, text="")

        else:
            self.test_status_lbl.configure(
                text="ℹ️ На текущем экране кружок не обнаружен.\nУбедитесь, что окно игры открыто и кружок виден на экране.",
                text_color="#f85149"
            )
            self.preview_lbl.configure(image=None, text="Кружок не найден на экране.")
            self._append_log("🔍 [Тест] Кружок не найден на текущем экране.")

    def on_closing(self):
        self.engine.stop_event.set()
        self.engine.stop_hunter()
        self.engine.stop_mouse()
        self.destroy()
