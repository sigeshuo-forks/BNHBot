# BNHBot 配置说明

## 📋 环境变量配置

BNHBot 使用环境变量进行配置，支持 `.env` 文件和系统环境变量。

### 🔧 快速配置

1. 复制环境变量示例文件：
   ```bash
   cp env.example .env
   ```

2. 编辑 `.env` 文件，配置必要的参数

3. 重启服务

## 🌐 Web服务器配置

### WEB_SERVER_HOST
- **说明**: 服务器监听地址
- **默认值**: `127.0.0.1`
- **可选值**:
  - `127.0.0.1` - 仅允许本地访问（开发环境推荐）
  - `0.0.0.0` - 允许所有IP访问（生产环境推荐）
  - 具体IP地址 - 绑定到特定网络接口

### WEB_SERVER_PORT
- **说明**: 服务器监听端口
- **默认值**: `3000`
- **注意事项**: 确保端口未被其他服务占用

### WEB_SERVER_BASE_URL
- **说明**: 完整的基础URL，用于生成各种链接
- **默认值**: `http://localhost:3000`
- **配置示例**:
  - 开发环境: `http://localhost:3000`
  - 生产环境: `https://yourdomain.com`
  - Docker环境: `http://yourdomain.com:3000`

## 🤖 钉钉机器人配置

### DINGTALK_WEBHOOK
- **说明**: 钉钉机器人的Webhook URL
- **获取方式**: 在钉钉群中添加自定义机器人后获取
- **格式**: `https://oapi.dingtalk.com/robot/send?access_token=YOUR_ACCESS_TOKEN`

### DINGTALK_SECRET
- **说明**: 机器人安全设置中的签名密钥
- **可选**: 如果设置了签名验证则需要配置
- **安全**: 建议在生产环境中启用

### DINGTALK_AT_ALL
- **说明**: 启动时是否@所有人
- **默认值**: `false`
- **可选值**: `true` / `false`

## 🗄️ 数据库配置

### DATABASE_URL
- **说明**: 数据库连接字符串
- **默认值**: `sqlite:data/bnhbot.db`
- **支持类型**:
  - SQLite: `sqlite:data/bnhbot.db`
  - MySQL: `mysql://user:password@localhost/bnhbot`
  - PostgreSQL: `postgresql://user:password@localhost/bnhbot`

## 📝 日志配置

### RUST_LOG
- **说明**: 日志级别
- **默认值**: `info`
- **可选值**: `debug`, `info`, `warn`, `error`

## 🚀 不同环境配置示例

### 开发环境
```bash
WEB_SERVER_HOST=127.0.0.1
WEB_SERVER_PORT=3000
WEB_SERVER_BASE_URL=http://localhost:3000
RUST_LOG=debug
```

### 生产环境
```bash
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=8080
WEB_SERVER_BASE_URL=https://yourdomain.com
RUST_LOG=info
```

### Docker环境
```bash
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=3000
WEB_SERVER_BASE_URL=http://yourdomain.com
RUST_LOG=info
```

### 内网部署
```bash
WEB_SERVER_HOST=0.0.0.0
WEB_SERVER_PORT=3000
WEB_SERVER_BASE_URL=http://192.168.1.100:3000
RUST_LOG=info
```

## 🔍 配置验证

启动服务后，检查日志输出确认配置是否正确：

```
Web服务器启动在: http://127.0.0.1:3000
报名表单: http://127.0.0.1:3000/register
管理界面: http://127.0.0.1:3000/admin
Webhook测试: http://127.0.0.1:3000/api/dingtalk/test
```

## ⚠️ 注意事项

1. **安全性**: 生产环境建议使用HTTPS
2. **端口**: 确保防火墙允许相应端口访问
3. **域名**: 生产环境建议配置域名和SSL证书
4. **备份**: 定期备份 `.env` 配置文件

## 🆘 常见问题

### Q: 无法从外网访问
**A**: 检查 `WEB_SERVER_HOST` 是否设置为 `0.0.0.0`

### Q: 端口被占用
**A**: 修改 `WEB_SERVER_PORT` 为其他可用端口

### Q: 链接不正确
**A**: 检查 `WEB_SERVER_BASE_URL` 是否配置正确

### Q: 钉钉机器人无法连接
**A**: 检查 `DINGTALK_WEBHOOK` 和 `DINGTALK_SECRET` 配置
