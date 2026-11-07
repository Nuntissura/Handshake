"""Writes every mock-*.html. Run gen_design_system.py and gen_layouts.py first."""
import importlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

for mod in ("mock_raster", "mock_vector", "mock_video", "mock_design"):
    try:
        m = importlib.import_module(mod)
    except ModuleNotFoundError as e:
        if e.name == mod:
            print("skip", mod)
            continue
        raise
    m.build()
    if mod == "mock_raster":
        m.build("light", "mock-raster-light.html")
