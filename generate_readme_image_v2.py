#!/usr/bin/env python3
"""
生成完整的README图片用于微博分享 - 版本2
"""

from PIL import Image, ImageDraw, ImageFont

def create_full_image():
    # 更大的画布容纳更多内容
    width = 1200
    height = 2200
    img = Image.new('RGB', (width, height), color='#0d1117')
    draw = ImageDraw.Draw(img)
    
    # 加载字体
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 52)
        subtitle_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 36)
        section_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 30)
        text_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 26)
        small_font = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 22)
    except:
        title_font = ImageFont.load_default()
        subtitle_font = ImageFont.load_default()
        section_font = ImageFont.load_default()
        text_font = ImageFont.load_default()
        small_font = ImageFont.load_default()
    
    # 颜色
    white = '#ffffff'
    cyan = '#58a6ff'
    yellow = '#ffd700'
    green = '#3fb950'
    orange = '#ff7b72'
    purple = '#d2a8ff'
    gray = '#8b949e'
    border = '#30363d'
    
    def draw_card(y_start, title, emoji, content_lines, title_color=yellow):
        """绘制卡片"""
        padding = 30
        card_margin = 80
        line_height = 38
        
        # 计算卡片高度
        card_height = 80 + len(content_lines) * line_height + padding
        
        # 卡片背景
        draw.rounded_rectangle(
            [(card_margin, y_start), (width - card_margin, y_start + card_height)],
            radius=15,
            fill='#161b22',
            outline=border,
            width=2
        )
        
        # 卡片标题
        y = y_start + 25
        draw.text((width//2, y), f"{emoji} {title}", font=section_font, fill=title_color, anchor="mm")
        y += 55
        
        # 内容
        for line in content_lines:
            if isinstance(line, tuple):
                # (前缀颜色, 前缀, 内容颜色, 内容)
                prefix_color, prefix, content_color, content = line
                draw.text((card_margin + 30, y), prefix, font=text_font, fill=prefix_color)
                draw.text((card_margin + 30 + len(prefix)*26, y), content, font=text_font, fill=content_color)
            else:
                draw.text((card_margin + 30, y), line, font=text_font, fill=white)
            y += line_height
        
        return y_start + card_height + 30
    
    y = 50
    
    # ========== 标题 ==========
    draw.text((width//2, y), "🔥 C语言向量数据库", font=title_font, fill=cyan, anchor="mm")
    y += 70
    draw.text((width//2, y), "高性能向量存储与相似度搜索", font=subtitle_font, fill=gray, anchor="mm")
    y += 60
    
    # ========== 性能指标卡片 ==========
    perf_lines = [
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        (cyan, "kimi25    ", white, "插入 131K/s  |  搜索 5.1ms  |  O(n)线性查找"),
        (green, "minimax25 ", white, "插入 267K/s  |  搜索 4.5ms  |  O(1)哈希索引"),
        (orange, "glm5      ", white, "插入 283K/s  |  搜索 8.8ms  |  O(1)哈希索引"),
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        "",
        (purple, "🏆 最快插入: ", yellow, "glm5 (283K vec/s)"),
        (purple, "🏆 最快搜索: ", yellow, "minimax25 (4.5ms)"),
    ]
    y = draw_card(y, "性能指标", "⚡", perf_lines, yellow)
    
    # ========== 功能特性卡片 ==========
    feature_lines = [
        (green, "✓ ", white, "向量CRUD - 完整的增删改查操作"),
        (green, "✓ ", white, "Top-K搜索 - 余弦/欧氏/点积距离度量"),
        (green, "✓ ", white, "哈希索引 - minimax25/glm5支持O(1)查找"),
        (green, "✓ ", white, "HNSW框架 - kimi25预留近似搜索结构"),
        (green, "✓ ", white, "数据持久化 - 二进制文件存储与加载"),
        (green, "✓ ", white, "性能测试 - 包含完整基准测试代码"),
        "",
        (gray, "📊 代码规模: ", white, "kimi25(560行) > minimax25(450行) > glm5(350行)"),
    ]
    y = draw_card(y, "功能特性", "✨", feature_lines, green)
    
    # ========== 适用场景卡片 ==========
    scene_lines = [
        (orange, "▸ kimi25    ", white, "小数据量 + 需要扩展HNSW近似搜索"),
        (cyan, "▸ minimax25 ", white, "中等数据量 + 通用场景 + 快速搜索"),
        (purple, "▸ glm5      ", white, "大数据量 + 内存敏感 + 快速插入"),
        "",
        (yellow, "💡 推荐: ", white, "一般场景选minimax25，追求速度选glm5"),
    ]
    y = draw_card(y, "适用场景", "🎯", scene_lines, purple)
    
    # ========== HNSW说明卡片 ==========
    hnsw_lines = [
        (yellow, "什么是HNSW?", white, ""),
        "分层可导航小世界图 - 近似最近邻搜索算法",
        "",
        (cyan, "加速效果:", white, " O(n) → O(log n)，加速100-1000倍"),
        (cyan, "适用规模:", white, " 向量数量 > 10万时效果显著"),
        (cyan, "精度损失:", white, " 1-5%（可接受范围）"),
        "",
        (gray, "当前状态: ", orange, "kimi25有框架，minimax25/glm5无HNSW"),
        (gray, "影响: ", green, "功能完整，小规模数据性能无差异"),
    ]
    y = draw_card(y, "HNSW索引", "🔍", hnsw_lines, orange)
    
    # ========== 底部信息 ==========
    y += 40
    draw.rounded_rectangle([(100, y), (width-100, y+140)], radius=10, fill='#238636', outline='#2ea043', width=2)
    y += 35
    draw.text((width//2, y), "📦 GitHub开源", font=section_font, fill=white, anchor="mm")
    y += 45
    draw.text((width//2, y), "github.com/supermy/c-vector-database", font=text_font, fill=white, anchor="mm")
    
    y += 80
    # 标签
    tags = "#C语言  #向量数据库  #相似度搜索  #HNSW  #开源项目  #程序员"
    draw.text((width//2, y), tags, font=small_font, fill=gray, anchor="mm")
    
    # 保存
    output_path = "/Users/moyong/project/ai/vdb/readme_weibo_v2.png"
    img.save(output_path, "PNG", quality=95)
    print(f"图片已生成: {output_path}")
    print(f"尺寸: {width}x{height}")
    return output_path

if __name__ == "__main__":
    from PIL import Image, ImageDraw, ImageFont
    path = create_full_image()
    print(f"\n✅ 图片路径: {path}")
    print("包含: 性能指标 | 功能特性 | 适用场景 | HNSW说明 | GitHub地址")
