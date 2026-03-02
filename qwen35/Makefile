# Makefile for qwen35 Vector Database
# Cross-platform build with automatic SIMD detection

CC ?= gcc
CXX ?= g++
AR ?= ar

# Detect platform and set appropriate flags
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

# Base flags
CFLAGS_BASE = -std=c11 -O3 -Wall -Wextra
LDFLAGS_BASE = -lpthread -lm

# Platform-specific flags
ifeq ($(UNAME_S),Darwin)
    # macOS
    CFLAGS_PLATFORM = -I/usr/local/include
    LDFLAGS_PLATFORM = -L/usr/local/lib
    ifeq ($(UNAME_M),arm64)
        # Apple Silicon (ARM64)
        CFLAGS_SIMD = -DQWEN35_SIMD_NEON
        ARCH_FLAGS = -target arm64-apple-macos11
    else
        # Intel Mac (x86_64)
        CFLAGS_SIMD = -mavx -mavx2 -msse4.2
        ARCH_FLAGS = -target x86_64-apple-macos10.14
    endif
else ifeq ($(UNAME_S),Linux)
    # Linux
    CFLAGS_PLATFORM =
    LDFLAGS_PLATFORM =
    ifeq ($(UNAME_M),aarch64)
        # ARM64 Linux
        CFLAGS_SIMD = -DQWEN35_SIMD_NEON
    else
        # x86_64 Linux - use SSE4.2 only for better compatibility
        CFLAGS_SIMD = -msse4.2
    endif
else
    # Fallback
    CFLAGS_PLATFORM =
    LDFLAGS_PLATFORM =
    CFLAGS_SIMD =
endif

# Combine all flags
CFLAGS = $(CFLAGS_BASE) $(CFLAGS_PLATFORM) $(CFLAGS_SIMD) $(ARCH_FLAGS)
LDFLAGS = $(LDFLAGS_BASE) $(LDFLAGS_PLATFORM)

TARGET = test_qwen35
OBJS = qwen35_vdb.o test_qwen35.o

.PHONY: all clean test install benchmark info

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(OBJS) -o $(TARGET) $(LDFLAGS)

%.o: %.c
	$(CC) -c $< -o $@ $(CFLAGS)

test: $(TARGET)
	./$(TARGET)

clean:
	rm -f $(OBJS) $(TARGET) *.o

install: $(TARGET)
	cp $(TARGET) /usr/local/bin/

benchmark: $(TARGET)
	./$(TARGET) --benchmark

info:
	@echo "Platform: $(UNAME_S) $(UNAME_M)"
	@echo "Compiler: $(CC)"
	@echo "CFLAGS: $(CFLAGS)"
	@echo "LDFLAGS: $(LDFLAGS)"

# Print SIMD info
simd-info:
	@echo "=== SIMD Configuration ==="
	@echo "Platform: $(UNAME_S) $(UNAME_M)"
ifeq ($(UNAME_S),Darwin)
    ifeq ($(UNAME_M),arm64)
		@echo "Architecture: ARM64 (Apple Silicon)"
		@echo "SIMD: NEON"
    else
		@echo "Architecture: x86_64 (Intel)"
		@echo "SIMD: AVX/AVX2/SSE4.2"
    endif
else ifeq ($(UNAME_S),Linux)
    ifeq ($(UNAME_M),aarch64)
		@echo "Architecture: ARM64"
		@echo "SIMD: NEON"
    else
		@echo "Architecture: x86_64"
		@echo "SIMD: AVX/AVX2/SSE4.2"
    endif
endif
