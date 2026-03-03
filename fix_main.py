import sys

with open('src/main.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = 0
for i, line in enumerate(lines):
    if skip > 0:
        skip -= 1
        continue
    
    if 'Commands::Tui { provider, model } => {' in line:
        new_lines.append(line)
        new_lines.append('                    start_full_system(config, false).await\n')
        # Skip until the end of this block
        j = i + 1
        while j < len(lines) and 'Ok(())' not in lines[j]:
            j += 1
        skip = j - i
    else:
        new_lines.append(line)

with open('src/main.rs', 'w') as f:
    f.writelines(new_lines)
