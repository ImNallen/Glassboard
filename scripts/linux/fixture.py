#!/usr/bin/env python3
import json
import sys
from pathlib import Path

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, Gtk

state_path = Path(sys.argv[1])
state = {"clicks": [], "size": None}


def persist():
    temporary = state_path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state))
    temporary.replace(state_path)


def draw(widget, context):
    width, height = widget.get_allocated_width(), widget.get_allocated_height()
    state["size"] = [width, height]
    persist()
    context.set_source_rgb(0.94, 0.96, 0.98)
    context.paint()
    for index, color in enumerate(((0.15, 0.50, 0.85), (0.10, 0.65, 0.35), (0.90, 0.70, 0.10))):
        context.set_source_rgb(*color)
        context.rectangle(width * (0.1 + index * 0.25), height * 0.6, width * 0.15, height * 0.18)
        context.fill()


def clicked(widget, event):
    state["clicks"].append([event.x, event.y, event.button])
    persist()


window = Gtk.Window(title="Glassboard Linux fixture")
window.set_default_size(850, 450)
window.connect("destroy", Gtk.main_quit)
canvas = Gtk.DrawingArea()
canvas.add_events(Gdk.EventMask.BUTTON_PRESS_MASK)
canvas.connect("draw", draw)
canvas.connect("button-press-event", clicked)
window.add(canvas)
window.show_all()
Gtk.main()
