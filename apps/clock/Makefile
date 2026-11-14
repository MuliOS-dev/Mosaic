CC      = gcc
CFLAGS  = -Wall -Wextra $(shell pkg-config --cflags gtk+-3.0 libnotify)
LDLIBS  = $(shell pkg-config --libs gtk+-3.0 libnotify) -lpthread -lm -ldl

build: src/main.c
	make clean
	mkdir -p build
	$(CC) $(CFLAGS) src/main.c -o build/clock $(LDLIBS)

clean:
	rm -rf build
