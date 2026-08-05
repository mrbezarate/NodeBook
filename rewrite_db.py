with open('crates/brain-core/src/db.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'unwrap()' in line:
        if 'lock()' in line:
            line = line.replace('.unwrap()', '.map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?')
        elif 'query_map' in line or 'query_row' in line or 'execute' in line or 'prepare' in line or 'transaction' in line:
            # We only replace the .unwrap() at the end of the chain. 
            # To be safe, we just replace all unwrap() on that line if it's a db operation.
            # But wait, there might be row.get(0).unwrap() inside the closure.
            line = line.replace('.unwrap()', '.map_err(|e| BrainError::Database(e.to_string()))?')
        elif 'serde_json::from_str' in line or 'DateTime::parse' in line:
            # Inside closure
            line = line.replace('.unwrap()', '.map_err(|_| rusqlite::Error::InvalidQuery)?')
        
    lines[i] = line

with open('crates/brain-core/src/db.rs', 'w') as f:
    f.writelines(lines)
