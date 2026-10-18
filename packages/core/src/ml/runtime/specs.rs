//! Pinned official ONNX Runtime builds (github.com/microsoft/onnxruntime releases), one per
//! release target. `ort` 2.0.0-rc.13 asks for C API 17, so any runtime >= 1.17 loads; 1.28.3
//! matches what `ort` itself builds against. Microsoft stopped publishing Intel macOS builds
//! after 1.23.2, so that target stays there. Every URL and checksum here was checked by
//! downloading the asset.

use super::{Archive, RuntimeSpec};

pub const RUNTIMES: [RuntimeSpec; 6] = [
    RuntimeSpec {
        os: "linux",
        arch: "x86_64",
        version: "1.28.3",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.3/onnxruntime-linux-x64-1.28.3.tgz",
        archive: Archive::Tgz,
        sha256: "db14e4863bd37893fc59729d986ab2a0d043d10b7d44da1913c4982b7e3d009c",
        size: 9_130_098,
        member: "onnxruntime-linux-x64-1.28.3/lib/libonnxruntime.so.1.28.3",
        lib_name: "libonnxruntime.so",
        lib_sha256: "5a1ce74e56e8c4b278d5a9e9d12b7192d9f70811084dde9616da4fa1710efc5a",
    },
    RuntimeSpec {
        os: "linux",
        arch: "aarch64",
        version: "1.28.3",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.3/onnxruntime-linux-aarch64-1.28.3.tgz",
        archive: Archive::Tgz,
        sha256: "6c6b1ae96d7b0be9f555092857c6a4f2b0b5587a5298d732d9e52e8253038ce7",
        size: 8_123_797,
        member: "onnxruntime-linux-aarch64-1.28.3/lib/libonnxruntime.so.1.28.3",
        lib_name: "libonnxruntime.so",
        lib_sha256: "c17d9c3b522c69594ef2b077d7f27a92ff0635d9df0c68c8997b7cad7bf71c61",
    },
    RuntimeSpec {
        os: "macos",
        arch: "aarch64",
        version: "1.28.3",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.3/onnxruntime-osx-arm64-1.28.3.tgz",
        archive: Archive::Tgz,
        sha256: "c436bd9f47dbce6f6311ccc21de97f829b3c862c09d24aa321958cb72f0a1d32",
        size: 33_057_743,
        member: "onnxruntime-osx-arm64-1.28.3/lib/libonnxruntime.1.28.3.dylib",
        lib_name: "libonnxruntime.dylib",
        lib_sha256: "0c8707e83b3389849d03a3264308e905169b94b546f84df6bf662649bd89c3bb",
    },
    RuntimeSpec {
        os: "macos",
        arch: "x86_64",
        version: "1.23.2",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-x86_64-1.23.2.tgz",
        archive: Archive::Tgz,
        sha256: "d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6",
        size: 11_676_322,
        member: "onnxruntime-osx-x86_64-1.23.2/lib/libonnxruntime.1.23.2.dylib",
        lib_name: "libonnxruntime.dylib",
        lib_sha256: "8c9c78de65ea3786f987c0d980e9c1b13a3a5fbc6b3e2965ba05b450e6e4c054",
    },
    RuntimeSpec {
        os: "windows",
        arch: "x86_64",
        version: "1.28.3",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.3/onnxruntime-win-x64-1.28.3.zip",
        archive: Archive::Zip,
        sha256: "1d6fab48e85f948436af7c8c971d2c145cf224e2c444755dec894f8b0de11a83",
        size: 78_606_946,
        member: "onnxruntime-win-x64-1.28.3/lib/onnxruntime.dll",
        lib_name: "onnxruntime.dll",
        lib_sha256: "4d2774a5f64e4a230b16b74b167171e92f65b0683c19680c1d04fb284b92a6e2",
    },
    RuntimeSpec {
        os: "windows",
        arch: "aarch64",
        version: "1.28.3",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.3/onnxruntime-win-arm64-1.28.3.zip",
        archive: Archive::Zip,
        sha256: "76d5e5a23fb7fdbc2ca0c3214bad1b58b2a41016c7052e74e60ed8004bea71b1",
        size: 79_730_664,
        member: "onnxruntime-win-arm64-1.28.3/lib/onnxruntime.dll",
        lib_name: "onnxruntime.dll",
        lib_sha256: "20bcd3d4718376ef449d9520ec074d3c73938655e74f42c410de0e5aab6a8ed5",
    },
];
