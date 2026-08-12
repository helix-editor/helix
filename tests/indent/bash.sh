#!/usr/bin/env bash

greet() {
  local name="$1"
  echo "hello, $name"
}

if [ -d "$HOME" ]; then
  echo "home exists"
elif [ -f /etc/passwd ]; then
  echo "passwd"
else
  echo "neither"
fi

for f in a b c; do
  echo "$f"
done

while read -r line; do
  echo "$line"
done

case "$1" in
  start)
    echo "starting"
    ;;
  *)
    echo "usage"
    ;;
esac
