#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets with special characters and edge cases
# Tests handling of special characters in keys, values, and group names
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🎯 Testing secrets with special characters and edge cases"
echo "========================================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Testing special characters..."

# Test secrets with special characters in value
"$HOME/.cargo/bin/secrets" store special_chars 'value!@#$%^&*(){}[]|\:;"'"'"'<>?,./~`' || fail "Failed to store secret with special characters"

OUTPUT=$("$HOME/.cargo/bin/secrets" read special_chars)
[ "$OUTPUT" = 'value!@#$%^&*(){}[]|\:;"'"'"'<>?,./~`' ] || fail "Special characters not preserved. Got: '$OUTPUT'"
echo "✅ Special characters in values work correctly"

# Test unicode characters
"$HOME/.cargo/bin/secrets" store unicode_test "🔐🗝️🔒 密码 пароль كلمة المرور" || fail "Failed to store secret with unicode"

OUTPUT=$("$HOME/.cargo/bin/secrets" read unicode_test)
[ "$OUTPUT" = "🔐🗝️🔒 密码 пароль كلمة المرور" ] || fail "Unicode characters not preserved. Got: '$OUTPUT'"
echo "✅ Unicode characters work correctly"

# Test very long value
LONG_VALUE=$(printf 'a%.0s' {1..1000})
"$HOME/.cargo/bin/secrets" store long_value "$LONG_VALUE" || fail "Failed to store very long secret"

OUTPUT=$("$HOME/.cargo/bin/secrets" read long_value)
[ "$OUTPUT" = "$LONG_VALUE" ] || fail "Long value not preserved correctly"
echo "✅ Very long values work correctly"

# Test empty value
"$HOME/.cargo/bin/secrets" store empty_value "" || fail "Failed to store empty secret"

OUTPUT=$("$HOME/.cargo/bin/secrets" read empty_value)
[ "$OUTPUT" = "" ] || fail "Empty value not preserved. Got: '$OUTPUT'"
echo "✅ Empty values work correctly"

# Test values with newlines
"$HOME/.cargo/bin/secrets" store multiline_value $'line1\nline2\nline3' || fail "Failed to store multiline secret"

OUTPUT=$("$HOME/.cargo/bin/secrets" read multiline_value)
[ "$OUTPUT" = $'line1\nline2\nline3' ] || fail "Multiline value not preserved. Got: '$OUTPUT'"
echo "✅ Multiline values work correctly"

# Test keys with special characters (if allowed)
if "$HOME/.cargo/bin/secrets" store "key-with-dashes" "dash_value" 2>/dev/null; then
    OUTPUT=$("$HOME/.cargo/bin/secrets" read "key-with-dashes")
    [ "$OUTPUT" = "dash_value" ] || fail "Key with dashes not working"
    echo "✅ Keys with dashes work correctly"
fi

if "$HOME/.cargo/bin/secrets" store "key_with_underscores" "underscore_value" 2>/dev/null; then
    OUTPUT=$("$HOME/.cargo/bin/secrets" read "key_with_underscores")
    [ "$OUTPUT" = "underscore_value" ] || fail "Key with underscores not working"
    echo "✅ Keys with underscores work correctly"
fi

if "$HOME/.cargo/bin/secrets" store "key.with.dots" "dot_value" 2>/dev/null; then
    OUTPUT=$("$HOME/.cargo/bin/secrets" read "key.with.dots")
    [ "$OUTPUT" = "dot_value" ] || fail "Key with dots not working"
    echo "✅ Keys with dots work correctly"
fi

# Test group names with special characters
if "$HOME/.cargo/bin/secrets" store -g "my-service" "api-key" "service_value" 2>/dev/null; then
    OUTPUT=$("$HOME/.cargo/bin/secrets" read -g "my-service" "api-key")
    [ "$OUTPUT" = "service_value" ] || fail "Group with dashes not working"
    echo "✅ Group names with dashes work correctly"
fi

if "$HOME/.cargo/bin/secrets" store -g "prod.env" "database.url" "prod_db_url" 2>/dev/null; then
    OUTPUT=$("$HOME/.cargo/bin/secrets" read -g "prod.env" "database.url")
    [ "$OUTPUT" = "prod_db_url" ] || fail "Group with dots not working"
    echo "✅ Group names with dots work correctly"
fi

# Test case sensitivity
"$HOME/.cargo/bin/secrets" store CamelCase "camel_value" || fail "Failed to store CamelCase key"
"$HOME/.cargo/bin/secrets" store camelcase "lowercase_value" || fail "Failed to store lowercase key"

OUTPUT1=$("$HOME/.cargo/bin/secrets" read CamelCase)
OUTPUT2=$("$HOME/.cargo/bin/secrets" read camelcase)
[ "$OUTPUT1" = "camel_value" ] || fail "CamelCase key not preserved"
[ "$OUTPUT2" = "lowercase_value" ] || fail "lowercase key not preserved"
[ "$OUTPUT1" != "$OUTPUT2" ] || fail "Keys should be case sensitive"
echo "✅ Case sensitivity works correctly"

# Test values with JSON
JSON_VALUE='{"api_key": "secret123", "timeout": 30, "enabled": true}'
"$HOME/.cargo/bin/secrets" store json_config "$JSON_VALUE" || fail "Failed to store JSON value"

OUTPUT=$("$HOME/.cargo/bin/secrets" read json_config)
[ "$OUTPUT" = "$JSON_VALUE" ] || fail "JSON value not preserved. Got: '$OUTPUT'"
echo "✅ JSON values work correctly"

# Test values with XML
XML_VALUE='<?xml version="1.0"?><config><key>value</key></config>'
"$HOME/.cargo/bin/secrets" store xml_config "$XML_VALUE" || fail "Failed to store XML value"

OUTPUT=$("$HOME/.cargo/bin/secrets" read xml_config)
[ "$OUTPUT" = "$XML_VALUE" ] || fail "XML value not preserved. Got: '$OUTPUT'"
echo "✅ XML values work correctly"

# Test base64 encoded values
BASE64_VALUE="SGVsbG8gV29ybGQhIFRoaXMgaXMgYSB0ZXN0IG9mIGJhc2U2NCBlbmNvZGluZy4="
"$HOME/.cargo/bin/secrets" store base64_value "$BASE64_VALUE" || fail "Failed to store base64 value"

OUTPUT=$("$HOME/.cargo/bin/secrets" read base64_value)
[ "$OUTPUT" = "$BASE64_VALUE" ] || fail "Base64 value not preserved. Got: '$OUTPUT'"
echo "✅ Base64 values work correctly"

echo "🎉 Special characters and edge cases test completed successfully!"
