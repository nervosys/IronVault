import json, os
with open("target/tarpaulin-report.json") as f:
    data = json.load(f)
for fi in data["files"]:
    p = "/".join(fi["path"])
    if "src/" not in p:
        continue
    c = fi["covered"]
    t = fi["coverable"]
    if t > 0:
        short = p.split("IronVault/")[-1] if "IronVault/" in p else p
        pct = 100*c/t
        uncov = t - c
        print(f"{c:4d}/{t:4d} ({pct:5.1f}%) uncov={uncov:3d}  {short}")
