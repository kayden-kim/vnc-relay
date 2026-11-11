# 고지

이 페이지에서 도움말 사이트에 싣는 제3자 구성요소와 라이선스를 확인할 수 있다. 이 고지는 도움말 사이트에만 적용한다. 실행파일과 배포 묶음의 고지는 묶음 안의 `THIRD-PARTY-NOTICES.txt`와 `LICENSE`에 있다.

## 구성요소

| 구성요소 | 버전 | 라이선스 | 출처 | 라이선스 전문 |
| --- | --- | --- | --- | --- |
| mdBook 프런트엔드(HTML 템플릿, CSS, JavaScript, 코드 강조 스타일) | 0.5.4 | MPL-2.0 | [rust-lang/mdBook](https://github.com/rust-lang/mdBook/tree/v0.5.4) | [mdBook LICENSE](https://github.com/rust-lang/mdBook/blob/v0.5.4/LICENSE) |
| Font Awesome Free 아이콘 | 6.2.0 | CC BY 4.0 | [FortAwesome/Font-Awesome](https://github.com/FortAwesome/Font-Awesome/tree/6.2.0) | [Font Awesome Free License](https://fontawesome.com/license/free) |
| highlight.js | 10.1.1 | BSD-3-Clause | [highlightjs/highlight.js](https://github.com/highlightjs/highlight.js/tree/10.1.1) | [highlight.js LICENSE](https://github.com/highlightjs/highlight.js/blob/10.1.1/LICENSE) |
| clipboard.js | 2.0.4 | MIT | [zenorocha/clipboard.js](https://github.com/zenorocha/clipboard.js/tree/v2.0.4) | [이 페이지의 clipboard.js 라이선스](#clipboardjs-라이선스) |
| Pretendard 글꼴 | 1.3.9 | OFL-1.1 | [orioncactus/pretendard](https://github.com/orioncactus/pretendard/tree/v1.3.9) | [사이트에 함께 싣는 LICENSE.txt](../assets/fonts/LICENSE.txt) |

## 구성요소별 안내

- mdBook 프런트엔드 파일은 고치지 않고 싣는다. 사이트의 모양과 동작은 이 프로젝트가 더한 테마 파일로 바꾼다. 코드 강조 스타일은 Atelier Dune Light(Bram de Haan), Base16과 Tomorrow(Chris Kempson), Ayu(Dempfi) 테마를 바탕으로 한 mdBook 파일이다.
- Font Awesome Free 아이콘은 mdBook 템플릿이 페이지에 넣는다. 아이콘 저작자는 Fonticons, Inc.(Font Awesome)다.
- highlight.js는 코드 블록의 문법을 구분한다. 저작권은 Ivan Sagalaev에게 있다.
- clipboard.js는 mdBook 템플릿이 불러온다. 저작권은 Zeno Rocha에게 있다.
- Pretendard 글꼴은 운영 대시보드와 같은 파일이다. 저작권 고지와 라이선스 전문은 글꼴 파일과 같은 폴더의 `LICENSE.txt`에 있다.

도움말 본문과 이 프로젝트가 더한 테마 파일은 vnc-relay 저장소의 [MIT 라이선스](https://github.com/kayden-kim/vnc-relay/blob/main/LICENSE)를 따른다.

## clipboard.js 라이선스

clipboard.js 2.0.4는 파일 머리말에 `Licensed MIT © Zeno Rocha`를 적는다. MIT 라이선스 전문은 다음과 같다.

MIT License

Copyright (c) Zeno Rocha

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
