# Credits

Cut Craft is built with free libraries, pictures and icons made by other people. Thank you to
all of them. Each keeps its own licence, given here in full. The same credits are in the app,
under **Help ▸ Credits** (also a button in **Help ▸ About Cut Craft**).

- [libraries.md](libraries.md): every Rust library Cut Craft is built with, grouped by
  licence, with the licence text.
- [other.md](other.md): the bundled pictures and icons, and the Rust standard library.

Cut Craft's own licence is the LICENSE file beside this folder.

## Keeping it up to date

`libraries.md` is made from `Cargo.lock` by [cargo-about](https://github.com/EmbarkStudios/cargo-about)
(settings in `about.toml`, layout in `about.hbs`), so never edit it by hand. After any change
to `Cargo.lock`, run `scripts/credits` and commit the result; CI runs `scripts/credits --check`
and fails until you do. A library whose licence is not in `about.toml`'s `accepted` list
(GPL, for example) makes it fail too, so it is noticed before it ships.

`other.md` is kept by hand: add anything bundled that is not a Rust library (a picture, an icon,
a font file, a model) with its credit and licence.
