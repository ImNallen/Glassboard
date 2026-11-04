#!/usr/bin/env python3
"""An independent app for Glassboard to draw over. It counts the clicks it receives."""
import sys
import tkinter
from pathlib import Path

clicks_file = Path(sys.argv[1])
clicks = 0
clicks_file.write_text("0")

root = tkinter.Tk()
root.title("Glassboard Windows fixture")
root.geometry("800x500+40+40")
root.resizable(False, False)
canvas = tkinter.Canvas(root, width=800, height=500, background="#ffffff", highlightthickness=0)
canvas.pack()
for index, color in enumerate(("#d8e8f8", "#f8e8d8", "#e0f4e0", "#f0e0f4")):
    canvas.create_rectangle(0, index * 125, 800, (index + 1) * 125, fill=color, outline="")


def clicked(_event):
    global clicks
    clicks += 1
    clicks_file.write_text(str(clicks))


canvas.bind("<Button-1>", clicked)
root.mainloop()
