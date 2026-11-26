import json
import subprocess
import os

with open('.codeyam/state/command-output/runs/reconcile-registry-4c58fbdf-87a4-4f29-9253-a04a36a10612.txt', 'r') as f:
    lines = f.readlines()

adds = []
for line in lines:
    if "skipped: add 'test › " in line and "description_missing" in line:
        # Extract the key
        start_idx = line.find("add 'test › ") + len("add '")
        end_idx = line.find("' (description_missing:")
        key = line[start_idx:end_idx]
        
        # Extract description and map it to a format CodeYam likes
        desc = key.replace("test › ", "")
        
        adds.append({
            "key": key,
            "runner": "node",
            "description": desc
        })

print(f"Found {len(adds)} missing descriptions to backfill")

with open('.codeyam/tmp/backfill.json', 'w') as f:
    json.dump(adds, f, indent=2)

subprocess.run(["codeyam-editor", "editor", "register-test-batch", "@.codeyam/tmp/backfill.json"], check=True)
