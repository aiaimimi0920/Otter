# Third-Party Notices

Otter is licensed under the MIT License. The complete resolved dependency
versions used by the public build are recorded in `package-lock.json` and
`src-tauri/Cargo.lock`.

Otter does not intentionally include proprietary Otter-owned components. The
project's direct dependencies and vendored source are distributed under
open-source licenses. Important bundled-source attribution follows.

## Cap `scap-*` capture crates

- Components: `scap-targets`, `scap-direct3d`
- Source family: <https://github.com/CapSoftware/Cap>
- License: MIT
- Copyright: Copyright (c) 2023 Cap Software, Inc.
- Local attribution: `src-tauri/crates/CAPTURE_CRATES_SOURCE.md`
- License text: `src-tauri/crates/LICENSE_CAP_SCAP_MIT`

Only the `scap-*` crate source covered by Cap's MIT grant was imported. Otter
does not import or relicense the rest of the Cap application under Otter's MIT
license.

## CrabNebula `drag` crate

- Component: `drag` 2.1.1
- Copyright holder: CrabNebula Ltd.
- License: Apache-2.0 OR MIT
- License texts:
  - `src-tauri/crates/drag/LICENSE_APACHE-2.0`
  - `src-tauri/crates/drag/LICENSE_MIT`

## Framework and package dependencies

- Tauri and its Rust/JavaScript ecosystem dependencies are used under the
  licenses declared by their package manifests and lockfile entries.
- SolidJS and frontend dependencies are used under the licenses declared by
  their package manifests and `package-lock.json` entries.
- `src-tauri/Cargo.lock` pins the resolved Rust dependency versions. Their SPDX
  expressions or license-file declarations come from the corresponding package
  manifests exposed by Cargo metadata, not from the lockfile itself.

The Windows operating system, Windows SDK interfaces, and Microsoft Edge
WebView2 runtime are system/platform components. The portable release archive
does not redistribute a standalone WebView2 installer or private Windows SDK
component.

Run `npm run audit:licenses` after `npm ci` to check the current direct npm
manifests, the resolved Rust graph, and local Rust crate license metadata.

## Experimental Windows-native JavaScript runtime

The independent `native/Cargo.lock` pins rquickjs, rquickjs-core and
rquickjs-sys 0.14.0. rquickjs-sys incorporates QuickJS-NG 0.16.2 at upstream
commit `0fdea21ff1090084e91dad812b343d92e79ba9d9`. These components belong to
the opt-in development runtime, not the existing public Tauri build. Their
original MIT notices follow. Transitive Rust packages retain the licenses
recorded by the resolved Cargo manifests; this section is not a complete
candidate distribution inventory or release approval.

### rquickjs

MIT License

Copyright (c) 2020 Mees Delzenne
Copyright (c) 2025 Rquickjs Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

### QuickJS-NG

The MIT License (MIT)

Copyright (c) 2017-2026 Fabrice Bellard
Copyright (c) 2017-2024 Charlie Gordon
Copyright (c) 2023-2026 Ben Noordhuis
Copyright (c) 2023-2026 Saúl Ibarra Corretgé

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
