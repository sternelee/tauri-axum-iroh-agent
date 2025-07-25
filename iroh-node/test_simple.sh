#!/bin/bash

# 简单的测试脚本

echo "=== Iroh P2P 聊天测试 ==="

# 检查编译
echo "1. 检查编译..."
cargo check --bin iroh-node

if [ $? -eq 0 ]; then
    echo "✅ 编译成功!"
    
    echo ""
    echo "2. 显示帮助信息..."
    cargo run --bin iroh-node -- --help
    
    echo ""
    echo "3. 测试说明:"
    echo "   创建聊天室: cargo run --bin iroh-node -- --name \"用户1\" open"
    echo "   加入聊天室: cargo run --bin iroh-node -- --name \"用户2\" join <邀请码>"
    
else
    echo "❌ 编译失败，请检查错误信息"
fi