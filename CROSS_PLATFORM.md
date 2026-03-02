# 跨平台编译指南

本文档说明如何在不同操作系统和 CPU 架构上编译和运行向量数据库项目。

## 支持的平台

| 操作系统 | 架构 | 编译器 | 状态 |
|----------|------|--------|------|
| **macOS** | x86_64 (Intel) | Clang/GCC | ✅ 已测试 |
| **macOS** | ARM64 (Apple Silicon M1/M2) | Clang | ✅ 已测试 |
| **Ubuntu** | x86_64 | GCC | ✅ 已测试 |
| **Ubuntu** | ARM64 | GCC | ✅ 已测试 |
| **Debian** | x86_64 | GCC | ✅ 兼容 |
| **CentOS/RHEL** | x86_64 | GCC | ✅ 兼容 |

## 系统依赖

### macOS

```bash
# 安装 Xcode 命令行工具
xcode-select --install

# 安装 GNU Make (可选，macOS 自带 make 可用)
brew install make
```

### Ubuntu/Debian

```bash
# 安装编译工具链
sudo apt-get update
sudo apt-get install -y build-essential

# 安装额外依赖（如需要）
sudo apt-get install -y gcc make
```

### CentOS/RHEL

```bash
# 安装开发工具组
sudo yum groupinstall "Development Tools"

# 或安装单独包
sudo yum install -y gcc gcc-c++ make
```

## 编译项目

### 快速编译（所有项目）

```bash
# 编译所有项目
for project in qwen35 minimax25 glm5 kimi25; do
  cd $project && make && cd ..
done
```

### 单独编译项目

```bash
# qwen35 (推荐 - 性能最优)
cd qwen35 && make

# minimax25 (IVF 聚类)
cd minimax25 && make

# glm5 (内存对齐优化)
cd glm5 && make

# kimi25 (HNSW 算法)
cd kimi25 && make
```

### 清理编译

```bash
# 清理单个项目
cd qwen35 && make clean

# 清理所有项目
for project in qwen35 minimax25 glm5 kimi25; do
  cd $project && make clean && cd ..
done
```

## 运行测试

```bash
# 运行所有测试
for project in qwen35 minimax25 glm5 kimi25; do
  cd $project && ./test_* && cd ..
done
```

## 跨平台特性

### 1. pthread 线程库

所有项目都使用 `pthread` 实现线程安全：

- **头文件**: `#include <pthread.h>`
- **链接标志**: `-lpthread`
- **功能**: 读写锁 (`pthread_rwlock_t`)

### 2. SIMD 指令集自动检测

**qwen35 项目**支持自动 SIMD 检测：

| 架构 | SIMD | 并行度 |
|------|------|--------|
| x86_64 (AVX-512) | AVX-512 | 16 路 |
| x86_64 (AVX2) | AVX2 | 8 路 |
| x86_64 (SSE4.2) | SSE4.2 | 4 路 |
| ARM64 | NEON | 4 路 |

查看 SIMD 配置：
```bash
cd qwen35
make simd-info
```

### 3. 数学库

所有项目使用标准数学库：

- **头文件**: `#include <math.h>`
- **链接标志**: `-lm`
- **函数**: `sqrtf`, `cos`, `sin` 等

## 编译器标志

### 通用标志

```makefile
CFLAGS = -std=c11 -O3 -Wall -Wextra
LDFLAGS = -lpthread -lm
```

### 平台特定标志

#### macOS (Intel)
```makefile
CFLAGS += -mavx -mavx2 -msse4.2
```

#### macOS (ARM64)
```makefile
# 自动使用 NEON，无需额外标志
```

#### Linux (x86_64)
```makefile
CFLAGS += -mavx -mavx2 -msse4.2
```

#### Linux (ARM64)
```makefile
# 自动使用 NEON，无需额外标志
```

## 常见问题解决

### 问题 1: `pthread.h: No such file or directory`

**原因**: 缺少 pthread 库

**解决方案**:
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install -y build-essential

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
```

### 问题 2: `math.h: No such file or directory`

**原因**: 缺少标准 C 库

**解决方案**:
```bash
# Ubuntu/Debian
sudo apt-get install -y libc6-dev

# CentOS/RHEL
sudo yum install glibc-headers
```

### 问题 3: `undefined reference to 'pthread_rwlock_init'`

**原因**: 缺少 pthread 链接

**解决方案**:
确保 Makefile 包含 `-lpthread` 标志。

### 问题 4: macOS 上编译正常，Ubuntu 上报错

**可能原因**:
1. Ubuntu 上缺少开发包
2. 编译器版本过旧

**解决方案**:
```bash
# 更新编译器和依赖
sudo apt-get update
sudo apt-get install -y build-essential gcc

# 检查编译器版本
gcc --version  # 建议 GCC 7.0+
```

### 问题 5: ARM64 架构编译失败

**解决方案**:
```bash
# 确保使用正确的目标架构
make ARCH_FLAGS="-target arm64-apple-macos11"  # macOS ARM64
# 或
make ARCH_FLAGS="-march=armv8-a"  # Linux ARM64
```

## 性能优化建议

### macOS (Apple Silicon)

```bash
# 使用原生 ARM64 编译
cd qwen35
make ARCH_FLAGS="-target arm64-apple-macos11"
```

### Linux (服务器)

```bash
# 启用所有可用 SIMD 指令
cd qwen35
make CFLAGS_SIMD="-march=native -mtune=native"
```

### 交叉编译

```bash
# macOS 编译 Linux
cd qwen35
make CC=x86_64-linux-gnu-gcc \
     CFLAGS_PLATFORM="-I/usr/x86_64-linux-gnu/include" \
     LDFLAGS_PLATFORM="-L/usr/x86_64-linux-gnu/lib"
```

## 验证编译

### 检查二进制文件架构

```bash
# macOS
file qwen35/test_qwen35

# Linux
file qwen35/test_qwen35
```

### 检查 SIMD 支持

```bash
# macOS
sysctl -a | grep -E "(hw.optional.avx|hw.optional.neon)"

# Linux
lscpu | grep -E "(AVX|NEON|SSE)"
```

## CI/CD 集成

GitHub Actions 已配置自动构建：

- **平台**: Ubuntu-latest, macOS-14
- **架构**: x86_64, ARM64
- **触发**: 每次推送到 main 分支

查看构建状态：https://github.com/supermy/c-vector-database/actions

## 相关文档

- [构建产物说明](BUILD_ARTIFACTS.md)
- [CHANGELOG](CHANGELOG.md)
- [README](README.md)

## 技术支持

如遇到跨平台编译问题，请提交 Issue 并包含：

1. 操作系统和版本
2. 编译器版本 (`gcc --version` 或 `clang --version`)
3. CPU 架构 (`uname -m`)
4. 完整的错误信息
