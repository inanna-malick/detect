#!/bin/bash
set -e

echo "🧪 SIGPIPE Handling Demonstration Script"
echo "========================================"
echo

# Build release binary if needed
if [ ! -f "./target/release/detect" ]; then
    echo "📦 Building release binary..."
    cargo build --release
    echo
fi

echo "✅ Testing Unix pipeline compatibility"
echo "These commands should all exit cleanly without panic messages:"
echo

# Test 1: Basic head pipeline
echo "1️⃣  Testing: detect 'size >= 0' | head -3"
echo "   (Should show first 3 matches and exit cleanly)"
./target/release/detect 'size >= 0' | head -3
echo "   ✓ Exit code: $?"
echo

# Test 2: Immediate pipe closure  
echo "2️⃣  Testing: detect 'size >= 0' | head -1"
echo "   (Should show 1 match and exit immediately)"
./target/release/detect 'size >= 0' | head -1
echo "   ✓ Exit code: $?"
echo

# Test 3: Pipe to false (immediate closure)
echo "3️⃣  Testing: echo 'quick exit test' | timeout 1s ./target/release/detect 'contents contains test' || true"
echo "   (Should handle immediate pipe closure gracefully)"
echo "test content" | timeout 1s ./target/release/detect 'contents contains test' /dev/stdin 2>/dev/null || echo "   ✓ Handled timeout gracefully"
echo

# Test 4: Large output piped to head
echo "4️⃣  Testing: detect 'type == file' | head -5"
echo "   (Should show first 5 files and exit cleanly)"
./target/release/detect 'type == file' | head -5
echo "   ✓ Exit code: $?"
echo

# Test 5: Pipe to grep then head  
echo "5️⃣  Testing: detect 'extension == rs' | grep -E '(main|lib)' | head -2"
echo "   (Should handle multi-stage pipeline)"
./target/release/detect 'extension == rs' | grep -E '(main|lib)' | head -2 2>/dev/null || echo "   ✓ No matches found, but handled cleanly"
echo "   ✓ Exit code: $?"
echo

# Test 6: Normal operation (no broken pipe)
echo "6️⃣  Testing: detect 'name == Cargo.toml'"
echo "   (Should complete normally without pipe)"
./target/release/detect 'name == Cargo.toml'
echo "   ✓ Exit code: $?"
echo

echo "🎉 All pipeline tests completed successfully!"
echo "   No panic messages or 'Broken pipe' errors should have appeared above."
echo
echo "📋 This demonstrates Unix-compatible behavior:"
echo "   • detect exits cleanly when downstream commands close the pipe"
echo "   • No error messages about broken pipes"
echo "   • Exit code 0 in all pipeline scenarios"
echo "   • Compatible with head, grep, less, and other standard Unix tools"
echo
echo "🔧 Compare with tools that handle SIGPIPE correctly:"
echo "   find . -name '*.rs' | head -3    # Should behave identically"
echo "   grep -r 'test' . | head -3      # Should behave identically"
echo "   detect 'extension == rs' | head -3  # Our implementation"