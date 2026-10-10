# Security Policy

## Supported versions

安全修复面向 `main`、`imyemail-cloud-native` 最新 `native-v*` 和 `imyemail-cloud-mygo` 最新 `mygo-v*` 两条版本线；旧 `v*` 是原生历史标签。旧标签/安装包永久保留供复核与回滚，但不承诺对每个历史版本单独回补。共享核心修复须两版回归，分别递增发布。见[版本保留](docs/VERSIONS.md)、[安装](docs/INSTALL.zh-CN.md)与[自签校验](docs/SIGNING.md)。

## Reporting a vulnerability

请通过 GitHub 仓库的 **Security → Report a vulnerability** 私下提交报告：

<https://github.com/logdns/imyemail-cloud/security/advisories/new>

请包含受影响版本/提交、平台、信任边界、最小复现和预期影响。不要附加真实邮箱凭据、token、用户邮件或生产数据库，也不要在公开 Issue 中披露尚未修复的问题。

项目维护者会确认收到报告、评估影响，并在修复可用后协调披露。请勿测试生产服务、第三方邮箱或不属于你的账户。
