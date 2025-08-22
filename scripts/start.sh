#!/bin/bash

echo "🚀 启动 BNHBot 系统..."

# 确保数据目录存在
echo "📁 创建数据目录..."
mkdir -p data

# 检查环境变量文件
if [ ! -f .env ]; then
    echo "⚠️  警告: .env 文件不存在，请先配置环境变量"
    echo "📝 复制环境变量示例文件:"
    echo "   cp env.example .env"
    echo "   然后编辑 .env 文件，配置钉钉机器人信息"
    echo ""
fi

# 启动服务
echo "🔧 启动 BNHBot 服务..."
cargo run

echo "✅ 服务已停止"
