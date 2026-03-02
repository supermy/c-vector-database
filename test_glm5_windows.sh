#!/bin/bash
# Test Windows cross-compilation for glm5

echo "=== Testing glm5 Windows cross-compilation ==="

# Clean
cd glm5
make clean

# Try to compile with MinGW if available
if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "MinGW found, testing Windows cross-compilation..."
    x86_64-w64-mingw32-gcc -std=c11 -O3 -Wall -Wextra -c glm5_vdb.c -o glm5_vdb_test.o 2>&1 | grep -i "error" || echo "✅ Cross-compilation successful"
    rm -f glm5_vdb_test.o
else
    echo "MinGW not found, skipping cross-compilation test"
    echo "Testing native compilation instead..."
    make 2>&1 | grep -i "error" || echo "✅ Native compilation successful"
fi

cd ..
echo "=== Test complete ==="
