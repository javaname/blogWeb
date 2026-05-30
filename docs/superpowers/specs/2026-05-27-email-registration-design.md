# 邮箱验证码注册设计

## 背景

当前系统只有后台管理员登录能力，用户表仅包含 `username`、`password`、`role` 和创建时间。新增注册功能需要支持用户使用邮箱注册，并通过网易邮箱 SMTP 发送验证码完成邮箱归属验证。

## 推荐方案

采用“先发送邮箱验证码，验证码校验通过后创建账号”的流程。

流程：

1. 用户在登录页切换到注册表单，输入邮箱并请求验证码。
2. 后端校验邮箱格式和重复注册状态，生成 6 位验证码。
3. 后端通过配置的网易邮箱 SMTP 账号发送验证码邮件。
4. 后端只保存验证码哈希、过期时间和使用状态，不保存明文验证码。
5. 用户提交邮箱、验证码、密码和确认密码。
6. 后端校验验证码未过期且未使用，创建 `role=user` 的普通账号，并标记邮箱已验证。
7. 用户可使用邮箱或用户名登录。

## 后端设计

新增配置：

- `email.smtp_host`
- `email.smtp_port`
- `email.username`
- `email.password`
- `email.from`
- `email.verification_ttl_sec`

网易邮箱对接使用标准 SMTP。密码字段存放邮箱授权码，不在代码中硬编码。

新增数据：

- `users.email`
- `users.email_verified_at`
- `email_verification_codes.email`
- `email_verification_codes.code_hash`
- `email_verification_codes.expires_at`
- `email_verification_codes.used_at`
- `email_verification_codes.created_at`

新增接口：

- `POST /api/auth/register/code`
  - 请求：`{ "email": "reader@example.com" }`
  - 响应：`201 { "sent": true, "expires_in": 600 }`

- `POST /api/auth/register`
  - 请求：`{ "email": "...", "code": "123456", "password": "...", "confirm_password": "..." }`
  - 响应：`201 { "user": { "id": 1, "username": "...", "email": "...", "role": "user" } }`

错误处理：

- 邮箱格式错误返回 `400 invalid_params`
- 邮箱已注册返回 `409 conflict`
- 邮件未配置或发送失败返回 `500 email_unavailable`
- 验证码错误、过期或已使用返回 `400 invalid_verification_code`
- 请求过频返回 `429 rate_limited`

## 前端设计

在现有登录页增加登录/注册切换，不新增复杂页面结构。

注册表单包含：

- 邮箱
- 验证码
- 发送验证码按钮
- 密码
- 确认密码

注册成功后展示成功提示并切换回登录表单，邮箱自动填入登录用户名输入框。

## 测试设计

后端按 TDD 增加测试：

- 发送验证码会写入验证码记录并调用 fake 邮件发送器。
- 错误邮箱格式会被拒绝。
- 错误验证码无法注册。
- 过期验证码无法注册。
- 正确验证码会创建普通 `user` 账号并标记邮箱已验证。
- 已注册邮箱不能重复发送注册验证码或重复注册。
- 使用邮箱可以登录。

前端验证：

- i18n key 检查通过。
- UI 完整性检查通过。
- 构建通过。

## 非目标

- 不实现找回密码。
- 不实现 OAuth 或第三方登录。
- 不在测试中连接真实网易邮箱。
- 不把网易邮箱账号和授权码提交到代码仓库。
