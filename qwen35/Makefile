# Makefile for qwen35 Vector Database

CC = gcc
CFLAGS = -std=c11 -O3 -Wall -Wextra -mavx -lpthread -lm
TARGET = test_qwen35
OBJS = qwen35_vdb.o test_qwen35.o

.PHONY: all clean test

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(OBJS) -o $(TARGET) $(CFLAGS)

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
