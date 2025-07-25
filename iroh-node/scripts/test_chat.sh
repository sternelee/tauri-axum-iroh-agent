#!/bin/bash

# Iroh P2P 聊天测试脚本

set -e

echo "=== Iroh P2P 聊天测试 ==="

# 检查是否安装了 Rust
if ! command -v cargo &> /dev/null; then
    echo "错误: 未找到 cargo，请先安装 Rust"
    exit 1
fi

# 进入项目目录
cd "$(dirname "$0")/.."

echo "1. 编译项目..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "编译失败"
    exit 1
fi

echo "2. 编译成功!"

echo "3. 运行帮助命令..."
cargo run -- --help

echo ""
echo "4. 测试创建聊天室..."
echo "运行以下命令来创建聊天室:"
echo "cargo run -- --name \"测试用户\" open"

echo ""
echo "5. 测试加入聊天室..."
echo "在另一个终端运行以下命令来加入聊天室:"
echo "cargo run -- --name \"用户2\" join <邀请码>"

echo ""
echo "=== 高级功能测试 ==="
echo "运行交互式聊天:"
echo "cargo run --example advanced_chat -- --name \"高级用户\" chat"

echo ""
echo "发送文件:"
echo "cargo run --example advanced_chat -- --name \"发送者\" send-file <文件路径> <接收者邀请码>"

echo ""
echo "接收文件:"
echo "cargo run --example advanced_chat -- --name \"接收者\" receive-file <发送者邀请码>"

echo ""
echo "测试脚本完成!"