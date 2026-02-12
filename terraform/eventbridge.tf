resource "aws_cloudwatch_event_rule" "checker_schedule" {
  name                = "${var.prefix}-checker-schedule"
  schedule_expression = "rate(2 minutes)"
  is_enabled          = false
}

resource "aws_cloudwatch_event_target" "checker" {
  rule = aws_cloudwatch_event_rule.checker_schedule.name
  arn  = aws_lambda_function.checker.arn
}

resource "aws_lambda_permission" "eventbridge" {
  statement_id  = "${var.prefix}-checker-eventbridge"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.checker.function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.checker_schedule.arn
}
