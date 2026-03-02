# Makefile for kimi25 Vector Database (HNSW)

CC = gcc
CFLAGS = -std=c11 -O3 -Wall -Wextra
LDFLAGS = -lpthread -lm
TARGET = test_vector_db
OBJS = vector_db.o test_vector_db.o

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
