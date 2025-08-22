# BNHBot - 钉钉机器人交易所余额播报系统

BNHBot 是一个基于 Rust 开发的钉钉机器人系统，用于每日自动播报用户在各大交易所的账户余额。

## 功能特性

- 🚀 **多交易所支持**: 支持币安(Binance)、欧易(OKX)、WEEX 三大交易所
- ⏰ **定时播报**: 每日早上8点自动查询并播报所有用户余额
- 🔐 **安全存储**: 使用 SQLite 数据库安全存储用户API配置
- 📱 **钉钉集成**: 通过钉钉机器人发送格式化的余额报告
- 🛡️ **API安全**: 支持API签名验证，确保安全性
- 📊 **余额统计**: 自动计算USDT等值，提供资产总览
- 📝 **报名系统**: 支持多种类型的用户报名和审核流程

## 系统架构

```
BNHBot/
├── src/
│   ├── models/          # 数据模型
│   ├── services/        # 核心服务
│   ├── handlers/        # 命令处理
│   └── utils/           # 工具函数
├── 数据库服务           # SQLite数据存储
├── 交易所服务           # 三大交易所API集成
├── 钉钉机器人服务       # 消息发送
├── 定时任务调度器       # 每日8点执行
└── 报名系统             # 用户报名和审核管理
```

## 安装和配置

### 1. 环境要求

- Rust 1.70+
- SQLite 3.x

### 2. 克隆项目

```bash
git clone <repository-url>
cd BNHBot
```

### 3. 安装依赖

```bash
cargo build
```

### 4. 配置环境变量

复制 `env.example` 为 `.env` 并配置：

```bash
cp env.example .env
```

编辑 `.env` 文件：

```env
# 钉钉机器人配置
DINGTALK_WEBHOOK=https://oapi.dingtalk.com/robot/send?access_token=YOUR_ACCESS_TOKEN
DINGTALK_SECRET=YOUR_SECRET_KEY

# 数据库配置
DATABASE_URL=sqlite:data/bnhbot.db

# 日志级别
RUST_LOG=info
```

### 5. 获取钉钉机器人配置

1. 在钉钉群中添加自定义机器人
2. 获取 Webhook URL 和 Secret
3. 配置到 `.env` 文件中

## 快速开始

```bash
# 1. 克隆项目
git clone <repository-url>
cd BNHBot

# 2. 安装依赖
cargo build

# 3. 配置环境变量
cp env.example .env
# 编辑 .env 文件，配置钉钉机器人信息

# 4. 启动完整服务
cargo run
# 或者使用 make 命令
make run
```

启动成功后，你可以：
- 🌐 访问 http://localhost:3000 查看系统主页
- 📝 访问 http://localhost:3000/register 进行用户报名
- ⚙️ 通过钉钉机器人进行交互

## 使用方法

### 1. 启动完整服务（推荐）

```bash
# 直接启动所有功能
cargo run

# 或者使用 make 命令
make run
```

启动后会自动运行：
- ✅ 定时任务调度器（每日8点执行余额查询）
- ✅ Web服务器（端口3000，提供报名表单和管理界面）
- ✅ 钉钉机器人服务
- ✅ 数据库服务

### 2. 命令行模式（高级用户）

如果需要使用命令行工具，可以添加参数：

```bash
# 添加用户（不推荐，建议使用报名系统）
cargo run -- add-user --dingtalk-id "user123" --name "张三"
```

### 2. 配置交易所API

```bash
# 币安
cargo run -- add-exchange \
  --dingtalk-id "user123" \
  --exchange "binance" \
  --api-key "your_api_key" \
  --secret-key "your_secret_key"

# 欧易（需要passphrase）
cargo run -- add-exchange \
  --dingtalk-id "user123" \
  --exchange "okx" \
  --api-key "your_api_key" \
  --secret-key "your_secret_key" \
  --passphrase "your_passphrase"

# WEEX
cargo run -- add-exchange \
  --dingtalk-id "user123" \
  --exchange "weex" \
  --api-key "your_api_key" \
  --secret-key "your_secret_key"
```

### 3. 查询用户余额

```bash
cargo run -- query-balance --dingtalk-id "user123"
```

### 4. 手动触发余额查询

```bash
cargo run -- trigger-query
```

### 5. 启动定时任务

```bash
cargo run -- start-scheduler
```

## 报名系统使用

### 6. 创建新报名

```bash
cargo run -- registration create \
  --dingtalk-id "user123" \
  --registration-type "exchange" \
  --title "申请绑定币安API" \
  --content "需要绑定币安API进行余额查询" \
  --contact-info "13800138000"
```

### 7. 查询用户报名记录

```bash
cargo run -- registration query --dingtalk-id "user123"
```

### 8. 审核报名

```bash
cargo run -- registration review \
  --registration-id "uuid-here" \
  --status "approved" \
  --admin-notes "审核通过" \
  --admin-id "admin-uuid"
```

### 9. 查看待审核列表

```bash
cargo run -- registration pending
```

### 10. 查看报名统计

```bash
cargo run -- registration stats
```

## 支持的交易所

### 币安 (Binance)
- API版本: v3
- 支持功能: 账户余额查询
- 特殊要求: 需要API Key和Secret Key

### 欧易 (OKX)
- API版本: v5
- 支持功能: 账户余额查询
- 特殊要求: 需要API Key、Secret Key和Passphrase

### WEEX
- API版本: v1
- 支持功能: 账户余额查询
- 特殊要求: 需要API Key和Secret Key

## 报名系统功能

### 支持的报名类型
- **交易所API绑定**: 申请绑定交易所API进行余额查询
- **活动报名**: 参与各类活动和会议
- **培训报名**: 参加培训和课程
- **其他**: 其他类型的申请和报名

### 报名流程
1. **用户发起**: 在钉钉群@机器人或通过Web表单
2. **信息填写**: 填写报名类型、标题、内容和联系信息
3. **自动通知**: 机器人通知管理员有新报名
4. **审核处理**: 管理员审核并更新状态
5. **结果通知**: 机器人通知用户审核结果

### 审核状态
- **待审核**: 新提交的报名，等待管理员处理
- **已通过**: 审核通过，可以进行后续操作
- **已拒绝**: 审核不通过，需要重新申请
- **已取消**: 用户或管理员取消的报名

## 数据库结构

系统使用 SQLite 数据库存储以下信息：

- **users**: 用户基本信息
- **user_exchanges**: 用户交易所配置
- **balances**: 余额记录历史
- **registrations**: 用户报名记录

## 安全注意事项

1. **API密钥安全**: 请妥善保管交易所API密钥，不要泄露给他人
2. **权限控制**: 建议只给API读取权限，避免交易权限
3. **网络安全**: 确保运行环境网络安全，避免API密钥被窃取
4. **定期更新**: 建议定期更换API密钥

## 故障排除

### 常见问题

1. **钉钉消息发送失败**
   - 检查 Webhook URL 是否正确
   - 确认机器人是否被踢出群聊
   - 检查网络连接

2. **交易所API调用失败**
   - 验证API密钥是否正确
   - 检查API权限设置
   - 确认网络连接和防火墙设置

3. **数据库连接失败**
   - 检查数据库文件权限
   - 确认SQLite版本兼容性

### 日志查看

```bash
RUST_LOG=debug cargo run -- <command>
```

### 报名系统管理

```bash
# 查看报名系统帮助
cargo run -- registration --help

# 创建报名
cargo run -- registration create --help

# 查询报名
cargo run -- registration query --help

# 审核报名
cargo run -- registration review --help
```

## 开发计划

- [ ] 支持更多交易所（火币、Gate.io等）
- [ ] 添加价格预警功能
- [ ] 支持Telegram机器人
- [ ] 添加Web管理界面
- [ ] 支持多钉钉群组
- [ ] 添加余额变化趋势分析
- [x] 报名系统基础功能
- [ ] 报名系统Web管理界面
- [ ] 支持报名模板和自定义字段
- [ ] 添加报名数据导出功能
- [ ] 支持批量审核操作

## 贡献指南

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License

## 联系方式

如有问题，请通过以下方式联系：
- 提交 GitHub Issue
- 发送邮件至：[rainweic@gmail.com](rainweic@gmail.com)

---

**免责声明**: 本工具仅用于学习和个人使用，请遵守相关法律法规和交易所使用条款。使用本工具产生的任何损失，开发者不承担责任。
