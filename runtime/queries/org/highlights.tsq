(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "*")) @markup.heading.1
(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "**")) @markup.heading.2
(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "***")) @markup.heading.3
(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "****")) @markup.heading.4
(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "*****")) @markup.heading.5
(headline (stars) @markup.heading.marker (#eq? @markup.heading.marker "******")) @markup.heading.6

(block) @markup.raw.block
(list) @markup.list.unnumbered
(directive) @markup.label
(property_drawer) @markup.label
 

((expr) @markup.bold
 (#match? @markup.bold "\\*.*\\*"))

((expr) @markup.italic
 (#match? @markup.italic "/.*/"))
((expr) @markup.raw.inline
 (#match? @markup.raw.inline "~.*~"))

((expr) @markup.quote
 (#match? @markup.quote "=.*="))

