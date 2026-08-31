import csv
import re


def expand_refs(refs):
    parts = []
    for token in refs.split(","):
        m = re.match(r"^([A-Za-z]+)(\d+)-([A-Za-z]+)?(\d+)$", token)
        if m:
            prefix, start, prefix2, end = m.groups()
            if prefix2 and prefix2 != prefix:
                parts.append(token)
                continue
            parts.extend(f"{prefix}{n}" for n in range(int(start), int(end) + 1))
        else:
            parts.append(token)
    return ",".join(parts)


rows = []
with open("hardware/electrical/v2/drone_fc-bom.csv", newline="", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    fieldnames = reader.fieldnames
    for row in reader:
        row["Refs"] = expand_refs(row["Refs"])
        rows.append(row)

with open("hardware/electrical/v2/drone_fc-bom-jlcpcb.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=fieldnames, quoting=csv.QUOTE_ALL)
    writer.writeheader()
    writer.writerows(rows)

print("wrote drone_fc-bom-jlcpcb.csv,", len(rows), "rows")
