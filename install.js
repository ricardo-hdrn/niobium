#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");

const VERSION = require(path.join(__dirname, "package.json")).version;
const REPO = "ricardo-hdrn/niobium";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  darwin: "apple-darwin",
  linux: "unknown-linux-gnu",
  win32: "pc-windows-msvc",
};

const ARCH_MAP = {
  x64: "x86_64",
  arm64: "aarch64",
};

// Release asset naming uses platform keys, not Rust target triples
const PLATFORM_KEY_MAP = {
  darwin: { x64: "darwin-x64", arm64: "darwin-arm64" },
  linux: { x64: "linux-x64" },
  win32: { x64: "win32-x64" },
};

function getAssetName() {
  const keys = PLATFORM_KEY_MAP[process.platform];
  if (!keys) {
    throw new Error(`Unsupported platform: ${process.platform}`);
  }
  const key = keys[process.arch];
  if (!key) {
    throw new Error(`Unsupported architecture: ${process.platform}-${process.arch}`);
  }

  const ext = process.platform === "win32" ? "zip" : "tar.gz";
  return { name: `niobium-mcp-${key}.${ext}`, key };
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return download(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const { name } = getAssetName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${name}`;

  console.log(`Downloading niobium v${VERSION} for ${process.platform}-${process.arch}...`);

  const data = await download(url);

  fs.mkdirSync(BIN_DIR, { recursive: true });

  if (name.endsWith(".zip")) {
    const tmpZip = path.join(BIN_DIR, "tmp.zip");
    fs.writeFileSync(tmpZip, data);
    execSync(
      `powershell -Command "Expand-Archive -Force '${tmpZip}' '${BIN_DIR}'"`,
      { stdio: "inherit" }
    );
    fs.unlinkSync(tmpZip);

    // Expand-Archive creates bin/ subdirectory — move contents up
    const nested = path.join(BIN_DIR, "bin");
    if (fs.existsSync(nested)) {
      for (const entry of fs.readdirSync(nested)) {
        fs.renameSync(path.join(nested, entry), path.join(BIN_DIR, entry));
      }
      fs.rmdirSync(nested);
    }
  } else {
    const tmpTar = path.join(BIN_DIR, "tmp.tar.gz");
    fs.writeFileSync(tmpTar, data);
    // --strip-components=1 removes the "bin/" prefix from the tarball
    execSync(`tar xzf "${tmpTar}" --strip-components=1 -C "${BIN_DIR}"`, {
      stdio: "inherit",
    });
    fs.unlinkSync(tmpTar);
  }

  // Ensure the native binary is executable
  const nativeBin = path.join(BIN_DIR, "niobium");
  if (process.platform !== "win32" && fs.existsSync(nativeBin)) {
    fs.chmodSync(nativeBin, 0o755);
  }

  console.log(`Installed niobium to ${BIN_DIR}`);
}

main().catch((err) => {
  console.error(`Failed to install niobium: ${err.message}`);
  process.exit(1);
});
