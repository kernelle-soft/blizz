#!/usr/bin/env bash

set -euo pipefail
set -x

# Isolate the test
source "$(dirname "$0")/isolate.sh"

echo "🚀 Testing full install-uninstall lifecycle"
echo "==========================================="

# PHASE 1: Install on clean system
echo "📦 Phase 1: Installing kernelle on clean system..."
./scripts/install.sh --non-interactive || fail "Install script failed"

 # Verify installation worked
echo "🔍 Verifying installation..."
test -f "$HOME/.cargo/bin/kernelle" || fail "kernelle binary not found after install"
test -d ~/.kernelle || fail "~/.kernelle directory not found after install"
test -f ~/.kernelle.source || fail "~/.kernelle.source not found after install"
test -d ~/.kernelle/volatile/.cursor || fail "~/.kernelle/volatile/.cursor not found after install"


# Check that binaries were installed (bentley is library-only, so exclude it)
ls -la "$HOME/.cargo/bin/" | grep -E "(kernelle|jerrod|blizz|violet|adam|sentinel)" || fail "Expected binaries not found in ~/.cargo/bin"


# Test that kernelle binary works
"$HOME/.cargo/bin/kernelle" --help > /dev/null || fail "kernelle --help failed"

echo "✅ Installation verified successfully"
echo

# PHASE 2: Uninstall the installed system
echo "🧹 Phase 2: Uninstalling kernelle..."
./scripts/uninstall.sh || fail "Uninstall script failed"

# Verify uninstallation worked
echo "🔍 Verifying uninstallation..."

# Verify kernelle.internal.source still exists (contains gone template)
test -f ~/.kernelle/kernelle.internal.source || fail "kernelle.internal.source not found after uninstall"
diff ~/.kernelle/kernelle.internal.source scripts/templates/kernelle.internal.source.gone.template || fail "kernelle.internal.source does not match gone template"

# Verify volatile directory was removed but persistent directory remains
test ! -d ~/.kernelle/volatile || fail "~/.kernelle/volatile directory still exists after uninstall"
test -d ~/.kernelle/persistent || true  # persistent may or may not exist if no user data was created


# Verify binaries were removed
test ! -f "$HOME/.cargo/bin/kernelle" || fail "kernelle binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/jerrod" || fail "jerrod binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/blizz" || fail "blizz binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/violet" || fail "violet binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/adam" || fail "adam binary still exists after uninstall"
test ! -f "$HOME/.cargo/bin/sentinel" || fail "sentinel binary still exists after uninstall"

echo "✅ Uninstallation verified successfully"
echo

echo "🎉 Full install-uninstall lifecycle test completed successfully!"
echo "   - Installed kernelle on clean system ✅"
echo "   - Verified all components were installed ✅"  
echo "   - Uninstalled kernelle cleanly ✅"
echo "   - Verified complete removal ✅" 
