CC = gcc
CFLAGS = -Wall -Wextra -O2 -std=c99
LDFLAGS = -lm

TARGET = test_vector_db
SRCS = vector_db.c test_vector_db.c
OBJS = $(SRCS:.c=.o)

.PHONY: all clean test

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(OBJS) -o $(TARGET) $(LDFLAGS)

%.o: %.c vector_db.h
	$(CC) $(CFLAGS) -c $< -o $@

test: $(TARGET)
	./$(TARGET)

clean:
	rm -f $(OBJS) $(TARGET) test_db.bin

# 调试版本
debug: CFLAGS = -Wall -Wextra -g -O0 -std=c99 -DDEBUG
debug: clean $(TARGET)

# 性能分析版本
profile: CFLAGS = -Wall -Wextra -O2 -std=c99 -pg
profile: LDFLAGS = -lm -pg
profile: clean $(TARGET)
