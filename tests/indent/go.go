package main

import (
	"fmt"
	"strings"
)

type Point struct {
	X int
	Y int
}

type Empty struct {}
//                 ^ 0

func empty() {
	switch nil {}
//              ^ 4
}

func compute(a int, b int) int {
	values := []int{
		1,
		2,
		3,
	}
	lookup := map[string]int{
		"a": 1,
		"b": 2,
	}
	for i, v := range values {
		if v > a {
			fmt.Println(i, v)
		} else {
			fmt.Println(lookup["a"])
		}
	}
	switch a {
	case 1:
//         ^ 8 eof
		return b
//              ^ 8
	case 2:
		return a
//              ^ 8
	default:
//          ^ 8 eof
		return 0
//              ^ 8
	}
}

func pipe(ch chan int) {
	select {
	case msg := <-ch:
		fmt.Println(msg)
//                      ^ 8
	default:
		fmt.Println(strings.TrimSpace("none"))
//                                            ^ 8
	}
Loop:
	for {
		break Loop
	}
}

type Shape interface {
	Area() float64
	Perimeter() float64
}

type Circle struct {
	Radius float64
}

func (c Circle) Area() float64 {
	return 3.14 * c.Radius * c.Radius
}

const (
	StatusOK    = 200
	StatusError = 500
)

var (
	count   int
	enabled bool
)

func process(items []int) (int, error) {
	defer func() {
		recover()
	}()

	sum := 0
	for _, v := range items {
		switch x := any(v).(type) {
		case int:
			sum += x
//                  ^ 12
		default:
			continue
//                  ^ 12
		}
	}

	go func() {
		fmt.Println(sum)
	}()

	point := struct {
		X, Y int
	}{
		X: 1,
		Y: 2,
	}

	return point.X, nil
}

func rawString() string {
	query := `
first
	indented in raw
last`
	return query
}
