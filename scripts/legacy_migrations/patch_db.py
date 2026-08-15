import json

edits = []
with open('/home/mrbez/.gemini/antigravity-cli/brain/2b39969d-24cd-467f-8066-bb138c96adcd/.system_generated/logs/transcript_full.jsonl', 'r') as f:
    for line in f:
        data = json.loads(line)
        if data.get('type') == 'PLANNER_RESPONSE':
            for tool_call in data.get('tool_calls', []):
                name = tool_call.get('name')
                args = tool_call.get('args', {})
                if name in ['multi_replace_file_content', 'replace_file_content']:
                    target = args.get('TargetFile', '')
                    if 'db.rs' in target:
                        chunks = args.get('ReplacementChunks', [])
                        if not chunks:
                            edits.append((args.get('TargetContent'), args.get('ReplacementContent')))
                        else:
                            for c in chunks:
                                edits.append((c.get('TargetContent'), c.get('ReplacementContent')))

with open('crates/brain-core/src/db.rs', 'r') as f:
    content = f.read()

for target, replacement in edits:
    content = content.replace(target, replacement)

with open('crates/brain-core/src/db.rs', 'w') as f:
    f.write(content)
