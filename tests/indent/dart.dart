class Example {
//             ^ 2 eof
  String classify(int value) {
//                            ^ 4 eof
    if (a) {}
//          ^ 4
    switch (value) {
      case 1:
        return "one";
      case 2:
        print("two");
        return "";
      default:
        return "other";
    }
  }
}

var multi = '''
unindented
  indented
''';
