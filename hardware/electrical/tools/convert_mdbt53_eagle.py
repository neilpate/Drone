"""Convert Raytac's Eagle MDBT53 package to a KiCad footprint."""

from __future__ import annotations

import argparse
import xml.etree.ElementTree as ET
from pathlib import Path


def layer_name(eagle_layer: str) -> str:
    return {
        "1": "F.SilkS",
        "21": "F.SilkS",
        "25": "F.SilkS",
        "27": "F.Fab",
        "41": "F.CrtYd",
    }.get(eagle_layer, "F.Fab")


def flip_y(value: str | None) -> str:
    """Eagle measures Y upwards, KiCad downwards; copying verbatim mirrors the part."""
    return f"{-float(value or 0):g}"


def convert(source: Path, destination: Path) -> int:
    root = ET.parse(source).getroot()
    package = root.find("./drawing/library/packages/package[@name='MDBT53']")
    if package is None:
        raise ValueError("Eagle library does not contain package MDBT53")

    pads = package.findall("smd")
    if len(pads) != 65:
        raise ValueError(f"expected 65 MDBT53 pads, found {len(pads)}")

    lines = [
        '(footprint "drone_fc_v2_footprints:MDBT53-P1M" (version 20240108) (generator "convert_mdbt53_eagle.py")',
        '  (layer "F.Cu")',
        '  (descr "Raytac MDBT53-P1M; converted from manufacturer Eagle library")',
        '  (tags "Raytac MDBT53-P1M nRF5340 65-pad 48-GPIO")',
        '  (property "Reference" "REF**" (at 0 -9.5 0) (layer "F.SilkS")',
        '    (effects (font (size 1 1) (thickness 0.15)))',
        '  )',
        '  (property "Value" "MDBT53-P1M" (at 0 9.5 0) (layer "F.Fab")',
        '    (effects (font (size 1 1) (thickness 0.15)))',
        '  )',
        '  (attr smd)',
    ]

    for wire in package.findall("wire"):
        if wire.get("layer") != "21":
            continue
        layer = layer_name(wire.get("layer", ""))
        width = float(wire.get("width", "0.127"))
        lines.append(
            f'  (fp_line (start {wire.get("x1")} {flip_y(wire.get("y1"))}) '
            f'(end {wire.get("x2")} {flip_y(wire.get("y2"))}) '
            f'(stroke (width {width:g}) (type solid)) (layer "{layer}"))'
        )

    # Eagle layer 41 carries RF keepout notes rather than an outline, so the
    # courtyard is drawn here; the extra margin at -Y clears the antenna end.
    lines.append(
        '  (fp_rect (start -5.7 -8.2) (end 5.7 7.7) '
        '(stroke (width 0.05) (type default)) (fill none) (layer "F.CrtYd"))'
    )

    lines.append('  (fp_text user "ANTENNA AREA" (at 0 -5.0 0) (layer "F.SilkS")')
    lines.append('    (effects (font (size 0.7 0.7) (thickness 0.1)))')
    lines.append("  )")

    for pad in pads:
        rotation = float(pad.get("rot", "R0").removeprefix("R"))
        rotation_text = f" {rotation:g}" if rotation else ""
        lines.append(
            f'  (pad "{pad.get("name")}" smd rect '
            f'(at {pad.get("x")} {flip_y(pad.get("y"))}{rotation_text}) '
            f'(size {pad.get("dx")} {pad.get("dy")}) '
            '(layers "F.Cu" "F.Paste" "F.Mask"))'
        )

    lines.append(
        '  (model "${KIPRJMOD}/drone_fc_v2_footprints.3dshapes/MDBT53-P1M.step"'
    )
    lines.append("    (offset (xyz 0 0 0))")
    lines.append("    (scale (xyz 1 1 1))")
    lines.append("    (rotate (xyz -90 0 0))")
    lines.append("  )")
    lines.append(")")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return len(pads)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    print(f"Converted {convert(args.source, args.destination)} pads")


if __name__ == "__main__":
    main()
