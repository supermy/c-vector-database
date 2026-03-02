# Makefile for glm5 Vector Database

CC = gcc
CFLAGS = -std=c11 -O3 -Wall -Wextra -lpthread -lm
TARGET = test_glm5
OBJS = glm5_vdb.o test_glm5.o

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
