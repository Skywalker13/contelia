#!/bin/sh

find . -iname "*.png" -exec sh -c '
  pngquant --quality=65-80 --skip-if-larger --force --ext .png -- "$1"
  optipng -o7 -strip all "$1"
' _ {} \;

find . -iname "*.jpg" -o -iname "*.jpeg" | xargs -I{} \
  jpegoptim --max=80 --strip-all --all-progressive "{}"
