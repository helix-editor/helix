<?php

function classify($value)
{
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
