; SPDX-FileCopyrightText: 2023-2024, Dai López "dpezto"
; SPDX-FileCopyrightText: Copied verbatim from https://github.com/dpezto/tree-sitter-gnuplot
; SPDX-License-Identifier: MIT
(comment) @comment @spell

(identifier) @variable

[
  "-"
  "+"
  "~"
  "!"
  "$"
  "|"
  "**"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "<<"
  ">>"
  "&"
  "^"
  "&&"
  "||"
  "="
  ","
  "."
] @operator

[
  "eq"
  "ne"
] @keyword.operator

(ternary_expression
  [
    "?"
    ":"
  ] @conditional.ternary)

"sum" @function.builtin

[
  "for"
  "in"
  "do"
  "while"
] @keyword.repeat

[
  (c_break)
  (c_cd)
  (c_clear)
  "evaluate"
  "fit"
  "help"
  "load"
  "lower"
  "print"
  (c_replot)
  (c_reread)
  "reset"
  "splot"
  "cmd"
  "test"
  "undefine"
  "vfill"
] @keyword

(c_pause 
  "pause" @keyword
  "mouse" @field 
  _? @attribute 
  (","
   _ @attribute)?)

(c_plot
  "plot" @keyword)

(c_show
  "show" @keyword
  "plot"? @attribute)

(c_stats
  "stats" @keyword
  ("name"
   (_))? @field)

[
  "via"
  "inverse"
  "sample"
] @keyword.function

[
  "if"
  "else"
] @keyword.conditional

(plot_element
  "axes"? @field)

(cntrparam
  "auto"? @property)

(colorbox
  "origin"? @attribute)

(contourfill
  "auto"? @field)

(format
  _? @attribute
  (_)
  _? @attribute)

(key
  "auto"? @property)

(polar
  "r" @attribute)

(style ; TODO: complete
  [
    "arrow"
    "boxplot"
    "data"
    "fs"
    "function"
    "line"
    "circle"
    "rectangle"
    "ellipse"
    "parallelaxis"
    "spiderplot"
    "textbox"
    ("watchpoint"
      "labels" @attribute
      (_)?)
    "histogram"
  ] @property)

(terminal 
  "name" @property)

; TODO: complete terminals in grammar and then simplify its options here
(t_cairolatex
  [
    "eps"
    "pdf"
    "png"
    "standalone"
    "input"
    "blacktext"
    "colortext"
    "colourtext"
    ("header"
      (_))
    "mono"
    "color"
    "background"
    "rounded"
    "butt"
  ]* 
    @attribute)
; (t_canvas)
; (t_cgm)
; (t_context)
; (t_domterm)
; (t_dumb)
; (t_dxf)
; (t_emf)
; (t_epscairo)
; (t_epslatex)
; (t_fig)
; (t_gif)
; (t_hpgl)
; (t_jpeg)
; (t_lua)
; (t_pc15)
; (t_pdfcairo)
; (t_png)
; (t_pngcairo)
; (t_postscript)
; (t_pslatex)
; (t_pstricks)
; (t_qt)
; (t_sixelgd)
; (t_svg [(font_spec)]* @attribute)
; (t_tek4xxx)
; (t_texdraw)
; (t_tikz)
; (t_tkcanvas)

(plot_style
  [
    "lines"
    "points"
    "lp"
    "financebars"
    "dots"
    "impulses"
    "labels"
    "surface"
    "steps"
    "fsteps"
    "histeps"
    "arrows"
    "vectors"
    "sectors"
    "contourfill"
    "errorbar"
    "errorlines"
    "parallelaxes"
    "boxes"
    "boxerrorbars"
    "boxxyerror"
    "isosurface"
    "boxplot"
    "candlesticks"
    "circles"
    "zerrorfill"
    "ellipses"
    ("filledcurves"
     "r" @property)
    "fillsteps"
    "histograms"
    "image"
    "spiderplot"
    "pm3d"
    "rgbalpha"
    "rgbimage"
    "polygons"
    "table"
    "mask"
  ] @attribute)

[
  "tc"
  "fc"
  "fs"
  "lc"
  "ls"
  "lw"
  "lt"
  "pt"
  "ps"
  "pi"
  "pn"
  "dt"
  "as"
  "start"
  "cycles"
  "saturation"
  "interval"
  "format"
  "keywidth"
  "samplen"
  "columns"
  "title"
  "notitle"
  "every"
  "index"
  "using"
  "with"
  "frac"
  "cb"
  "arg"
  "prefix"
  "primary"
  "specular"
  "spec2"
  "firstlinetype"
  "width"
  "height"
  "expand"
  "level"
  "array"
  "dx"
  "dy"
  "dz"
  "filetype"
  "center"
  "record"
] @field

[
  (angles)
  (clip)
  (colorsequence)
  (contour)
  (encoding)
  (mapping)
  (xdata)
  (theta)
  "wall"
  "on"
  "off"
  "opaque"
  "inside"
  "outside"
  "margin"
  "cen"
  "lef"
  "rig"
  "top"
  "bot"
  "lr"
  "a"
  "maxcols"
  "maxrows"
  "autojustify"
  "overlap"
  "spread"
  "wrap"
  "swarm"
  "range"
  "label"
  "mixed"
  "triangles"
  "insidecolor"
  "noinsidecolor"
  "cycle"
  "tics"
  "ztics"
  "cbtics"
  "user"
  "front"
  "back"
  "bdefault"
  "time"
  "palette"
  "terminal"
  "onecolor"
  "invert"
  "reverse"
  "writeback"
  "extend"
  "restore"
  "linear"
  "cubicspline"
  "bspline"
  "points"
  "order"
  "levels"
  "sorted"
  "autofreq"
  "add"
  "inout"
  "axis"
  "mirror"
  "type"
  "rowsfirst"
  "columnsfirst"
  "downwards"
  "upwards"
  "prevnext"
  "gray"
  "color"
  "gamma"
  "defined"
  "cubehelix"
  "model"
  "maxcolors"
  "file"
  "colormap"
  "rgbformulae"
  "viridis"
  "positive"
  "negative"
  "nops_allcF"
  "ps_allcF"
  "quiet"
  "full"
  "trip"
  "numbers"
  "small"
  "large"
  "fullwidth"
  "append"
  "bind"
  "errors"
  "session"
  "behind"
  "polar"
  "layerdefault"
  "locale"
  "axes"
  "fix"
  "keepfix"
  "noextend"
  "head"
  "fixed"
  "filled"
  "nofilled"
  "absolute"
  "at"
  "relative"
  "enhanced"
  "border"
  "noborder"
  "rgbcolor"
  "empty"
  "black"
  "bgnd"
  "nodraw"
  "size"
  "new"
  "clustered"
  "columnstacked"
  "rowstacked"
  "nokeyseparators"
  "errorbars"
  "first"
  "second"
  "screen"
  "graph"
  "character"
  "trianglepattern"
  "undefined"
  "noundefined"
  "altdiagonal"
  "bentover"
  "vertical"
  "horizontal"
  "square"
  "ratio"
  "noratio"
  "solid"
  "transparent"
  "pattern"
  "from"
  "to_rto"
  "length"
  "angle"
  "columnheaders"
  "fortran"
  "nofpe_trap"
  "missing"
  "separator"
  "commentschars"
  "log"
  "rangelimited"
  "offset"
  "nooffset"
  "scale"
  "font"
  "point"
  "nopoint"
  "boxed"
  "noboxed"
  "hypertext"
  "defaults"
  "keyentry"
  "newhistogram"
  "newspiderplot"
  "splines"
  "qnorm"
  "gauss"
  "cauchy"
  "exp"
  "box"
  "hann"
  "theta"
  "implicit"
  "explicit"
  "rotate"
  "by"
  "parallel"
  "norotate"
  "map"
  "projection"
  "equal"
  "azimuth"
  "nohidden3d"
  "nocontours"
  "nosurface"
  "colornames"
  "functions"
  "variables"
  "version"
  "nologfile"
  "logfile"
  "fit_out"
  "errorvariables"
  "covariancevariables"
  "errorscaling"
  "prescale"
  "maxiter"
  "limit"
  "limit_abs"
  "start-lambda"
  "lambda-factor"
  "script"
  "clip"
  "noclip"
  "units"
  "fontscale"
  "lighting"
  "depthorder"
  "interpolate"
  "corners2color"
  "flush"
  "scanorder"
  "hidden3d"
  "clipcb"
  "layout"
  "margins"
  "spacing"
  "smooth"
  "binary"
  "skip"
  "bins"
  "binrange"
  "binwidth"
  "binvalue"
  "mask"
  "convexhull"
  "concavehull"
  "volatile"
  "zsort"
  "nonuniform"
  "sparse"
  "matrix"
  "output"
] @attribute

[
  "x1"
  "x2"
  "y1"
  "y2"
  "y"
  "z"
  "xx"
  "xy"
  "yy"
  "xz"
  "yz"
  "xyz"
  "x1y1"
  "x2y2"
  "x1y2"
  "x2y1"
  "columnheader"
  "seconds"
  "minutes"
  "hours"
  "days"
  "weeks"
  "months"
  "years"
  "cm"
  "in"
  "discrete"
  "incremental"
  "default"
  "long"
  "nogrid"
  "unique"
  "frequency"
  "fnormal"
  "cumulative"
  "cnormal"
  "csplines"
  "acsplines"
  "mcsplines"
  "path"
  "bezier"
  "sbezier"
  "unwrap"
  "grid"
  "kdensity"
  "closed"
  "between"
  "above"
  "below"
  "variable"
  "pixels"
  "whiskerbars"
  "RGB"
  "CMY"
  "HSV"
  "base"
  "begin"
  "center"
  "end"
  "ftriangles"
  "clip1in"
  "clip4in"
  "c2c"
  "retrace"
  "whitespace"
  "tab"
  "comma"
  "push"
  "pop"
  "flipx"
  "flipy"
  "flipz"
] @property

(colorspec
  "palette" @attribute)

(datafile_modifiers
  "origin"? @field)

((datafile_modifiers
   filetype: (identifier) @property)
    (#any-of? @property "avs""bin""edf""ehf""gif""gpbin""jpeg""jpg""png""raw""rgb""auto"))

(macro) @function.macro

(datablock) @namespace

(function
  name: (identifier) @function)

((function
  name: (identifier) @function.builtin)
  (#any-of? @function.builtin "abs""acos""acosh""airy""arg""asin""asinh""atan""atan2""atanh""besj0""besj1""besjn""besy0""besy1""besyn""besi0""besi1""besin""cbrt""ceil""conj""cos""cosh""EllipticK""EllipticE""EllipticPi""erf""erfc""exp""expint""floor""gamma""ibeta""inverf""igamma""imag""int""invnorm""invibeta""invigamma""LambertW""lambertw""lgamma""lnGamma""log""log10""norm""rand""real""round""sgn""sin""sinh""sqrt""SynchrotronF""tan""tanh""uigamma""voigt""zeta""cerf""cdawson""faddeva""erfi""FresnelC""FresnelS""VP""VP_fwhm""Ai""Bi""BesselH1""BesselH2""BesselJ""BesselY""BesselI""BesselK""gprintf""sprintf""strlen""strstrt""substr""strptime""srtftime""system""trim""word""words""time""timecolumn""tm_hour""tm_mday""tm_min""tm_mon""tm_sec""tm_wday""tm_week""tm_yday""tm_year""weekday_iso""weekday_cdc""column""columnhead""exists""hsv2rgb""index""palette""rgbcolor""stringcolumn""valid""value""voxel"))

((identifier) @variable.builtin
  (#match? @variable.builtin "^\\w+_(records|headers|outofrange|invalid|blank|blocks|columns|column_header|index_(min|max)(_x|_y)?|(min|max)(_x|_y)?|mean(_err)?(_x|_y)?|stddev(_err)?(_x|_y)?)$"))

((identifier) @variable.builtin
  (#match? @variable.builtin "^\\w+_(sdd(_x|_y)?|(lo|up)_quartile(_x|_y)?|median(_x|_y)?|sum(sq)?(_x|_y)?|skewness(_err)?(_x|_y)?)$"))

((identifier) @variable.builtin
  (#match? @variable.builtin "^\\w+_(kurtosis(_err)?(_x|_y)?|adev(_x|_y)?|correlation|slope(_err)?|intercept(_err)?|sumxy|pos(_min|_max)_y|size(_x|_y))$"))

((identifier) @variable.builtin
  (#match? @variable.builtin "^((GPVAL|MOUSE|FIT)_\\w+|GNUTERM|NaN|VoxelDistance|GridDistance|pi)$"))

(array_def "array" @keyword.function)
(array (identifier) @function)

(number) @number

(string_literal) @string
