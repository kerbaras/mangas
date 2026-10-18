#!/usr/bin/env python3
"""Packs PNG renders into an .icns with every type iconutil writes (1x and @2x).

Usage: icns.py OUT.icns DIR  (DIR holds icns-<px>.png for 16 32 64 128 256 512 1024)
"""
import struct
import sys
from pathlib import Path

# (type, pixel size): 16/32/128/256/512 at 1x, then the @2x slots.
TYPES = [
    (b"icp4", 16), (b"icp5", 32), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
    (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512), (b"ic10", 1024),
]


def pack(entries):
    body = b"".join(t + struct.pack(">I", len(d) + 8) + d for t, d in entries)
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def main(out, src):
    entries = [(t, (Path(src) / f"icns-{px}.png").read_bytes()) for t, px in TYPES]
    Path(out).write_bytes(pack(entries))


if __name__ == "__main__":
    main(*sys.argv[1:3])
