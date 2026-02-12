resource "aws_ssm_parameter" "telegram_bot_token" {
  name  = "/${var.prefix}/telegram-bot-token"
  type  = "SecureString"
  value = var.telegram_bot_token

  tags = {
    Name = "${var.prefix}-telegram-bot-token"
  }
}

resource "aws_ssm_parameter" "telegram_chat_id" {
  name  = "/${var.prefix}/telegram-chat-id"
  type  = "SecureString"
  value = var.telegram_chat_id

  tags = {
    Name = "${var.prefix}-telegram-chat-id"
  }
}
