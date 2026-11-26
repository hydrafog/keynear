#!/usr/bin/env python3
"""Render light and dark Coverflow phone-cutout PNGs for README.md."""

from __future__ import annotations

import base64
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCREENSHOTS = ROOT / "design" / "website" / "public" / "screenshots"
BRAND = ROOT / "assets" / "brand"

HTML_TEMPLATE = """<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  html, body {{
    width: 1600px;
    height: 820px;
    background: transparent;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }}
  .stage {{
    position: relative;
    width: 1600px;
    height: 820px;
    display: flex;
    align-items: center;
    justify-content: center;
    perspective: 1850px;
    perspective-origin: 50% 48%;
  }}
  .aura {{
    position: absolute;
    width: 820px;
    height: 620px;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(ellipse at center,
      {aura_inner} 0%,
      {aura_mid} 44%,
      rgba(74, 187, 213, 0) 74%);
    filter: blur(26px);
    pointer-events: none;
    z-index: 0;
  }}
  .floor-glow {{
    position: absolute;
    width: 1080px;
    height: 110px;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    background: radial-gradient(ellipse at center,
      {floor_shadow} 0%,
      rgba(0, 0, 0, 0) 72%);
    filter: blur(18px);
    z-index: 1;
  }}
  .phone-wrap {{
    position: absolute;
    transform-style: preserve-3d;
  }}
  .phone-wrap.left {{
    transform: translateX(-430px) translateY(20px) translateZ(-120px) rotateY(20deg) scale(1.05);
    z-index: 2;
    filter: drop-shadow(-20px 30px 36px {side_shadow});
  }}
  .phone-wrap.center {{
    transform: translateX(0px) translateY(-4px) translateZ(80px) rotateY(0deg) scale(1.0);
    z-index: 5;
    filter: drop-shadow(0px 34px 44px {center_shadow}) drop-shadow(0px 10px 18px {center_shadow_tight});
  }}
  .phone-wrap.right {{
    transform: translateX(430px) translateY(20px) translateZ(-120px) rotateY(-20deg) scale(1.05);
    z-index: 2;
    filter: drop-shadow(20px 30px 36px {side_shadow});
  }}
  .phone {{
    position: relative;
    width: 326px;
    height: 678px;
    border-radius: 48px;
    padding: 11px;
    background: linear-gradient(155deg, {chassis_hi} 0%, {chassis_mid} 46%, {chassis_lo} 100%);
    box-shadow:
      inset 0 0 0 1.5px {rim_outer},
      inset 0 2px 3px {rim_top},
      inset 0 -2px 4px rgba(0, 0, 0, 0.45);
  }}
  /* Hardware side buttons (power & volume) */
  .phone::before {{
    content: "";
    position: absolute;
    right: -3.5px;
    top: 152px;
    width: 3.5px;
    height: 54px;
    border-radius: 0 3px 3px 0;
    background: linear-gradient(180deg, {chassis_hi}, {chassis_lo});
  }}
  .phone::after {{
    content: "";
    position: absolute;
    right: -3.5px;
    top: 226px;
    width: 3.5px;
    height: 92px;
    border-radius: 0 3px 3px 0;
    background: linear-gradient(180deg, {chassis_hi}, {chassis_lo});
  }}
  .screen-bezel {{
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 38px;
    overflow: hidden;
    background: #070A0D;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }}
  .screen-bezel img {{
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }}
  /* Subtle glass sheen on angled phones */
  .phone-wrap.left .screen-bezel::after {{
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(108deg, rgba(255,255,255,{side_glare}) 0%, rgba(255,255,255,0) 42%, rgba(0,0,0,{side_shade}) 100%);
    pointer-events: none;
  }}
  .phone-wrap.right .screen-bezel::after {{
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(-108deg, rgba(255,255,255,{side_glare}) 0%, rgba(255,255,255,0) 42%, rgba(0,0,0,{side_shade}) 100%);
    pointer-events: none;
  }}
  .phone-wrap.center .screen-bezel::after {{
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(150deg, rgba(255,255,255,{center_glare}) 0%, rgba(255,255,255,0) 36%);
    pointer-events: none;
  }}
  /* Front camera punch-hole */
  .camera {{
    position: absolute;
    top: 11px;
    left: 50%;
    transform: translateX(-50%);
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 35%, #2A343D 0%, #090C10 62%, #000000 100%);
    box-shadow: 0 0 0 1.2px rgba(255, 255, 255, 0.08);
    z-index: 4;
  }}
</style>
</head>
<body>
  <div class="stage">
    <div class="aura"></div>
    <div class="floor-glow"></div>

    <div class="phone-wrap left">
      <div class="phone">
        <div class="screen-bezel">
          <div class="camera"></div>
          <img src="{left_uri}" alt="Send screen">
        </div>
      </div>
    </div>

    <div class="phone-wrap center">
      <div class="phone">
        <div class="screen-bezel">
          <div class="camera"></div>
          <img src="{center_uri}" alt="Receive screen">
        </div>
      </div>
    </div>

    <div class="phone-wrap right">
      <div class="phone">
        <div class="screen-bezel">
          <div class="camera"></div>
          <img src="{right_uri}" alt="Settings screen">
        </div>
      </div>
    </div>
  </div>
</body>
</html>
"""

THEMES = {
    "light": {
        "aura_inner": "rgba(74, 187, 213, 0.18)",
        "aura_mid": "rgba(74, 187, 213, 0.06)",
        "floor_shadow": "rgba(7, 26, 33, 0.24)",
        "side_shadow": "rgba(7, 26, 33, 0.28)",
        "center_shadow": "rgba(7, 26, 33, 0.34)",
        "center_shadow_tight": "rgba(14, 116, 138, 0.16)",
        "chassis_hi": "#34414B",
        "chassis_mid": "#1D262D",
        "chassis_lo": "#10161B",
        "rim_outer": "rgba(255, 255, 255, 0.24)",
        "rim_top": "rgba(255, 255, 255, 0.30)",
        "side_glare": "0.14",
        "side_shade": "0.08",
        "center_glare": "0.07",
    },
    "dark": {
        "aura_inner": "rgba(74, 187, 213, 0.20)",
        "aura_mid": "rgba(74, 187, 213, 0.06)",
        "floor_shadow": "rgba(0, 0, 0, 0.65)",
        "side_shadow": "rgba(0, 0, 0, 0.72)",
        "center_shadow": "rgba(0, 0, 0, 0.80)",
        "center_shadow_tight": "rgba(74, 187, 213, 0.18)",
        "chassis_hi": "#3B4954",
        "chassis_mid": "#212A32",
        "chassis_lo": "#12171C",
        "rim_outer": "rgba(170, 242, 255, 0.22)",
        "rim_top": "rgba(255, 255, 255, 0.24)",
        "side_glare": "0.10",
        "side_shade": "0.16",
        "center_glare": "0.05",
    },
}


def data_uri(path: pathlib.Path) -> str:
    b64 = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:image/png;base64,{b64}"


def render_theme(theme: str) -> pathlib.Path:
    out_path = BRAND / f"screenshots-coverflow-{theme}.png"
    html = HTML_TEMPLATE.format(
        left_uri=data_uri(SCREENSHOTS / f"send-{theme}.png"),
        center_uri=data_uri(SCREENSHOTS / f"receive-{theme}.png"),
        right_uri=data_uri(SCREENSHOTS / f"settings-{theme}.png"),
        **THEMES[theme],
    )
    with tempfile.TemporaryDirectory() as tmp:
        html_path = pathlib.Path(tmp) / f"coverflow-{theme}.html"
        html_path.write_text(html, encoding="utf-8")
        subprocess.run(
            [
                "google-chrome",
                "--headless=new",
                f"--user-data-dir={tmp}/profile",
                "--no-sandbox",
                "--disable-gpu",
                "--hide-scrollbars",
                "--default-background-color=00000000",
                "--force-device-scale-factor=2",
                "--window-size=1600,820",
                f"--screenshot={out_path}",
                html_path.as_uri(),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    # Downsample 3200x1640 -> 1600x820 with Lanczos filter for crisp supersampled edges.
    subprocess.run(
        [
            "magick",
            str(out_path),
            "-filter",
            "Lanczos",
            "-resize",
            "1600x820",
            "-strip",
            str(out_path),
        ],
        check=True,
    )
    return out_path


def main() -> None:
    for theme in ("light", "dark"):
        out = render_theme(theme)
        print(f"rendered {out.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
