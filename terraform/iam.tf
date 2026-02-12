data "aws_iam_policy_document" "lambda_assume_role" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["lambda.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "lambda_exec" {
  name               = "${var.prefix}-checker-role"
  assume_role_policy = data.aws_iam_policy_document.lambda_assume_role.json
}

resource "aws_iam_role_policy_attachment" "lambda_basic_execution" {
  role       = aws_iam_role.lambda_exec.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

data "aws_iam_policy_document" "checker_dynamodb" {
  statement {
    sid = "DynamoDBAccess"
    actions = [
      "dynamodb:Scan",
      "dynamodb:Query",
      "dynamodb:UpdateItem",
      "dynamodb:GetItem",
    ]
    resources = [
      aws_dynamodb_table.monitors.arn,
      "${aws_dynamodb_table.monitors.arn}/index/*",
    ]
  }

  statement {
    sid = "SSMAccess"
    actions = [
      "ssm:GetParameter",
    ]
    resources = [
      aws_ssm_parameter.telegram_bot_token.arn,
      aws_ssm_parameter.telegram_chat_id.arn,
    ]
  }
}

resource "aws_iam_role_policy" "checker_dynamodb" {
  name   = "${var.prefix}-checker-dynamodb"
  role   = aws_iam_role.lambda_exec.id
  policy = data.aws_iam_policy_document.checker_dynamodb.json
}

resource "aws_iam_user" "api" {
  name = "${var.prefix}-api"
}

data "aws_iam_policy_document" "api_dynamodb" {
  statement {
    sid = "DynamoDBAccess"
    actions = [
      "dynamodb:PutItem",
      "dynamodb:GetItem",
      "dynamodb:UpdateItem",
      "dynamodb:DeleteItem",
      "dynamodb:Scan",
      "dynamodb:Query",
    ]
    resources = [
      aws_dynamodb_table.monitors.arn,
      "${aws_dynamodb_table.monitors.arn}/index/*",
      aws_dynamodb_table.api_keys.arn,
    ]
  }
}

resource "aws_iam_user_policy" "api_dynamodb" {
  name   = "${var.prefix}-api-dynamodb"
  user   = aws_iam_user.api.name
  policy = data.aws_iam_policy_document.api_dynamodb.json
}
