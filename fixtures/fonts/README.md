# Test fonts

`Inter-Latin.ttf` and `Inter-Latin.woff` are [Inter](https://github.com/rsms/inter) (SIL Open
Font License 1.1, see [OFL.txt](OFL.txt); no reserved font name), taken from Google Fonts'
`Inter[opsz,wght].ttf`, with the optical size pinned at 14 and the characters cut down to
Basic Latin, Latin-1, a few punctuation marks, and the symbols the demos use (arrows, check,
bullet, star, heart, play, keyboard keys). The weight axis stays: one file, weights 100
to 900.

Made with fontTools:

```sh
fonttools varLib.instancer 'Inter[opsz,wght].ttf' opsz=14 -o inter-wght.ttf
pyftsubset inter-wght.ttf --unicodes=U+0020-007E,U+00A0-00FF,U+2026,U+2019,U+201C,U+201D,\
U+2022,U+2039,U+203A,U+2190-2193,U+21E7,U+2303,U+2318,U+2325,U+23CE,U+25B6,U+2605,U+2665,\
U+2713,U+2764 \
  --layout-features=kern,liga,calt --output-file=Inter-Latin.ttf   # and --flavor=woff
```
