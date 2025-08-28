# BNHBot - 交易大赛机器人

BNHBot 是一个专为交易大赛设计的自动化机器人，支持多交易所API集成、实时排名计算、钉钉群通知等功能。

## 🚀 主要功能

### 1. 用户管理
- 用户注册和审核
- 多交易所API集成（Binance、OKX、Weex）
- 身份验证和权限管理

### 2. 排名系统
- 实时余额监控
- 日排名、周排名、月排名
- 翻仓用户识别和祝贺
- 累积收益率计算

### 3. 定时任务系统 ⏰
- **每小时排名更新**：自动收集用户余额并更新排名
- **每日晚上8点Top5播报**：在钉钉群播报前5名排名
- **翻仓用户即时祝贺**：发现翻仓用户立即发送祝贺消息

### 4. 钉钉集成
- 自动排名播报
- 翻仓祝贺通知
- 管理员手动触发功能

## 🛠️ 技术架构

- **后端**: Rust + Axum + SQLite
- **前端**: HTML + CSS + JavaScript + Chart.js
- **数据库**: SQLite
- **定时任务**: Tokio异步运行时
- **API集成**: 多交易所REST API

## 📋 环境变量配置

创建 `.env` 文件并配置以下变量：

```bash
# 钉钉机器人配置
DINGTALK_WEBHOOK=https://oapi.dingtalk.com/robot/send?access_token=YOUR_ACCESS_TOKEN
DINGTALK_SECRET=YOUR_SECRET_KEY
DINGTALK_AT_ALL=false

# 数据库配置
DATABASE_URL=sqlite:data/bnhbot.db

# Web服务器配置
WEB_SERVER_HOST=127.0.0.1
WEB_SERVER_PORT=3000
WEB_SERVER_BASE_URL=http://localhost:3000

# 日志级别
RUST_LOG=info
```

## 🚀 快速开始

### 1. 安装依赖
```bash
cargo install
```

### 2. 配置环境变量
```bash
cp env.example .env
# 编辑 .env 文件，填入你的配置
```

### 3. 运行服务
```bash
cargo run
```

### 4. 访问服务
- 报名表单: http://localhost:3000/register
- 排名页面: http://localhost:3000/rankings
- 管理界面: http://localhost:3000/admin

## 📊 定时任务详解

### 每小时排名更新
- **执行时间**: 每小时整点
- **功能**: 
  - 收集所有用户余额数据
  - 更新排名表
  - 检查翻仓用户并发送祝贺

### 每日Top5播报
- **执行时间**: 每日晚上8点（中国时间）
- **功能**:
  - 播报前5名用户排名
  - 显示用户详细信息
  - 包含排名页面链接

### 翻仓用户监控
- **执行频率**: 每5分钟检查一次
- **功能**:
  - 实时监控用户翻仓状态
  - 自动发送祝贺消息
  - 避免重复祝贺

## 🔧 管理API

### 手动触发功能
- `POST /api/admin/trigger-hourly-update` - 手动触发每小时排名更新
- `POST /api/admin/trigger-top5-broadcast` - 手动触发Top5播报
- `POST /api/admin/check-congratulations` - 手动检查翻仓祝贺
- `POST /api/admin/update-rankings` - 手动更新排名表

### 用户管理
- `GET /api/admin/registrations` - 获取所有注册用户
- `POST /api/admin/registrations/:id/review` - 审核用户注册
- `DELETE /api/admin/registrations/:id` - 删除用户注册

## 📈 排名算法

### 排名计算
- **日排名**: 基于1天的余额变化
- **周排名**: 基于7天的余额变化
- **月排名**: 基于30天的余额变化

### 翻仓识别
- 用户累积收益率 ≥ 100% 时自动识别为翻仓
- 系统会记录翻仓日期和金额
- 避免重复发送祝贺消息

## 🎯 使用场景

1. **交易大赛管理**: 自动化排名计算和播报
2. **用户激励**: 翻仓用户即时祝贺
3. **数据监控**: 实时余额和收益率跟踪
4. **群组管理**: 钉钉群自动化通知

## 🔍 故障排除

### 常见问题
1. **钉钉消息发送失败**: 检查Webhook配置和签名密钥
2. **排名数据不更新**: 检查交易所API配置和网络连接
3. **定时任务不执行**: 检查系统时间和日志输出

### 日志查看
```bash
# 设置日志级别
export RUST_LOG=info

# 运行服务查看详细日志
cargo run
```

## 🤝 贡献

欢迎提交Issue和Pull Request来改进BNHBot！

## 📄 许可证

MIT License

---

**注意**: 请确保在生产环境中正确配置钉钉机器人权限和交易所API密钥，并定期备份数据库文件。
