data "archive_file" "placeholder" {
  type        = "zip"
  output_path = "${path.module}/placeholder.zip"

  source {
    content  = "#!/bin/sh\nexit 0"
    filename = "bootstrap"
  }
}

resource "aws_lambda_function" "checker" {
  function_name = "${var.prefix}-checker"
  role          = aws_iam_role.lambda_exec.arn

  runtime       = "provided.al2023"
  architectures = ["arm64"]
  handler       = "bootstrap"
  memory_size   = 128
  timeout       = 30

  filename         = data.archive_file.placeholder.output_path
  source_code_hash = data.archive_file.placeholder.output_base64sha256

  reserved_concurrent_executions = 1

  environment {
    variables = {
      HEARTBEAT_TABLE_NAME          = aws_dynamodb_table.monitors.name
      HEARTBEAT_API_KEYS_TABLE_NAME = aws_dynamodb_table.api_keys.name
      TELEGRAM_BOT_TOKEN_PARAM      = aws_ssm_parameter.telegram_bot_token.name
      TELEGRAM_CHAT_ID_PARAM        = aws_ssm_parameter.telegram_chat_id.name
    }
  }

  lifecycle {
    ignore_changes = [filename, source_code_hash]
  }
}

resource "aws_cloudwatch_log_group" "checker" {
  name              = "/aws/lambda/${var.prefix}-checker"
  retention_in_days = 14
}
