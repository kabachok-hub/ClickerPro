import os
import sys
import ctypes

def main():
    # Set Windows Process DPI Awareness
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2) # Per monitor DPI aware
    except Exception:
        try:
            ctypes.windll.user32.SetProcessDPIAware()
        except Exception:
            pass

    from gui import ClickerAppGUI
    app = ClickerAppGUI()
    app.mainloop()

if __name__ == "__main__":
    main()
