resource "aws_dynamodb_table" "monitors" {
  name         = "${var.prefix}-monitors"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "slug"

  attribute {
    name = "slug"
    type = "S"
  }

  attribute {
    name = "check_partition"
    type = "S"
  }

  attribute {
    name = "next_due"
    type = "N"
  }

  global_secondary_index {
    name            = "overdue-check-index"
    hash_key        = "check_partition"
    range_key       = "next_due"
    projection_type = "ALL"
  }

  ttl {
    attribute_name = "expires_at"
    enabled        = true
  }

  tags = {
    Name = "${var.prefix}-monitors"
  }
}

resource "aws_dynamodb_table" "api_keys" {
  name         = "${var.prefix}-api-keys"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "api_key"

  attribute {
    name = "api_key"
    type = "S"
  }

  tags = {
    Name = "${var.prefix}-api-keys"
  }
}
