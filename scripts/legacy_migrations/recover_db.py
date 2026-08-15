import json

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
                        print("FOUND EDIT for db.rs")
                        chunks = args.get('ReplacementChunks', [])
                        if not chunks:
                            print(args.get('ReplacementContent'))
                        else:
                            for c in chunks:
                                print(c.get('ReplacementContent'))
                        print("-" * 80)
