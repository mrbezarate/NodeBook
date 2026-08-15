import json

with open('/home/mrbez/.gemini/antigravity-cli/brain/2b39969d-24cd-467f-8066-bb138c96adcd/.system_generated/logs/transcript_full.jsonl', 'r') as f:
    for line in f:
        data = json.loads(line)
        if data.get('type') == 'PLANNER_RESPONSE':
            for tc in data.get('tool_calls', []):
                args = tc.get('args', {})
                # Check for run_command with db.rs
                if tc.get('name') == 'run_command' and 'db.rs' in args.get('CommandLine', ''):
                    print(f"COMMAND: {args.get('CommandLine')}")
                elif tc.get('name') in ['replace_file_content', 'multi_replace_file_content', 'write_to_file'] and 'db.rs' in args.get('TargetFile', args.get('AbsolutePath', '')):
                    print(f"TOOL: {tc.get('name')}")
                    print(json.dumps(args, indent=2))
