resource "aws_sns_topic" "checker_alerts" {
  name = "${var.prefix}-checker-alerts"
}

resource "aws_sns_topic_subscription" "checker_alerts_email" {
  topic_arn = aws_sns_topic.checker_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

resource "aws_cloudwatch_metric_alarm" "checker_errors" {
  alarm_name          = "${var.prefix}-checker-errors"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  evaluation_periods  = 1
  metric_name         = "Errors"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Sum"
  threshold           = 3
  treat_missing_data  = "notBreaching"

  dimensions = {
    FunctionName = aws_lambda_function.checker.function_name
  }

  alarm_actions = [aws_sns_topic.checker_alerts.arn]

  alarm_description = "Triggers when the heartbeat checker Lambda has 3+ errors in 5 minutes"
}
