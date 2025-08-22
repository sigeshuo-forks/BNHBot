#!/bin/bash

echo "🚀 设置 BNHBot 项目..."

# 创建数据目录
mkdir -p data

# 复制环境变量文件
if [ ! -f .env ]; then
    echo "📝 创建 .env 文件..."
    cp env.example .env
    echo "请编辑 .env 文件，配置钉钉机器人信息"
else
    echo "✅ .env 文件已存在"
fi

# 检查 Rust 版本
echo "🔍 检查 Rust 版本..."
rustc --version

# 构建项目
echo "🔨 构建项目..."
cargo build

echo "✅ 项目设置完成！"
echo ""
echo "下一步："
echo "1. 编辑 .env 文件，配置钉钉机器人信息"
echo "2. 运行 'cargo run -- --help' 查看可用命令"
echo "3. 添加用户：cargo run -- add-user --dingtalk-id 'user123' --name '张三'"
