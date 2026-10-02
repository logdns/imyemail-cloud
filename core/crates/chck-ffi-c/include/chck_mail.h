#pragma once

#ifdef __cplusplus
extern "C" {
#endif

void *chck_mail_open(const char *path, char **out_err);
void *chck_mail_open_with_file_secrets(const char *path, char **out_err);
char *chck_mail_call(void *engine, const char *method, const char *args_json, char **out_err);
int chck_mail_last_status(const char *err);
void chck_mail_free(char *ptr);
void chck_mail_close(void *engine);

#ifdef __cplusplus
}
#endif
