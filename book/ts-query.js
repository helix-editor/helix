hljs.registerLanguage("tsq", function (hljs) {
  const IDENT = /[A-Za-z_][\w-]*/;
  return {
    name: "Tree-sitter Query",
    aliases: ["scm"],
    contains: [
      hljs.COMMENT(";", "$"),
      hljs.QUOTE_STRING_MODE,
      // @capture.name (with dots and dashes)
      { className: "variable", begin: /@[\w.\-]+/ },
      // #predicate? — #eq?, #match?, #any-of?, #kind-eq?, #same-line?, ...
      { className: "built_in", begin: /#[\w\-]+\?/ },
      // #directive! — #set!, #select-adjacent!, #strip!, #trim!, #gsub!
      { className: "meta", begin: /#[\w\-]+!/ },
      // field name:   or negated !field:
      { className: "attr", begin: /!?[A-Za-z_][\w-]*:/ },
      // wildcard and special node kinds
      { className: "literal", begin: /\b(?:_|MISSING|ERROR)\b/ },
      // named node kind (must come after field/wildcard rules)
      { className: "title", begin: IDENT },
      // anchors and quantifiers: .  *  +  ?
      { className: "operator", begin: /[.*+?]/ },
    ],
  };
});
