import sys

with open('src/main.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if 'return start_full_system(config, provider, model, false).await;' in line:
        if 'Commands::Tui' not in line: # This check is actually by line content, but Tui command doesn't use 'return'
             new_lines.append(line.replace('provider, model', 'None, None'))
        else:
             new_lines.append(line)
    else:
        new_lines.append(line)

with open('src/main.rs', 'w') as f:
    f.writelines(new_lines)
