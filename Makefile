APP_NAME = picam
VERSION = 1.0.0
BUILD_DIR = build
DEB_DIR = $(BUILD_DIR)/deb
INSTALL_DIR = /opt/$(APP_NAME)
BIN_DIR = $(DEB_DIR)$(INSTALL_DIR)
SYSTEMD_DIR = /etc/systemd/system
DEBIAN_DIR = debian

.PHONY: all clean build package install service

all: build package

clean:
	rm -rf $(BUILD_DIR)

build:
	cargo build --release
	mkdir -p $(BIN_DIR)
	cp target/release/$(APP_NAME) $(BIN_DIR)
	cp -r frontend $(BIN_DIR)

package: build
	mkdir -p $(DEB_DIR)/DEBIAN
	cp $(DEBIAN_DIR)/control $(DEB_DIR)/DEBIAN/control
	dpkg-deb --build $(DEB_DIR) $(BUILD_DIR)/$(APP_NAME)_$(VERSION).deb

install: package
	dpkg -i $(BUILD_DIR)/$(APP_NAME)_$(VERSION).deb

service:
	mkdir -p $(SYSTEMD_DIR)
	cp $(DEBIAN_DIR)/$(APP_NAME).service $(SYSTEMD_DIR)/$(APP_NAME).service
	systemctl daemon-reload
	systemctl enable $(APP_NAME)
	systemctl start $(APP_NAME)
