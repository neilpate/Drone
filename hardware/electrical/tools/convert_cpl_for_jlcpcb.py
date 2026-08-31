import csv

rows = []
with open("hardware/electrical/v2/drone_fc-cpl.csv", newline="", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        rotation = float(row["Rot"]) % 360
        rows.append(
            {
                "Designator": row["Ref"],
                "Mid X": row["PosX"],
                "Mid Y": row["PosY"],
                "Layer": row["Side"].capitalize(),
                "Rotation": f"{rotation:.6f}",
            }
        )

with open("hardware/electrical/v2/drone_fc-cpl-jlcpcb.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=["Designator", "Mid X", "Mid Y", "Layer", "Rotation"])
    writer.writeheader()
    writer.writerows(rows)

print("wrote drone_fc-cpl-jlcpcb.csv,", len(rows), "rows")
