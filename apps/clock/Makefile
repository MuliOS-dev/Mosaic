CC      = gcc
CFLAGS  = -Wall -Wextra $(shell pkg-config --cflags gtk+-3.0 libnotify)
LDLIBS  = $(shell pkg-config --libs gtk+-3.0 libnotify) -lpthread -lm -ldl

build: src/main.c
	make clean
	mkdir -p build
	$(CC) $(CFLAGS) src/main.c -o build/clock $(LDLIBS)

clean:
	rm -rf build

install:
	make build
	sudo mkdir -p /opt/mosaic/clock
	sudo cp build/clock /opt/mosaic/clock
	sudo cp -r resources/ /opt/mosaic/clock
	sudo cp -r include/ /opt/mosaic/clock
	sudo ln -s /opt/mosaic/clock/clock /bin/mosaic-clock
	cp clock.desktop ~/.local/share/applications/clock.desktop
	chmod +x ~/.local/share/applications/clock.desktop
	sudo cp -r src/ /opt/mosaic/clock/
	sudo chown "$$(id -un):$$(id -gn)" /opt/mosaic/clock/include/clockstate /opt/mosaic/clock/include/deftheme

reinstall:
	sudo rm -rf /opt/mosaic/clock/
	sudo rm -rf /bin/mosaic-clock
	make install
