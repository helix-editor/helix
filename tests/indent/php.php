<?php

function classify($value)
{
    if ($a) {}
//           ^ 4
    switch ($value) {
        case 1:
            return "one";
        case 2:
            echo "two";
            return "";
        default:
            return "other";
    }
}

$doc = <<<EOT
unindented
    indented
EOT;
