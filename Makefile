ca.PHONY: help build test clean setup run add-user add-exchange query-balance trigger-query start-scheduler registration

help: ## 显示帮助信息
	@echo "BNHBot - 钉钉机器人交易所余额播报系统"
	@echo ""
	@echo "可用命令:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## 构建项目
	cargo build

release: ## 构建发布版本
	cargo build --release

test: ## 运行测试
	cargo test

clean: ## 清理构建文件
	cargo clean

setup: ## 初始化项目
	@echo "🚀 设置 BNHBot 项目..."
	@mkdir -p data
	@if [ ! -f .env ]; then \
		echo "📝 创建 .env 文件..."; \
		cp env.example .env; \
		echo "请编辑 .env 文件，配置钉钉机器人信息"; \
	else \
		echo "✅ .env 文件已存在"; \
	fi
	@echo "🔨 构建项目..."
	@cargo build
	@echo "✅ 项目设置完成！"

run: ## 启动完整服务
	@echo "🚀 启动 BNHBot 完整服务..."
	@echo "📁 确保数据目录存在..."
	@mkdir -p data
	@echo "🔧 启动服务..."
	cargo run

run-cli: ## 运行命令行模式
	cargo run -- --help

add-user: ## 添加用户示例
	@echo "添加用户示例:"
	@echo "cargo run -- add-user --dingtalk-id 'user123' --name '张三'"

add-exchange: ## 添加交易所配置示例
	@echo "添加交易所配置示例:"
	@echo "币安: cargo run -- add-exchange --dingtalk-id 'user123' --exchange 'binance' --api-key 'your_key' --secret-key 'your_secret'"
	@echo "欧易: cargo run -- add-exchange --dingtalk-id 'user123' --exchange 'okx' --api-key 'your_key' --secret-key 'your_secret' --passphrase 'your_passphrase'"
	@echo "WEEX: cargo run -- add-exchange --dingtalk-id 'user123' --exchange 'weex' --api-key 'your_key' --secret-key 'your_secret'"

query-balance: ## 查询余额示例
	@echo "查询余额示例:"
	@echo "cargo run -- query-balance --dingtalk-id 'user123'"

trigger-query: ## 手动触发余额查询
	cargo run -- trigger-query

start-scheduler: ## 启动定时任务
	cargo run -- start-scheduler

registration: ## 报名系统示例
	@echo "报名系统使用示例:"
	@echo "创建报名: cargo run -- registration create --dingtalk-id 'user123' --registration-type 'exchange' --title '申请绑定API' --content '需要绑定币安API' --contact-info '13800138000'"
	@echo "查询报名: cargo run -- registration query --dingtalk-id 'user123'"
	@echo "审核报名: cargo run -- registration review --registration-id 'uuid' --status 'approved' --admin-notes '审核通过' --admin-id 'admin-uuid'"
	@echo "待审核列表: cargo run -- registration pending"
	@echo "报名统计: cargo run -- registration stats"

check: ## 检查代码
	cargo check
	cargo clippy
	cargo fmt -- --check

fmt: ## 格式化代码
	cargo fmt

clippy: ## 运行 Clippy 检查
	cargo clippy

install: ## 安装到系统
	cargo install --path .

uninstall: ## 从系统卸载
	cargo uninstall bnhbot
