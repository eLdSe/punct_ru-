import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter

def create_punct_icon(size=256):
    # Create image with RGBA
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Rounded rectangle background (Squircle)
    margin = int(size * 0.06)
    radius = int(size * 0.22)
    bg_box = [margin, margin, size - margin, size - margin]

    # Create background mask with supersampling for ultra-crisp edges
    scale = 4
    hi_size = size * scale
    hi_img = Image.new("RGBA", (hi_size, hi_size), (0, 0, 0, 0))
    hi_draw = ImageDraw.Draw(hi_img)
    hi_margin = margin * scale
    hi_radius = radius * scale
    hi_box = [hi_margin, hi_margin, hi_size - hi_margin, hi_size - hi_margin]

    # Subtle modern dark gradient: Deep navy/slate (#0f172a to #1e1b4b / #1e293b)
    # Draw vertical gradient inside rounded rect
    gradient = Image.new("RGBA", (hi_size, hi_size))
    for y in range(hi_size):
        t = y / hi_size
        # Top: #131b2e (19, 27, 46) -> Bottom: #090d16 (9, 13, 22)
        r = int(19 * (1 - t) + 10 * t)
        g = int(27 * (1 - t) + 14 * t)
        b = int(46 * (1 - t) + 26 * t)
        for x in range(hi_size):
            gradient.putpixel((x, y), (r, g, b, 255))

    # Rounded rect mask
    mask = Image.new("L", (hi_size, hi_size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle(hi_box, radius=hi_radius, fill=255)

    # Subtle inner border / glow
    hi_img.paste(gradient, (0, 0), mask)

    # Border: subtle modern glass border (top slightly brighter)
    border_width = int(2.5 * scale)
    hi_draw.rounded_rectangle(hi_box, radius=hi_radius, outline=(255, 255, 255, 35), width=border_width)

    # 2. Draw the typography:
    # A bold, geometric modern Cyrillic "П" with an accent punctuation mark
    # Let's draw "П" geometrically for pixel-perfect sharpness across all scales:
    # Left bar, Top bar, Right bar
    p_top = int(hi_size * 0.26)
    p_bottom = int(hi_size * 0.74)
    p_left = int(hi_size * 0.23)
    p_right = int(hi_size * 0.61)
    bar_thick = int(hi_size * 0.088)

    # Subtle drop shadow for glyph
    shadow_offset = int(4 * scale)
    shadow_mask = Image.new("RGBA", (hi_size, hi_size), (0, 0, 0, 0))
    s_draw = ImageDraw.Draw(shadow_mask)

    def draw_glyph_p(target_draw, col):
        # Top bar
        target_draw.rounded_rectangle([p_left, p_top, p_right, p_top + bar_thick], radius=int(scale * 3), fill=col)
        # Left bar
        target_draw.rounded_rectangle([p_left, p_top, p_left + bar_thick, p_bottom], radius=int(scale * 3), fill=col)
        # Right bar
        target_draw.rounded_rectangle([p_right - bar_thick, p_top, p_right, p_bottom], radius=int(scale * 3), fill=col)

    # Draw shadow
    draw_glyph_p(s_draw, (0, 0, 0, 160))
    # Comma shadow
    comma_cx = int(hi_size * 0.77)
    comma_cy = int(hi_size * 0.65)
    c_radius = int(hi_size * 0.065)

    def draw_comma(target_draw, dot_col, tail_col):
        # Dot of comma
        target_draw.ellipse([comma_cx - c_radius, comma_cy - c_radius, comma_cx + c_radius, comma_cy + c_radius], fill=dot_col)
        # Curved tail of comma
        tail_poly = [
            (comma_cx + int(c_radius * 0.8), comma_cy + int(c_radius * 0.2)),
            (comma_cx + int(c_radius * 0.9), comma_cy + int(c_radius * 0.9)),
            (comma_cx + int(c_radius * 0.2), comma_cy + int(c_radius * 2.2)),
            (comma_cx - int(c_radius * 0.4), comma_cy + int(c_radius * 1.5)),
            (comma_cx, comma_cy + int(c_radius * 0.8)),
        ]
        target_draw.polygon(tail_poly, fill=tail_col)

    draw_comma(s_draw, (0, 0, 0, 160), (0, 0, 0, 160))
    # Period above or quotation accent
    period_cx = int(hi_size * 0.77)
    period_cy = int(hi_size * 0.35)
    p_radius = int(hi_size * 0.045)
    s_draw.ellipse([period_cx - p_radius, period_cy - p_radius, period_cx + p_radius, period_cy + p_radius], fill=(0, 0, 0, 160))

    # Blur shadow
    shadow_blurred = shadow_mask.filter(ImageFilter.GaussianBlur(radius=6 * scale))
    hi_img.paste(shadow_blurred, (0, shadow_offset), shadow_blurred)

    # Draw main "П" in crisp clean white (#f8fafc) with subtle vertical gradient
    draw_glyph_p(hi_draw, (248, 250, 252, 255))

    # Glow for the comma (AI accent: Cyan / Electric Blue #06b6d4 -> #38bdf8)
    glow_img = Image.new("RGBA", (hi_size, hi_size), (0, 0, 0, 0))
    g_draw = ImageDraw.Draw(glow_img)
    draw_comma(g_draw, (6, 182, 212, 180), (6, 182, 212, 180))
    g_draw.ellipse([period_cx - p_radius, period_cy - p_radius, period_cx + p_radius, period_cy + p_radius], fill=(168, 85, 247, 180))
    glow_blurred = glow_img.filter(ImageFilter.GaussianBlur(radius=8 * scale))
    hi_img.paste(glow_blurred, (0, 0), glow_blurred)

    # Draw vivid Comma: Electric Cyan (#00e5ff / #38bdf8)
    draw_comma(hi_draw, (0, 229, 255, 255), (0, 229, 255, 255))

    # Draw vibrant Period / Accent dot: Neon Purple / Fuchsia (#c084fc)
    hi_draw.ellipse([period_cx - p_radius, period_cy - p_radius, period_cx + p_radius, period_cy + p_radius], fill=(192, 132, 252, 255))

    # Downsample with Lanczos high quality filter
    img = hi_img.resize((size, size), Image.Resampling.LANCZOS)
    return img

if __name__ == "__main__":
    out_dir = r"d:\Projects\my_tests\punct_ru\assets"
    os.makedirs(out_dir, exist_ok=True)

    img_256 = create_punct_icon(256)
    img_256.save(os.path.join(out_dir, "punct_256.png"), "PNG")

    # Generate standard multi-resolution ICO (16, 24, 32, 48, 64, 128, 256)
    sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_path = os.path.join(out_dir, "punct.ico")
    img_256.save(ico_path, format="ICO", sizes=sizes)

    # Also copy to dist and app_rust
    dist_ico = r"d:\Projects\my_tests\punct_ru\dist\punct.ico"
    app_ico = r"d:\Projects\my_tests\punct_ru\app_rust\punct.ico"
    img_256.save(dist_ico, format="ICO", sizes=sizes)
    img_256.save(app_ico, format="ICO", sizes=sizes)

    # Also save 32x32 PNG for tray embedding
    img_32 = create_punct_icon(32)
    img_32.save(os.path.join(out_dir, "punct_32.png"), "PNG")
    img_32.save(r"d:\Projects\my_tests\punct_ru\app_rust\punct_32.png", "PNG")

    print(f"Icons generated successfully: {ico_path}")
