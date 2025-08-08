#!/usr/bin/env python3
"""
Create proper icon files for SymbioteIDE
"""
import os
from PIL import Image, ImageDraw

def create_icon_image(size):
    """Create a simple SymbioteIDE icon"""
    # Create a new image with transparent background
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Create a modern gradient-like effect
    # Background circle
    margin = size // 8
    draw.ellipse([margin, margin, size-margin, size-margin], 
                fill=(45, 55, 72, 255), outline=(99, 102, 241, 255), width=2)
    
    # Inner design - representing code/AI
    center = size // 2
    inner_size = size // 3
    
    # Draw some geometric shapes to represent AI/code
    # Triangle (representing AI/intelligence)
    triangle_size = inner_size // 2
    triangle_points = [
        (center, center - triangle_size//2),
        (center - triangle_size//2, center + triangle_size//2),
        (center + triangle_size//2, center + triangle_size//2)
    ]
    draw.polygon(triangle_points, fill=(99, 102, 241, 255))
    
    # Small circles (representing nodes/connections)
    dot_size = size // 20
    positions = [
        (center - inner_size//3, center - inner_size//3),
        (center + inner_size//3, center - inner_size//3),
        (center, center + inner_size//2)
    ]
    
    for pos in positions:
        draw.ellipse([pos[0]-dot_size, pos[1]-dot_size, 
                     pos[0]+dot_size, pos[1]+dot_size], 
                    fill=(255, 255, 255, 255))
    
    return img

def main():
    # Create icons directory
    icons_dir = "src-tauri/icons"
    os.makedirs(icons_dir, exist_ok=True)
    
    # Create different sized icons
    sizes = [32, 128, 256]
    
    for size in sizes:
        img = create_icon_image(size)
        
        # Save PNG files
        if size == 32:
            img.save(f"{icons_dir}/32x32.png", "PNG")
        elif size == 128:
            img.save(f"{icons_dir}/128x128.png", "PNG")
            # Also create @2x version
            img_2x = create_icon_image(256)
            img_2x.save(f"{icons_dir}/128x128@2x.png", "PNG")
    
    # Create ICO file (Windows)
    ico_sizes = [16, 32, 48, 64, 128, 256]
    ico_images = []
    
    for size in ico_sizes:
        ico_images.append(create_icon_image(size))
    
    # Save as ICO
    ico_images[0].save(f"{icons_dir}/icon.ico", format='ICO', 
                      sizes=[(img.width, img.height) for img in ico_images])
    
    # Create ICNS file (macOS) - simplified version
    # For proper ICNS, we'd need more sizes, but this will work
    img_512 = create_icon_image(512)
    img_512.save(f"{icons_dir}/icon.icns", format='ICNS')
    
    print("✅ Icons created successfully!")
    print(f"📁 Created in: {icons_dir}")
    print("📋 Files created:")
    for file in os.listdir(icons_dir):
        print(f"   - {file}")

if __name__ == "__main__":
    main()
