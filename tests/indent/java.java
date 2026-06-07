package example;

import java.util.List;

public class Example {
  private int value;

  public Example(int value) {
    this.value = value;
  }

  public int compute(int a, int b) {
    int[] numbers = {
      1,
      2,
      3,
    };
    for (int i = 0; i < numbers.length; i++) {
      if (numbers[i] > a) {
        System.out.println(numbers[i]);
      } else {
        a += numbers[i];
      }
    }
    while (a > 0)
      a--;
    if (a > 5)
      a -= 5;
    for (int i = 0; i < b; i++)
      a += i;
    switch (b) {
      case 1:
        return a;
      case 2:
        return b;
      default:
        return 0;
    }
  }
}

class Braceless {
  void m(boolean cond) {
    if (cond)
      doThing();
    else
      other();
  }
}
