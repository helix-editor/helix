class Example {
  String classify(int value) {
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
