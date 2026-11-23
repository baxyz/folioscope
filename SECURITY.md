# Security

## Reporting a vulnerability

Please do not open a public issue. Report it privately, through
[GitHub's private vulnerability reporting](https://github.com/baxyz/folioscope/security/advisories/new)
on this repository. You will get an answer within 7 days; a confirmed flaw is fixed in a new
release as soon as possible, and credited to you if you wish.

Fixes go into the latest release only: `folioscope` on npm and crates.io, `@baxyz/folioscope` on JSR.

## What Folioscope protects

A design file is data from someone else: a site may play a file its author did not make, and a
visitor may drop any file on a page that plays it. Folioscope is built so that a design file,
whatever it says:

- **runs nothing** in the page that plays it, on the site that serves what it was baked into, or
  on the machine that bakes it;
- **reads nothing** but itself: no path of the machine, no address on the network;
- **leads nowhere** but to a web address, an e-mail or a phone number, when it holds a link.

A way around any of these is a vulnerability. So is a crash of the bake or of the player on a
crafted file (a panic, an endless loop). How it is done is in the specifications of
[the bake](docs/spec/bake.md#untrusted-files) and of [the player](docs/spec/player.md#lite-player),
and `crates/folioscope/tests/untrusted.rs` holds the tests.

## What it does not

- **The cost of a bake.** An archive can unzip to gigabytes and a drawing can take minutes. Give a
  bake of files from strangers limits on memory and time, as for any converter. In the browser, a
  file is baked in a worker of the visitor's own tab.
- **Your page's own code.** `src`, `poster` and the fonts are addresses your page gives: the
  player fetches what you name.
