import sys

with open('src/main.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if 'Commands::Tui { provider, model } => {' in line:
        new_lines.append(line)
        new_lines.append('                    start_full_system(config, provider, model, false).await\n')
        # We need to find the next '}' and skip lines in between
    elif 'start_full_system(config, None, None, false).await' in line and 'Commands::Tui' not in line:
         # This is probably from the previous script's mass replace, but let's check context
         new_lines.append(line)
    else:
        new_lines.append(line)

# Wait, the previous script replaced ALL start_full_system calls.
# Let's just fix the one in Commands::Tui specifically.
