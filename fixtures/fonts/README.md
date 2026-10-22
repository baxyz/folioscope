# Test fonts

`Inter-Latin.ttf` and `Inter-Latin.woff` are [Inter](https://github.com/rsms/inter) (SIL Open
Font License 1.1, see [OFL.txt](OFL.txt); no reserved font name), taken from Google Fonts'
`Inter[opsz,wght].ttf`, with the optical size pinned at 14 and the characters cut down to
Basic Latin, Latin-1 and a few punctuation marks. The weight axis stays: one file, weights 100
to 900.

Made with fontTools:

```sh
fonttools varLib.instancer 'Inter[opsz,wght].ttf' opsz=14 -o inter-wght.ttf
pyftsubset inter-wght.ttf --unicodes=U+0020-007E,U+00A0-00FF,U+2026,U+2019,U+201C,U+201D \
  --layout-features=kern,liga,calt --output-file=Inter-Latin.ttf   # and --flavor=woff
```
