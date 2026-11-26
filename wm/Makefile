# BraveWM - custom window manager for BraveOS (C++17, Xlib + Cairo + Pango)
BINS  := bravos-wm
SRCS  := src/main.cpp src/wm.cpp src/client.cpp src/events.cpp \
	 src/menu.cpp src/switcher.cpp src/bar.cpp src/i18n.cpp src/util.cpp
OBJS  := $(SRCS:.cpp=.o)
CXX   ?= g++
CXXFLAGS ?= -O2 -std=c++17 -Wall -Wextra -Wno-unused-parameter
CXXFLAGS += $(shell pkg-config --cflags pangocairo 2>/dev/null)
LDFLAGS ?=
LIBS  := -lX11 -lXext -lcairo -lpangocairo-1.0 -lpango-1.0 -lgobject-2.0 -ldl

all: $(BINS)

bravos-wm: $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $(OBJS) $(LIBS)

src/%.o: src/%.cpp src/*.hpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

clean:
	rm -f $(OBJS) $(BINS)

install: bravos-wm
	install -Dm755 bravos-wm $(DESTDIR)/usr/local/bin/bravos-wm

.PHONY: all clean install
