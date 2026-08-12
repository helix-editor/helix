{
  # Simple bindings.
  a = 1;
  b = "two";
  c = a + 1;

  # Nested attrsets.
  nested = {
    c = 3;
    deep = {
      d = 4;
      deeper = {
        e = 5;
      };
    };
  };

  # Lists, including nested.
  list = [
    1
    2
    [
      3
      4
    ]
  ];

  # rec attrset.
  recursive = rec {
    x = 1;
    y = x + 1;
  };

  # let ... in: bindings indent, `in` and the body sit at the `let` column.
  computed =
    let
      x = 1;
      y = 2;
    in
    x + y;

  # Nested let.
  nestedLet =
    let
      outer =
        let
          inner = 1;
        in
        inner;
    in
    outer;

  # Functions.
  id = x: x;
  add = a: b: a + b;

  # Pattern parameter.
  pattern =
    {
      foo,
      bar,
      ...
    }:
    foo + bar;

  # Pattern with `@` binding.
  patternAt =
    {
      foo,
      bar,
    }@args:
    foo + bar + args.foo;

  # if / then / else, including else-if chains.
  sign =
    if a > 0 then
      "positive"
    else if a < 0 then
      "negative"
    else
      "zero";

  # with.
  withExpr =
    with builtins;
    length [
      1
      2
    ];

  # assert.
  checked =
    assert a > 0;
    a;

  # Function application across lines.
  applied =
    builtins.map
      (x: x + 1)
      [
        1
        2
      ];

  # Operator continuation via the binding value.
  merged =
    {
      a = 1;
    }
    // {
      b = 2;
    };

  hasAttr = { a = 1; } ? a;

  # Parenthesized expression across lines.
  grouped = (
    1
    + 2
    + 3
  );

  # Indented string with a non-script name: not injected, so its interior is
  # preserved verbatim by @opaque (author indentation kept as-is).
  notes = ''
    first line
      a deeper line, preserved verbatim
    back to the base
  '';

  # A realistic derivation.
  package = stdenv.mkDerivation rec {
    pname = "hello";
    version = "1.0";

    src = fetchurl {
      url = "https://example.com/${pname}-${version}.tar.gz";
      sha256 = "0000";
    };

    buildInputs = [
      cmake
      ninja
    ];

    buildPhase = ''
      make
      make install
    '';

    meta = {
      description = "A program";
      license = licenses.mit;
    };
  };

  # inherit variants.
  inherit a b;
  inherit (builtins) length map;
}
