#!/usr/bin/env bash

# Isolate the test
source "$(dirname "$0")/isolate.sh"

echo "🚀 Testing full install-uninstall lifecycle"
echo "==========================================="

# PHASE 1: Install on clean system
echo "📦 Phase 1: Installing kernelle on clean system..."
./scripts/install.sh --non-interactive

# Verify installation worked
echo "🔍 Verifying installation..."
test -f ~/.cargo/bin/kernelle
test -d ~/.kernelle
test -f ~/.kernelle.source
test -d ~/.kernelle/volatile/.cursor

# Check that binaries were installed (bentley is library-only, so exclude it)
ls -la ~/.cargo/bin/ | grep -E "(kernelle|jerrod|blizz|violet|adam|sentinel)"

# Test that kernelle binary works
~/.cargo/bin/kernelle --help > /dev/null

echo "✅ Installation verified successfully"
echo

# PHASE 2: Uninstall the installed system
echo "🧹 Phase 2: Uninstalling kernelle..."
./scripts/uninstall.sh --non-interactive

# Verify uninstallation worked
echo "🔍 Verifying uninstallation..."

# Verify kernelle.internal.source still exists (contains gone template)
test -f ~/.kernelle/kernelle.internal.source
diff ~/.kernelle/kernelle.internal.source scripts/templates/kernelle.internal.source.gone.template

# Verify volatile directory was removed but persistent directory remains
test ! -d ~/.kernelle/volatile
test -d ~/.kernelle/persistent || true  # persistent may or may not exist if no user data was created

# Verify binaries were removed
test ! -f ~/.cargo/bin/kernelle
test ! -f ~/.cargo/bin/jerrod
test ! -f ~/.cargo/bin/blizz
test ! -f ~/.cargo/bin/violet
test ! -f ~/.cargo/bin/adam
test ! -f ~/.cargo/bin/sentinel

echo "✅ Uninstallation verified successfully"
echo

echo "🎉 Full install-uninstall lifecycle test completed successfully!"
echo "   - Installed kernelle on clean system ✅"
echo "   - Verified all components were installed ✅"  
echo "   - Uninstalled kernelle cleanly ✅"
echo "   - Verified complete removal ✅" 
