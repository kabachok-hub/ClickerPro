import sys
import os

# Ensure the root directory is on the python search path
ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
if ROOT_DIR not in sys.path:
    sys.path.insert(0, ROOT_DIR)

from src.ui.app import UltimateClickerApp

def main():
    app = UltimateClickerApp()
    app.mainloop()

if __name__ == "__main__":
    main()
