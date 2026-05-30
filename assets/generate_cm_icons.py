import os
import subprocess
import xml.etree.ElementTree as ET

symbols = {
    "alpha": "alpha",
    "alpha-upper": "Alpha",
    "beta": "beta",
    "beta-upper": "Beta",
    "gamma": "gamma",
    "gamma-upper": "Gamma",
    "theta": "theta",
    "theta-upper": "Theta",
    "omega": "omega",
    "omega-upper": "Omega",
    "pi": "pi",
    "pi-upper": "Pi",
    "integral": "integral",
    "sigma": "sum",
    "fraction": "x/y",
    "superscript": "x^y",
    "subscript": "x_y",
    "matrix": "mat(a, b; c, d)",
}

# Custom target sizes (out of 24) to keep visual size uniform
# e.g., beta is tall due to descender, so scaling it to a 16px box makes it look tiny.
# By setting a larger target box, we balance its visual weight.
targets = {
    "beta": 21.0,
    "beta-upper": 18.0,
    "integral": 22.0,
    "sigma": 18.0,
    "fraction": 20.0,
    "superscript": 18.0,
    "subscript": 18.0,
    "matrix": 19.0,
    "alpha-upper": 17.0,
    "gamma-upper": 17.0,
    "theta-upper": 17.0,
    "omega-upper": 17.0,
    "pi-upper": 17.0,
}

# Namespace mappings for parsing SVG
namespaces = {
    'svg': 'http://www.w3.org/2000/svg',
    'xlink': 'http://www.w3.org/1999/xlink'
}
# Register namespaces globally for serialization
ET.register_namespace('', 'http://www.w3.org/2000/svg')
ET.register_namespace('xlink', 'http://www.w3.org/1999/xlink')

def generate_symbol(name, cmd):
    typ_file = "temp_symbol.typ"
    svg_file = "temp_symbol.svg"
    
    # Write Typst file
    with open(typ_file, "w") as f:
        f.write("#set page(width: auto, height: auto, margin: 0pt)\n")
        f.write("#set text(size: 24pt)\n")
        f.write(f"$ {cmd} $\n")
        
    try:
        # Compile to SVG
        subprocess.run(["typst", "compile", typ_file, svg_file], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        
        # Parse SVG
        tree = ET.parse(svg_file)
        root = tree.getroot()
        
        # Extract viewBox
        viewbox_str = root.attrib.get('viewBox', '0 0 24 24')
        _, _, w_str, h_str = viewbox_str.split()
        W = float(w_str)
        H = float(h_str)
        
        # Remove background path (fill="#ffffff")
        bg_paths = []
        for path in root.findall('.//svg:path', namespaces):
            if path.attrib.get('fill') == '#ffffff':
                bg_paths.append(path)
        for bg in bg_paths:
            # Remove from parent
            for parent in root.iter():
                if bg in parent:
                    parent.remove(bg)
                    
        # Replace fill="#000000" with currentColor
        for elem in root.iter():
            if elem.attrib.get('fill') == '#000000':
                elem.attrib['fill'] = 'currentColor'
                
        # Calculate scaling to fit in a 24x24 box with a safe margin
        if name in ("superscript", "subscript"):
            # Unified layout reference from x_y^y (W=24.9504, H=25.512)
            W_ref = 24.9504
            H_ref = 25.512
            target_size = 18.0
            scale = target_size / H_ref
            
            dx = (24.0 - W_ref * scale) / 2.0
            if name == "superscript":
                dy = (24.0 - H_ref * scale) / 2.0
            else: # subscript
                dy = (24.0 - H_ref * scale) / 2.0 + 5.5488 * scale
        else:
            target_size = targets.get(name, 16.0)
            max_dim = max(W, H)
            scale = target_size / max_dim if max_dim > 0 else 1.0
            
            # Center in 24x24 box
            W_prime = W * scale
            H_prime = H * scale
            dx = (24.0 - W_prime) / 2.0
            dy = (24.0 - H_prime) / 2.0
        
        # Collect children to wrap
        children_to_wrap = []
        for child in list(root):
            if child.tag != f"{{{namespaces['svg']}}}defs" and child.attrib.get('id') != 'glyph':
                children_to_wrap.append(child)
                root.remove(child)
                
        # Create wrapper group with translation and scale
        wrapper = ET.Element(f"{{{namespaces['svg']}}}g")
        wrapper.attrib['transform'] = f"translate({dx:.4f}, {dy:.4f}) scale({scale:.6f})"
        for child in children_to_wrap:
            wrapper.append(child)
            
        root.append(wrapper)
        
        # Clean SVG attributes
        root.attrib['viewBox'] = "0 0 24 24"
        if 'width' in root.attrib:
            del root.attrib['width']
        if 'height' in root.attrib:
            del root.attrib['height']
            
        # Save to assets/icons/math/
        target_path = f"assets/icons/math/{name}.svg"
        tree.write(target_path, encoding="utf-8", xml_declaration=True)
        print(f"Generated {target_path} successfully using CM math font (target size: {target_size})!")
        
    finally:
        # Cleanup temporary files
        if os.path.exists(typ_file):
            os.remove(typ_file)
        if os.path.exists(svg_file):
            os.remove(svg_file)

if __name__ == "__main__":
    for name, cmd in symbols.items():
        generate_symbol(name, cmd)
