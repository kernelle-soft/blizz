#!/usr/bin/env bash

set -euo pipefail
set -x

# Isolate the test
source "$(dirname "$0")/isolate.sh"

echo "🚀 Testing full install-uninstall lifecycle"
echo "==========================================="

# PHASE 1: Install on clean system
echo "📦 Phase 1: Installing blizz on clean system..."
./scripts/install.sh --non-interactive --from-source || fail "Install script failed"

 # Verify installation worked
echo "🔍 Verifying installation..."
test -f "$HOME/.cargo/bin/blizz" || fail "blizz binary not found after install"
test -d ~/.blizz || fail "~/.blizz directory not found after install"
test -f ~/.blizz.source || fail "~/.blizz.source not found after install"
test -d ~/.blizz/volatile/.cursor || fail "~/.blizz/volatile/.cursor not found after install"
test -f ~/.blizz/uninstall.sh || fail "~/.blizz/uninstall.sh not found after install"
test -f ~/.blizz/volatile/blizz.internal.source.gone.template || fail "~/.blizz/volatile/blizz.internal.source.gone.template not found after install"

# Check that binaries were installed (bentley is library-only, so exclude it)
ls -la "$HOME/.cargo/bin/" | grep -E "(blizz|violet|adam|sentinel)" || fail "Expected binaries not found in ~/.cargo/bin"

# Test that Blizz works
"$HOME/.cargo/bin/blizz" --help > /dev/null || fail "blizz --help failed"

echo "✅ Installation verified successfully"
echo

# PHASE 2: Uninstall the installed system
echo "🧹 Phase 2: Uninstalling blizz..."
./scripts/uninstall.sh || fail "Uninstall script failed"

# Verify uninstallation worked
echo "🔍 Verifying uninstallation..."

# Verify blizz.internal.source still exists (contains gone template)
test -f ~/.blizz/blizz.internal.source || fail "blizz.internal.source not found after uninstall"
diff ~/.blizz/blizz.internal.source scripts/templates/blizz.internal.source.gone.template || fail "blizz.internal.source does not match gone template"

# Verify uninstaller and template were self-cleaned
test ! -f ~/.blizz/uninstall.sh || fail "~/.blizz/uninstall.sh still exists after uninstall"
test ! -f ~/.blizz/volatile/blizz.internal.source.gone.template || fail "~/.blizz/volatile/blizz.internal.source.gone.template still exists after uninstall"

# Verify volatile directory was removed but persistent directory remains
test ! -d ~/.blizz/volatile || fail "~/.blizz/volatile directory still exists after uninstall"
test -d ~/.blizz/persistent || true  # persistent may or may not exist if no user data was created


# Verify binaries were removed
test ! -f "$HOME/.cargo/bin/blizz" || fail "blizz binary still exists after uninstall"

test ! -f "$HOME/.cargo/bin/violet" || fail "violet binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/adam" || fail "adam binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/sentinel" || fail "sentinel binary still exists after uninstall"

echo "✅ Uninstallation verified successfully"
echo

echo "🎉 Full install-uninstall lifecycle test completed successfully!"
echo "   - Installed blizz on clean system ✅"
echo "   - Verified all components were installed ✅"  
echo "   - Uninstalled blizz cleanly ✅"
echo "   - Verified complete removal ✅" 
