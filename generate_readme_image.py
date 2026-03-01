#!/usr/bin/env python3
"""
生成README图片用于微博分享
"""

from PIL import Image, ImageDraw, ImageFont
import textwrap
import os

def create_readme_image():
    # 创建白色背景图片 (微博推荐尺寸 1200x675 或 900x500)
    width = 1200
    height = 1600
    img = Image.new('RGB', (width, height), color='#1a1a2e')
    draw = ImageDraw.Draw(img)
    
    # 尝试加载字体
    try:
        # macOS 系统字体
        title_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 48)
        subtitle_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 32)
        text_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 24)
        small_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 20)
    except:
        try:
            title_font = ImageFont.truetype("/System/Library/Fonts/STHeiti Light.ttc", 48)
            subtitle_font = ImageFont.truetype("/System/Library/Fonts/STHeiti Light.ttc", 32)
            text_font = ImageFont.truetype("/System/Library/Fonts/STHeiti Light.ttc", 24)
            small_font = ImageFont.truetype("/System/Library/Fonts/STHeiti Light.ttc", 20)
        except:
            title_font = ImageFont.load_default()
            subtitle_font = ImageFont.load_default()
            text_font = ImageFont.load_default()
            small_font = ImageFont.load_default()
    
    # 颜色定义
    white = '#ffffff'
    cyan = '#00d4ff'
    yellow = '#ffd700'
    green = '#00ff88'
    orange = '#ff6b6b'
    gray = '#888888'
    
    y = 40
    
    # 标题
    draw.text((width//2, y), "C语言向量数据库", font=title_font, fill=cyan, anchor="mm")
    y += 70
    
    draw.text((width//2, y), "高性能向量存储与相似度搜索", font=subtitle_font, fill=gray, anchor="mm")
    y += 80
    
    # 分隔线
    draw.line([(100, y), (width-100, y)], fill='#333333', width=2)
    y += 40
    
    # 性能对比
    draw.text((width//2, y), "⚡ 性能对比", font=subtitle_font, fill=yellow, anchor="mm")
    y += 60
    
    # 表格
    headers = ["版本", "插入速度", "搜索速度", "ID查找"]
    col_widths = [200, 280, 280, 240]
    x_start = 100
    
    # 表头背景
    draw.rectangle([(x_start, y), (width-x_start, y+45)], fill='#16213e')
    x = x_start
    for i, header in enumerate(headers):
        draw.text((x + col_widths[i]//2, y+22), header, font=text_font, fill=white, anchor="mm")
        x += col_widths[i]
    y += 50
    
    # 数据行
    data = [
        ["kimi25", "131K vec/s", "5.1 ms", "O(n)"],
        ["minimax25", "267K vec/s", "4.5 ms", "O(1)"],
        ["glm5", "283K vec/s", "8.8 ms", "O(1)"],
    ]
    
    colors = [orange, green, cyan]
    for row_idx, row in enumerate(data):
        x = x_start
        for i, cell in enumerate(row):
            color = colors[row_idx] if i == 0 else white
            draw.text((x + col_widths[i]//2, y+20), cell, font=text_font, fill=color, anchor="mm")
            x += col_widths[i]
        y += 45
    
    y += 40
    
    # 功能特性
    draw.text((width//2, y), "✨ 功能特性", font=subtitle_font, fill=yellow, anchor="mm")
    y += 60
    
    features = [
        "✅ 向量CRUD操作 - 完整的增删改查",
        "✅ Top-K相似度搜索 - 余弦/欧氏/点积距离",
        "✅ 哈希索引 - O(1) ID查找 (minimax25/glm5)",
        "✅ HNSW框架 - 近似最近邻搜索预留 (kimi25)",
        "✅ 数据持久化 - 二进制文件存储",
        "✅ 性能测试 - 包含对比基准测试",
    ]
    
    for feature in features:
        draw.text((x_start, y), feature, font=text_font, fill=white)
        y += 40
    
    y += 30
    
    # 适用场景
    draw.text((width//2, y), "🎯 适用场景", font=subtitle_font, fill=yellow, anchor="mm")
    y += 60
    
    scenarios = [
        ("kimi25", "小数据量、需要扩展HNSW索引"),
        ("minimax25", "中等数据量、通用场景、快速搜索"),
        ("glm5", "大数据量、内存敏感、快速插入"),
    ]
    
    for name, desc in scenarios:
        draw.text((x_start, y), f"• {name}:", font=text_font, fill=cyan)
        draw.text((x_start + 150, y), desc, font=text_font, fill=white)
        y += 40
    
    y += 40
    
    # 分隔线
    draw.line([(100, y), (width-100, y)], fill='#333333', width=2)
    y += 40
    
    # GitHub地址
    draw.text((width//2, y), "📦 GitHub", font=subtitle_font, fill=green, anchor="mm")
    y += 50
    
    draw.text((width//2, y), "github.com/supermy/c-vector-database", font=text_font, fill=cyan, anchor="mm")
    y += 60
    
    # 标签
    tags = ["#C语言", "#向量数据库", "#相似度搜索", "#HNSW", "#开源"]
    tag_text = "  ".join(tags)
    draw.text((width//2, y), tag_text, font=small_font, fill=gray, anchor="mm")
    
    # 保存图片
    output_path = "/Users/moyong/project/ai/vdb/readme_weibo.png"
    img.save(output_path, "PNG", quality=95)
    print(f"图片已生成: {output_path}")
    print(f"尺寸: {width}x{height}")
    
    return output_path

if __name__ == "__main__":
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError:
        print("正在安装 Pillow...")
        import subprocess
        subprocess.run(["pip3", "install", "Pillow", "-q"])
        from PIL import Image, ImageDraw, ImageFont
    
    path = create_readme_image()
    print(f"\n图片路径: {path}")
    print("可以直接用于微博分享!")
