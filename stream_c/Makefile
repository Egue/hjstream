CC = gcc
CFLAGS = -Wall -O3 -pthread -march=native
LDFLAGS = -lavformat -lavcodec -lavutil -lpthread -lm

TARGET = stream_relay
SRC = stream_relay.c

all: $(TARGET)

$(TARGET): $(SRC)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRC) $(LDFLAGS)

clean:
	rm -f $(TARGET)

install:
	cp $(TARGET) /usr/local/bin/
	chmod +x /usr/local/bin/$(TARGET)

.PHONY: all clean install
