# Notices

Use this page to find the third-party components that this help site ships and their licenses. These notices apply to the help site only. The notices for the executable and the release bundle are in `THIRD-PARTY-NOTICES.txt` and `LICENSE` in the bundle.

## Components

| Component | Version | License | Source | License text |
| --- | --- | --- | --- | --- |
| mdBook front end (HTML template, CSS, JavaScript, code highlight styles) | 0.5.4 | MPL-2.0 | [rust-lang/mdBook](https://github.com/rust-lang/mdBook/tree/v0.5.4) | [mdBook LICENSE](https://github.com/rust-lang/mdBook/blob/v0.5.4/LICENSE) |
| Font Awesome Free icons | 6.2.0 | CC BY 4.0 | [FortAwesome/Font-Awesome](https://github.com/FortAwesome/Font-Awesome/tree/6.2.0) | [Font Awesome Free License](https://fontawesome.com/license/free) |
| highlight.js | 10.1.1 | BSD-3-Clause | [highlightjs/highlight.js](https://github.com/highlightjs/highlight.js/tree/10.1.1) | [highlight.js LICENSE](https://github.com/highlightjs/highlight.js/blob/10.1.1/LICENSE) |
| clipboard.js | 2.0.4 | MIT | [zenorocha/clipboard.js](https://github.com/zenorocha/clipboard.js/tree/v2.0.4) | [clipboard.js license on this page](#clipboardjs-license) |
| Pretendard font | 1.3.9 | OFL-1.1 | [orioncactus/pretendard](https://github.com/orioncactus/pretendard/tree/v1.3.9) | [LICENSE.txt shipped with the site](../assets/fonts/LICENSE.txt) |

## About each component

- The mdBook front-end files ship unmodified. Theme files that this project adds change the look and behavior of the site. The code highlight styles are mdBook files based on the Atelier Dune Light (Bram de Haan), Base16 and Tomorrow (Chris Kempson), and Ayu (Dempfi) themes.
- The mdBook template puts the Font Awesome Free icons into the pages. The icons are by Fonticons, Inc. (Font Awesome).
- highlight.js marks the syntax in code blocks. Copyright Ivan Sagalaev.
- The mdBook template loads clipboard.js. Copyright Zeno Rocha.
- The Pretendard font files are the same files as in the operations dashboard. The copyright notice and the full license text are in `LICENSE.txt` in the same folder as the font files.

The help text and the theme files that this project adds are under the [MIT License](https://github.com/kayden-kim/vnc-relay/blob/main/LICENSE) of the vnc-relay repository.

## clipboard.js license

clipboard.js 2.0.4 states `Licensed MIT © Zeno Rocha` in its file header. The full MIT License text follows.

MIT License

Copyright (c) Zeno Rocha

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
