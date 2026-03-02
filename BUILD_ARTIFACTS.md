# 构建产物说明

## GitHub Actions 构建产物

### 临时构建产物 (90 天)

每次推送到 main 分支或 Pull Request 都会触发 GitHub Actions 构建，构建产物会保留 **90 天**。

**获取方式：**
1. 访问 [Actions](https://github.com/supermy/c-vector-database/actions)
2. 点击对应的构建工作流
3. 在页面底部找到 "Artifacts" 部分
4. 下载对应的构建产物

**产物内容：**
- `qwen35-binary-*` - qwen35 测试程序
- `minimax25-binary-*` - minimax25 测试程序
- `glm5-binary-*` - glm5 测试程序
- `kimi25-binary-*` - kimi25 测试程序

### 永久构建产物 (GitHub Releases)

通过打标签 (tag) 触发 Release 构建，产物会**永久保存**在 GitHub Releases。

**获取方式：**
1. 访问 [Releases](https://github.com/supermy/c-vector-database/releases)
2. 选择对应的版本
3. 下载 Assets 中的压缩包

**产物内容：**
- `qwen35-linux-x64.tar.gz`
- `minimax25-linux-x64.tar.gz`
- `glm5-linux-x64.tar.gz`
- `kimi25-linux-x64.tar.gz`
- `SHA256SUMS.txt` (校验和文件)

## 触发 Release 构建

### 创建新版本

```bash
# 本地打标签
git tag -a v1.0.0 -m "Release version 1.0.0"

# 推送到 GitHub
git push origin v1.0.0
```

推送标签后会自动触发 Release 构建工作流。

### 手动触发

1. 访问 [Actions](https://github.com/supermy/c-vector-database/actions/workflows/release.yml)
2. 点击 "Run workflow"
3. 选择分支
4. 点击 "Run workflow"

## 本地构建

如果不想使用 GitHub Actions，也可以本地构建：

```bash
# 构建所有项目
for project in qwen35 minimax25 glm5 kimi25; do
  cd $project && make && cd ..
done

# 单独构建
cd qwen35 && make
```

## 产物说明

### 测试程序
- 包含完整的测试代码
- 可用于验证功能
- 需要动态链接库

### 静态编译版本 (Release)
- 完全静态链接
- 无外部依赖
- 可直接在任何 Linux x64 系统运行

## 校验和验证

下载 Release 产物后，建议验证完整性：

```bash
# 下载 SHA256SUMS.txt
# 下载对应的 tar.gz 文件

# 验证
sha256sum -c SHA256SUMS.txt
```

预期输出：
```
qwen35-linux-x64.tar.gz: OK
minimax25-linux-x64.tar.gz: OK
glm5-linux-x64.tar.gz: OK
kimi25-linux-x64.tar.gz: OK
```

## 存储策略对比

| 存储方式 | 保留期限 | 触发方式 | 用途 |
|----------|----------|----------|------|
| Actions Artifacts | 90 天 | 每次推送 | 开发测试 |
| GitHub Releases | 永久 | 打标签 | 正式发布 |
| 本地构建 | 永久 | 手动 | 自定义需求 |

## 注意事项

1. **Artifacts 过期**: GitHub Actions 的构建产物会在 90 天后自动删除
2. **存储空间**: GitHub 为每个仓库提供 500MB 的 Actions 存储空间
3. **清理旧产物**: 定期清理旧的构建产物以释放空间
4. **Release 永久**: GitHub Releases 的附件永久保存，除非手动删除

## 相关链接

- [Actions 工作流](https://github.com/supermy/c-vector-database/actions)
- [Releases 页面](https://github.com/supermy/c-vector-database/releases)
- [CI 工作流配置](.github/workflows/ci.yml)
- [Release 工作流配置](.github/workflows/release.yml)
