import re
import sys

def merge_specs(delta_path, main_path, output_path):
    with open(delta_path, 'r', encoding='utf-8') as f:
        delta_text = f.read()
    
    with open(main_path, 'r', encoding='utf-8') as f:
        main_text = f.read()

    # Find the requirements section in delta
    # We want everything from '## Requirements' to the end of the file or the next section
    delta_req_match = re.search(r'## Requirements\s*(.*)', delta_text, re.DOTALL | re.IGNORECASE)
    if not delta_req_match:
        print("Error: Could not find '## Requirements' in delta spec.")
        return

    delta_reqs_content = delta_req_match.group(1).strip()
    
    # Parse requirements from delta
    # A requirement starts with '### Requirement: '
    delta_reqs = re.split(r'(?=### Requirement:)', delta_reqs_content)
    delta_reqs = [r for r in delta_reqs if r.strip()]

    # Find the requirements section in main
    main_req_match = re.search(r'(## Requirements\s*)(.*?)(\n## |$)', main_text, re.DOTALL | re.IGNORECASE)
    if not main_req_match:
        print("Error: Could not find '## Requirements' in main spec.")
        return

    main_req_header = main_req_match.group(1)
    main_reqs_content = main_req_match.group(2)
    main_rest = main_text[main_req_match.end():]

    # Parse requirements from main
    main_reqs = re.split(r'(?=### Requirement:)', main_reqs_content)
    main_reqs = [r for r in main_reqs if r.strip()]

    # Create a map of existing requirements by name
    # Requirement name is the part after '### Requirement: ' until the first newline
    def get_req_name(req_text):
        match = re.search(r'### Requirement:\s*(.*)', req_text)
        return match.group(1).strip() if match else None

    main_req_map = {get_req_name(r): r for r in main_reqs if get_req_name(r)}
    
    # Apply delta
    for d_req in delta_reqs:
        name = get_req_name(d_req)
        if name:
            # If it exists, replace it (MODIFIED/REPLACED)
            # If it doesn't, it's ADDED.
            # Note: The skill says 'APPEND' for ADDED, 'REPLACE' for MODIFIED.
            # Given this is a single delta file, we treat everything in it as "new/updated"
            main_req_map[name] = d_req

    # Reconstruct requirements section
    # Sort by name? Or just maintain some order? Let's just append new ones to the end of the list.
    # Actually, let's try to keep the order of main and just append new ones.
    
    new_main_reqs_list = []
    seen_names = set()

    # First, add existing ones (updated if they were in delta)
    for r in main_reqs:
        name = get_req_name(r)
        if name:
            if name in main_req_map:
                new_main_reqs_list.append(main_req_map[name])
                seen_names.add(name)
            else:
                new_main_reqs_list.append(r)
        else:
            # This shouldn't happen with proper split but for safety:
            new_main_reqs_list.append(r)

    # Second, add completely new ones from delta that weren't in main
    for d_req in delta_reqs:
        name = get_req_name(d_req)
        if name and name not in seen_names:
            new_main_reqs_list.append(d_req)
            seen_names.add(name)

    # Final Assembly
    # The main_req_match.group(1) was '## Requirements\n'
    # We need to handle the spacing.
    
    new_reqs_section = main_req_header + "\n\n" + "\n\n".join(new_main_reqs_list).strip() + "\n"
    
    # The original main_text structure:
    # [Text before ## Requirements]
    # ## Requirements
    # [Requirements content]
    # [Text after Requirements]
    
    # We need to find the text before the match
    pre_req_match = re.search(r'(.*?)## Requirements', main_text, re.DOTALL | re.IGNORECASE)
    pre_req_text = pre_req_match.group(1) if pre_req_match else ""

    final_text = pre_req_text + new_reqs_section + main_rest
    
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(final_text)
    
    print(f"Successfully merged {len(delta_reqs)} requirements into {main_path}")
    print(f"Total requirements now: {len(main_req_map)}")

if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("Usage: python merge.py <delta_path> <main_path> <output_path>")
    else:
        merge_specs(sys.argv[1], sys.argv[2], sys.argv[3])
