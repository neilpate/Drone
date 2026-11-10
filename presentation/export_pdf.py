"""Export reveal.js slides to PDF by rendering each slide as a standalone page.

Bypasses reveal.js entirely — extracts each <section> and wraps it in the
same CSS, so headless Chrome screenshots a static page with no navigation
timing issues.
"""

import re
import subprocess
from pathlib import Path

SLIDES_DIR = Path(r"C:\dev\Misc\Drone\presentation")
HTML_FILE = SLIDES_DIR / "index.html"
OUTPUT_PDF = SLIDES_DIR / "slides.pdf"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
WIDTH = 1280
HEIGHT = 720
SCALE = 2

html = HTML_FILE.read_text(encoding="utf-8")

# Extract the <style> block
style_match = re.search(r"(<style>.*?</style>)", html, re.DOTALL)
style = style_match.group(1) if style_match else ""

# Extract top-level <section> blocks (slides)
# reveal.js uses <section> for slides; nested <section> means vertical slides
sections = re.findall(r"<section[^>]*>(.*?)</section>", html, re.DOTALL)

print(f"Found {len(sections)} slides")

screenshots = []

for i, content in enumerate(sections):
    # Check for data-background-color on the section tag
    bg_match = re.search(
        rf'<section[^>]*data-background-color="([^"]*)"',
        html,
    )
    # Re-extract with attributes to get per-slide bg
    section_tags = re.findall(r"(<section[^>]*>)", html)
    bg_color = "#ffffff"
    if i < len(section_tags):
        bg = re.search(r'data-background-color="([^"]*)"', section_tags[i])
        if bg:
            bg_color = bg.group(1)

    static_html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@5.1.0/dist/reveal.css">
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@5.1.0/dist/theme/white.css">
{style}
<style>
  html, body {{
    margin: 0; padding: 0;
    width: {WIDTH}px; height: {HEIGHT}px;
    overflow: hidden;
    background: {bg_color};
  }}
  .slide-container {{
    width: {WIDTH}px; height: {HEIGHT}px;
    display: flex; align-items: center; justify-content: center;
    box-sizing: border-box;
    padding: 40px 60px;
  }}
  .slide-content {{
    width: 100%;
    text-align: center;
    font-family: "Source Sans Pro", Helvetica, sans-serif;
    font-size: 32px;
    color: #2d2d2d;
  }}
  .slide-content h1 {{ font-size: 1.8em; font-weight: 700; color: #1a1a2e; }}
  .slide-content h2 {{ font-size: 1.4em; font-weight: 700; color: #1a1a2e; margin-bottom: 0.6em; }}
  .slide-content h3 {{ font-size: 1.1em; font-weight: 600; color: #555; }}
  .slide-content ul {{ text-align: left; display: inline-block; }}
  .slide-content li {{ margin-bottom: 0.4em; line-height: 1.4; }}
  .slide-content img {{ max-height: 55vh; border-radius: 6px; box-shadow: 0 4px 20px rgba(0,0,0,0.12); }}
</style>
</head>
<body>
<div class="slide-container">
  <div class="slide-content">
    {content}
  </div>
</div>
</body>
</html>"""

    page_path = SLIDES_DIR / f"_page_{i:02d}.html"
    page_path.write_text(static_html, encoding="utf-8")

    png_path = SLIDES_DIR / f"_slide_{i:02d}.png"

    subprocess.run([
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--hide-scrollbars",
        f"--window-size={WIDTH},{HEIGHT}",
        f"--force-device-scale-factor={SCALE}",
        f"--screenshot={png_path}",
        "--virtual-time-budget=5000",
        "--run-all-compositor-stages-before-draw",
        f"file:///{page_path.as_posix()}",
    ], check=True, capture_output=True)

    page_path.unlink()

    if png_path.exists() and png_path.stat().st_size > 5000:
        screenshots.append(png_path)
        print(f"  captured slide {i}")
    else:
        print(f"  FAILED slide {i}")
        if png_path.exists():
            png_path.unlink()

# Assemble into PDF
from PIL import Image

images = []
for p in screenshots:
    img = Image.open(p).convert("RGB")
    images.append(img)

if images:
    images[0].save(
        OUTPUT_PDF,
        save_all=True,
        append_images=images[1:],
        resolution=150,
    )
    print(f"\nPDF written: {OUTPUT_PDF} ({len(images)} pages)")

# Clean up temp PNGs
for p in screenshots:
    p.unlink()

print("Done.")
