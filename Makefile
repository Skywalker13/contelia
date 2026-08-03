CARGO    ?= cargo
PROFILE  ?= release
TARGET   ?=
DESTDIR  ?=
PREFIX   ?= /usr/local

ifeq ($(TARGET),)
  BIN_PATH    := target/$(PROFILE)/contelia
else
  TARGET_FLAG := --target $(TARGET)
  BIN_PATH    := target/$(TARGET)/$(PROFILE)/contelia
endif

BINDIR := $(DESTDIR)$(PREFIX)/bin
SHAREDIR := $(DESTDIR)$(PREFIX)/share/contelia

.PHONY: build test install install-bin install-www clean

build:
	$(CARGO) build --$(PROFILE) $(TARGET_FLAG)

test:
	$(CARGO) test $(TARGET_FLAG)

install-bin: build
	install -D -m755 $(BIN_PATH) $(BINDIR)/contelia
	install -d $(SHAREDIR)/assets
	install -m644 assets/*.png $(SHAREDIR)/assets/
	cp -r books $(SHAREDIR)/

install-www:
	$(MAKE) -C www install DESTDIR=$(DESTDIR)

install: install-bin install-www

clean:
	$(CARGO) clean
	$(MAKE) -C www clean
