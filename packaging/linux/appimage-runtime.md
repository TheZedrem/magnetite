# The AppImage runtime

This AppImage starts with AppImage's runtime, a small program that mounts
the rest of the file and starts Magnetite from it. It is not part of
Magnetite and is distributed unchanged, as built by the AppImage project:

- Binary: <https://github.com/AppImage/type2-runtime/releases/tag/20251108>
- Source and build scripts, including the libraries below and the patch to
  libfuse: <https://github.com/AppImage/type2-runtime/tree/20251108>

The runtime itself is under the MIT licence (Copyright (c) 2004-23
probonopd). It is statically linked with:

| Library | Licence | Source |
| --- | --- | --- |
| libfuse 3 (patched) | LGPL-2.1 | <https://github.com/libfuse/libfuse> |
| squashfuse | BSD-2-Clause | <https://github.com/vasi/squashfuse> |
| zstd | BSD-3-Clause | <https://github.com/facebook/zstd> |
| zlib | Zlib | <https://zlib.net> |
| mimalloc | MIT | <https://github.com/microsoft/mimalloc> |
| musl libc | MIT | <https://musl.libc.org> |

The text of the LGPL-2.1 is in `CREDITS.html` (FFmpeg's entry). libfuse can
be replaced by rebuilding the runtime from the source above and putting it
in front of this file's squashfs image (`--appimage-offset` prints where the
image starts).
