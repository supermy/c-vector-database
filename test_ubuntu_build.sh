#!/bin/bash
# 模拟 Ubuntu 编译环境测试

set -e

echo "=== 模拟 Ubuntu 编译测试 ==="
echo ""

for project in qwen35 minimax25 glm5 kimi25; do
    echo ">>> 测试 $project"
    cd $project
    
    # 清理
    make clean 2>/dev/null || true
    
    # 使用严格的 GCC 编译选项（模拟 Ubuntu）
    echo "  编译中..."
    if gcc -std=c11 -O3 -Wall -Wextra -c *.c 2>&1 | grep -i "error"; then
        echo "  ❌ 编译失败"
        exit 1
    else
        echo "  ✅ 编译成功"
    fi
    
    # 链接
    echo "  链接中..."
    if gcc *.o -o test_${project} -lpthread -lm 2>&1 | grep -i "error"; then
        echo "  ❌ 链接失败"
        exit 1
    else
        echo "  ✅ 链接成功"
    fi
    
    # 清理
    rm -f *.o test_${project}
    cd ..
    echo ""
done

echo "=== 所有项目测试通过 ==="
