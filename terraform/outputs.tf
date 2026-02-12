output "monitors_table_name" {
  description = "Name of the DynamoDB monitors table"
  value       = aws_dynamodb_table.monitors.name
}

output "monitors_table_arn" {
  description = "ARN of the DynamoDB monitors table"
  value       = aws_dynamodb_table.monitors.arn
}

output "api_keys_table_name" {
  description = "Name of the DynamoDB API keys table"
  value       = aws_dynamodb_table.api_keys.name
}

output "api_keys_table_arn" {
  description = "ARN of the DynamoDB API keys table"
  value       = aws_dynamodb_table.api_keys.arn
}

output "lambda_function_name" {
  description = "Name of the Lambda checker function"
  value       = aws_lambda_function.checker.function_name
}

output "lambda_function_arn" {
  description = "ARN of the Lambda checker function"
  value       = aws_lambda_function.checker.arn
}

output "api_user_name" {
  description = "Name of the IAM user for API access"
  value       = aws_iam_user.api.name
}

output "lambda_role_arn" {
  description = "ARN of the Lambda execution role"
  value       = aws_iam_role.lambda_exec.arn
}
