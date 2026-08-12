CC = gcc

all: foo bar
	$(CC) -o app foo.o bar.o

foo.o: foo.c
	$(CC) -c foo.c
