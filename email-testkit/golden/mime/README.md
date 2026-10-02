# MIME golden

每份样本一对：`fixtures/*.eml` → 本目录同名 `.json` 期望输出。

S3 接入 `chck-mime` 后，`core` 与 `windows` 必须对同一文件给出一致的 `subject` / `from` / `text` / `html_sanitized` / `remote_blocked`。
