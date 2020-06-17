TARGET = example
LIBS = -lm -L../../target/release -lresvg_cairo `pkg-config --libs gtk+-3.0`
CC = gcc
CFLAGS = -g -Wall `pkg-config --cflags gtk+-3.0` -I../../c-api/

.PHONY: default all clean

default: $(TARGET)
all: default

OBJECTS = $(patsubst %.c, %.o, $(wildcard *.c))

%.o: %.c $(CC) $(CFLAGS) -c $< -o $@

.PRECIOUS: $(TARGET) $(OBJECTS)

$(TARGET): $(OBJECTS)
	$(CC) $(OBJECTS) -Wall $(LIBS) -o $@

clean:
	-rm -f *.o
	-rm -f $(TARGET)
