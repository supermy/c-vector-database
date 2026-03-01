#!/usr/bin/env python3
"""
将README.md完整内容渲染为图片 - 修复字体乱码
"""

from PIL import Image, ImageDraw, ImageFont
import os

def get_font(size):
    """获取可用字体"""
    font_paths = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ]
    
    for path in font_paths:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except:
                continue
    
    # 使用默认字体
    return ImageFont.load_default()

def get_code_font(size):
    """获取等宽字体"""
    font_paths = [
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Courier.dfont",
        "/System/Library/Fonts/PingFang.ttc",
    ]
    
    for path in font_paths:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except:
                continue
    
    return get_font(size)

def render_readme_to_image():
    width = 1400
    height = 6500
    img = Image.new('RGB', (width, height), color='#0d1117')
    draw = ImageDraw.Draw(img)
    
    # 加载字体
    h1_font = get_font(44)
    h2_font = get_font(36)
    h3_font = get_font(28)
    text_font = get_font(22)
    code_font = get_code_font(18)
    small_font = get_font(18)
    
    # 颜色
    white = '#e6edf3'
    cyan = '#58a6ff'
    yellow = '#d29922'
    green = '#3fb950'
    orange = '#ffa657'
    red = '#ff7b72'
    purple = '#bc8cff'
    gray = '#8b949e'
    border = '#30363d'
    code_bg = '#161b22'
    
    margin = 60
    y = 40
    
    def draw_h1(text):
        nonlocal y
        draw.text((margin, y), text, font=h1_font, fill=cyan)
        y += 55
        draw.line([(margin, y), (width-margin, y)], fill=border, width=2)
        y += 25
    
    def draw_h2(text):
        nonlocal y
        y += 15
        draw.text((margin, y), text, font=h2_font, fill=yellow)
        y += 45
    
    def draw_h3(text):
        nonlocal y
        y += 10
        draw.text((margin, y), text, font=h3_font, fill=orange)
        y += 35
    
    def draw_text(text, color=white, indent=0):
        nonlocal y
        draw.text((margin + indent, y), text, font=text_font, fill=color)
        y += 30
    
    def draw_code_block(lines):
        nonlocal y
        block_height = len(lines) * 26 + 40
        draw.rounded_rectangle(
            [(margin, y), (width-margin, y+block_height)],
            radius=8, fill=code_bg, outline=border, width=1
        )
        y += 15
        for line in lines:
            if line.strip():
                draw.text((margin + 20, y), line, font=code_font, fill=white)
            y += 26
        y += 15
    
    def draw_table(headers, rows):
        nonlocal y
        col_count = len(headers)
        col_width = (width - 2*margin) // col_count
        
        draw.rectangle([(margin, y), (width-margin, y+35)], fill='#21262d')
        x = margin
        for h in headers:
            draw.text((x + 10, y+8), h, font=text_font, fill=yellow)
            x += col_width
        y += 38
        
        for row in rows:
            x = margin
            for i, cell in enumerate(row):
                cell_str = str(cell)
                if cell_str == '✅':
                    color = green
                elif cell_str == '❌':
                    color = red
                elif '⚠' in cell_str:
                    color = orange
                else:
                    color = white
                draw.text((x + 10, y+5), cell_str, font=text_font, fill=color)
                x += col_width
            y += 32
        y += 15
    
    # ========== 开始渲染 ==========
    
    draw_h1("向量数据库 (Vector Database)")
    draw_text("C语言实现的高性能向量数据库，支持向量存储、相似度搜索和持久化。", gray)
    y += 10
    
    draw_h2("项目结构")
    draw_code_block([
        "vdb/",
        "├── kimi25/           # kimi25 版本",
        "│   ├── vector_db.h",
        "│   ├── vector_db.c",
        "│   └── test_vector_db.c",
        "├── minimax25/        # minimax25 版本",
        "│   ├── vdb.h",
        "│   ├── vdb.c",
        "│   └── test_vdb.c",
        "├── glm5/             # glm5 版本",
        "│   ├── glm5_vdb.h",
        "│   ├── glm5_vdb.c",
        "│   └── test_glm5.c",
        "└── benchmark.c       # 性能对比测试"
    ])
    
    draw_h2("版本对比")
    
    draw_h3("性能指标")
    draw_table(
        ["指标", "kimi25", "minimax25", "glm5"],
        [
            ["插入速度", "131,503 vec/s", "267,294 vec/s", "283,134 vec/s"],
            ["搜索速度", "5.1 ms/query", "4.5 ms/query", "8.8 ms/query"],
            ["ID查找", "O(n) 线性", "O(1) 哈希", "O(1) 哈希"],
        ]
    )
    
    draw_h3("功能特性")
    draw_table(
        ["功能", "kimi25", "minimax25", "glm5"],
        [
            ["向量CRUD", "[OK]", "[OK]", "[OK]"],
            ["Top-K搜索", "[OK]", "[OK]", "[OK]"],
            ["余弦相似度", "[OK]", "[OK]", "[OK]"],
            ["欧氏距离", "[OK]", "[OK]", "[OK]"],
            ["点积距离", "[X]", "[OK]", "[OK]"],
            ["距离度量切换", "[X]", "[OK]", "[OK]"],
            ["哈希索引", "[X]", "[OK]", "[OK]"],
            ["HNSW框架", "[OK]", "[X]", "[X]"],
            ["持久化", "[OK]", "[OK]", "[OK]"],
            ["重复ID检测", "[X]", "[OK]", "[OK]"],
        ]
    )
    
    draw_h3("适用场景")
    draw_table(
        ["版本", "适用场景"],
        [
            ["kimi25", "小数据量、需要扩展HNSW索引"],
            ["minimax25", "中等数据量、通用场景、需要快速搜索"],
            ["glm5", "大数据量、内存敏感、需要快速插入"],
        ]
    )
    
    draw_h2("HNSW 索引详解")
    
    draw_h3("什么是 HNSW?")
    draw_text("HNSW (Hierarchical Navigable Small World) 是一种近似最近邻搜索(ANN)算法，", white)
    draw_text("用于在高维向量空间中快速找到最相似的向量。", white)
    
    draw_h3("核心问题: 暴力搜索的瓶颈")
    draw_code_block([
        "假设 100万 个 128维向量:",
        "- 暴力搜索: 需要计算 100万次 余弦相似度",
        "- 时间复杂度: O(n)",
        "- 搜索时间: 约 500ms - 1s",
        "",
        "HNSW 解决方案:",
        "- 搜索时间: 约 1-5ms",
        "- 时间复杂度: O(log n)",
        "- 加速比: 100-1000倍"
    ])
    
    draw_h3("工作原理")
    draw_code_block([
        "层级结构示意:",
        "",
        "Layer 2 (最高层)    [Node A] ----------- [Node B]",
        "                         |                     |",
        "Layer 1 (中间层)    [C]--[D]--[E]--[F]--[G]--[H]",
        "                         |    |    |    |",
        "Layer 0 (底层)    [1]-[2]-[3]-[4]-[5]-[6]-[7]-[8]",
        "                         ^",
        "                      查询入口"
    ])
    
    draw_text("搜索过程:", yellow)
    draw_text("1. 从最高层入口点开始", white, 20)
    draw_text("2. 在当前层找到最近邻", white, 20)
    draw_text("3. 下降到下一层，以上一层结果为起点", white, 20)
    draw_text("4. 重复直到底层，返回结果", white, 20)
    
    draw_h3("与其他索引对比")
    draw_table(
        ["索引类型", "搜索速度", "构建速度", "内存占用", "精度"],
        [
            ["暴力搜索", "最慢", "无需构建", "最低", "100%"],
            ["HNSW", "极快", "中等", "较高", "95-99%"],
            ["IVF", "快", "快", "中等", "90-95%"],
            ["LSH", "较快", "快", "低", "80-90%"],
        ]
    )
    
    draw_h3("实际应用场景")
    draw_code_block([
        "场景1: RAG (检索增强生成)",
        "用户问题 -> 向量嵌入 -> HNSW搜索 -> 相关文档 -> LLM -> 回答",
        "",
        "场景2: 推荐系统",
        "用户向量 -> HNSW搜索 -> 相似用户/商品 -> 推荐结果",
        "",
        "场景3: 图像检索",
        "图像特征向量 -> HNSW搜索 -> 相似图片"
    ])
    
    draw_h3("HNSW 缺失 != 功能缺失")
    draw_table(
        ["功能", "kimi25", "minimax25", "glm5", "说明"],
        [
            ["向量存储", "[OK]", "[OK]", "[OK]", "完整"],
            ["向量检索", "[OK]", "[OK]", "[OK]", "完整"],
            ["相似度搜索", "[OK]", "[OK]", "[OK]", "完整"],
            ["Top-K查询", "[OK]", "[OK]", "[OK]", "完整"],
            ["持久化", "[OK]", "[OK]", "[OK]", "完整"],
            ["HNSW索引", "[框架]", "[X]", "[X]", "加速优化"],
        ]
    )
    
    draw_h3("实际影响")
    draw_table(
        ["数据规模", "无HNSW", "有HNSW", "差异"],
        [
            ["1,000", "< 1ms", "< 1ms", "无差异"],
            ["10,000", "~5-10ms", "~1ms", "可接受"],
            ["100,000", "~50-100ms", "~2ms", "开始明显"],
            ["1,000,000", "~500ms-1s", "~5ms", "必须优化"],
        ]
    )
    
    draw_h3("何时需要 HNSW?")
    draw_code_block([
        "需要 HNSW 的场景:",
        "|-- 向量数量 > 100,000",
        "|-- 实时搜索要求 (< 10ms 响应)",
        "|-- 高并发查询场景",
        "|-- 内存充足 (HNSW 需要额外 20-50% 内存)",
        "",
        "不需要 HNSW 的场景:",
        "|-- 向量数量 < 10,000",
        "|-- 离线批量处理",
        "|-- 内存受限环境",
        "|-- 追求代码简洁"
    ])
    
    draw_h2("kimi25 版本")
    draw_text("* HNSW索引框架预留", green)
    draw_text("* 完整的向量操作API", green)
    draw_text("* 支持相似度阈值过滤", green)
    
    draw_h3("编译运行")
    draw_code_block([
        "cd kimi25",
        "make",
        "./test_vector_db"
    ])
    
    draw_h3("API 示例")
    draw_code_block([
        "#include \"vector_db.h\"",
        "",
        "VectorDB* db = vectordb_create(128, false);",
        "Vector* vec = vector_create(128);",
        "vectordb_insert(db, 1, vec, \"metadata\", 9);",
        "",
        "Vector* query = vector_create(128);",
        "SearchOptions opts = { .top_k = 10 };",
        "uint32_t count;",
        "SearchResult* results = vectordb_search(db, query, &opts, &count);"
    ])
    
    draw_h2("minimax25 版本")
    draw_text("* 哈希索引，O(1) ID查找", green)
    draw_text("* 支持多种距离度量", green)
    draw_text("* 重复ID检测", green)
    
    draw_h3("编译运行")
    draw_code_block([
        "cd minimax25",
        "gcc -O2 -o test_vdb vdb.c test_vdb.c -lm",
        "./test_vdb"
    ])
    
    draw_h3("API 示例")
    draw_code_block([
        "#include \"vdb.h\"",
        "",
        "VectorDatabase* db = vdb_create(128);",
        "Vector* vec = vector_new(128);",
        "vdb_insert(db, 1, vec, \"metadata\", 9);",
        "",
        "SearchOptions opts = { .top_k = 10, .metric = DISTANCE_COSINE };",
        "SearchResult* results = vdb_search(db, query, &opts, &count);"
    ])
    
    draw_h2("glm5 版本")
    draw_text("* 代码精简 (~350行)", green)
    draw_text("* 大容量哈希桶 (8192)", green)
    draw_text("* 最快的插入速度", green)
    draw_text("* 支持记录计数", green)
    
    draw_h3("编译运行")
    draw_code_block([
        "cd glm5",
        "gcc -O2 -o test_glm5 glm5_vdb.c test_glm5.c -lm",
        "./test_glm5"
    ])
    
    draw_h3("API 示例")
    draw_code_block([
        "#include \"glm5_vdb.h\"",
        "",
        "VecDB* db = vdb_new(128);",
        "Vector* v = vec_new(128);",
        "vdb_add(db, 1, v, \"metadata\", 9);",
        "",
        "QueryOpts opts = { .k = 10, .metric = METRIC_COSINE };",
        "QueryResult* r = vdb_query(db, q, &opts, &n);",
        "uint64_t count = vdb_count(db);"
    ])
    
    draw_h2("距离度量说明")
    draw_table(
        ["度量方式", "说明", "值范围"],
        [
            ["余弦相似度", "向量夹角的余弦值", "[-1, 1]"],
            ["欧氏距离", "向量间的直线距离", "[0, +inf)"],
            ["点积", "向量内积", "(-inf, +inf)"],
        ]
    )
    
    draw_text("选择建议:", yellow)
    draw_text("* 余弦相似度: 文本嵌入、语义搜索", white, 20)
    draw_text("* 欧氏距离: 图像特征、物理距离", white, 20)
    draw_text("* 点积: 归一化向量、推荐系统", white, 20)
    
    draw_h2("持久化")
    draw_text("所有版本都支持数据持久化:", white)
    draw_code_block([
        "// 保存",
        "vdb_save(db, \"database.bin\");",
        "",
        "// 加载",
        "VecDB* db = vdb_load(\"database.bin\");"
    ])
    
    draw_h2("性能优化建议")
    draw_text("1. 大数据量: 使用 glm5 版本，哈希桶更多", white)
    draw_text("2. 频繁搜索: 使用 minimax25 版本，搜索更快", white)
    draw_text("3. 需要近似搜索: 扩展 kimi25 的 HNSW 实现", white)
    
    draw_h2("依赖")
    draw_text("* C99 标准库", white)
    draw_text("* 数学库 (-lm)", white)
    
    draw_h2("License")
    draw_text("MIT License", green)
    
    y += 30
    draw.rounded_rectangle([(margin, y), (width-margin, y+80)], radius=10, fill='#238636')
    y += 20
    draw.text((width//2, y), "GitHub: github.com/supermy/c-vector-database", font=h3_font, fill=white, anchor="mm")
    y += 50
    draw.text((width//2, y), "#C语言 #向量数据库 #相似度搜索 #HNSW #开源", font=small_font, fill=gray, anchor="mm")
    
    output_path = "/Users/moyong/project/ai/vdb/readme_full.png"
    img.save(output_path, "PNG", quality=95)
    print(f"图片已生成: {output_path}")
    print(f"尺寸: {width}x{height}")
    return output_path

if __name__ == "__main__":
    from PIL import Image, ImageDraw, ImageFont
    path = render_readme_to_image()
