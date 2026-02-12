variable "prefix" {
  description = "Prefix for all resource names"
  type        = string
  default     = "heartbeat"
}

variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "eu-north-1"
}

variable "telegram_bot_token" {
  description = "Telegram bot token for sending alert notifications"
  type        = string
  sensitive   = true
}

variable "telegram_chat_id" {
  description = "Telegram chat ID to receive alert notifications"
  type        = string
  sensitive   = true
}

variable "alert_email" {
  description = "Email address for CloudWatch alarm notifications"
  type        = string
  sensitive   = true
}
