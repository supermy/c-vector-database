# Makefile for minimax25 Vector Database

CC = gcc
CFLAGS = -std=c11 -O3 -Wall -Wextra
LDFLAGS = -lpthread -lm
TARGET = test_vdb
OBJS = vdb.o test_vdb.o

.PHONY: all clean test

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
