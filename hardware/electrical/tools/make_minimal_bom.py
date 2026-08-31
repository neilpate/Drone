import csv

rows = []
with open("hardware/electrical/v2/drone_fc-bom-jlcpcb.csv", newline="", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        rows.append(
            {
                "Comment": row["Value"],
                "Designator": row["Refs"],
                "Footprint": row["Footprint"].split(":")[-1],
            }
        )

with open("hardware/electrical/v2/drone_fc-bom-minimal.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=["Comment", "Designator", "Footprint"])
    writer.writeheader()
    writer.writerows(rows)

print("wrote drone_fc-bom-minimal.csv,", len(rows), "rows")
