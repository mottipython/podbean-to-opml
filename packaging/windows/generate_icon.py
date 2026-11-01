#!/usr/bin/env python3
"""
generate_icon.py - Generates a modern Windows .ico file for Podcast Rescue
"""

import os
from PIL import Image, ImageDraw

def create_podcast_icon(path="icon.ico"):
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    images = []

    for w, h in sizes:
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)

        # Background rounded circle (emerald/indigo gradient tone)
        margin = max(1, w // 16)
        draw.ellipse([margin, margin, w - margin, h - margin], fill=(16, 185, 129, 255), outline=(5, 150, 105, 255), width=max(1, w // 32))

        # Center microphone / broadcast symbol
        cw, ch = w // 2, h // 2
        r = w // 5

        # Inner broadcast dot
        draw.ellipse([cw - r // 2, ch - r // 2, cw + r // 2, ch + r // 2], fill=(255, 255, 255, 255))

        # Broadcast arcs
        arc_width = max(1, w // 24)
        if w >= 32:
            draw.arc([cw - r, ch - r, cw + r, ch + r], start=210, end=330, fill=(255, 255, 255, 220), width=arc_width)
            draw.arc([cw - r * 1.5, ch - r * 1.5, cw + r * 1.5, ch + r * 1.5], start=210, end=330, fill=(255, 255, 255, 180), width=arc_width)

        images.append(img)

    images[0].save(
        path,
        format="ICO",
        sizes=[(im.width, im.height) for im in images],
        append_images=images[1:]
    )
    print(f"[+] Windows icon generated: {path}")

if __name__ == "__main__":
    os.makedirs(os.path.dirname(os.path.abspath(__file__)), exist_ok=True)
    create_podcast_icon(os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.ico"))
