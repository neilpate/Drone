import fitz

doc = fitz.open("doc/hardware/MicroBit_V2.2.1_nRF52820 schematic.PDF")
print("pages:", doc.page_count)
for i, page in enumerate(doc):
    text = page.get_text()
    if "antenna" in text.lower() or "rf" in text.lower() or "2.4" in text.lower():
        print(f"===== page {i + 1} =====")
        print(text[:2000])
