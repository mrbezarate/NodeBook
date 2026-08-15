with open('crates/brain-core/src/db.rs', 'r') as f:
    content = f.read()

content = content.replace('}).unwrap();', '}).map_err(|e| BrainError::Database(e.to_string()))?;')
content = content.replace(').unwrap();', ').map_err(|e| BrainError::Database(e.to_string()))?;')

with open('crates/brain-core/src/db.rs', 'w') as f:
    f.write(content)
