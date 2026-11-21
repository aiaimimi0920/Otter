import { describe, expect, it } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const packageJson = JSON.parse(
  readFileSync(resolve(process.cwd(), "package.json"), "utf8"),
) as {
  scripts: Record<string, string>;
};

describe("Otter developer entrypoints contract", () => {
  it("uses package.json for the local verification gate and drops one-off wrapper launchers", () => {
    const otterRoot = process.cwd();
    const verifyWrapperPath = resolve(otterRoot, "verify-otter-full.bat");
    const captureWrapperPath = resolve(otterRoot, "start-otter-capture-test.bat");
    const readme = readFileSync(resolve(otterRoot, "README.md"), "utf8");
    const docsIndex = readFileSync(resolve(otterRoot, "docs/README.md"), "utf8");
    const developmentGuide = readFileSync(resolve(otterRoot, "docs/guide/development.md"), "utf8");
    const verifyScript = packageJson.scripts["verify:local"] ?? "";

    expect(existsSync(verifyWrapperPath)).toBe(false);
    expect(existsSync(captureWrapperPath)).toBe(false);

    expect(packageJson.scripts.build).toContain("cargo build --locked --release --workspace --manifest-path native/Cargo.toml");
    expect(packageJson.scripts.dev).toContain("cargo run --locked --manifest-path native/Cargo.toml --bin otter-native");
    expect(verifyScript).toContain("cargo fmt --all --check --manifest-path native/Cargo.toml");
    expect(verifyScript).toContain("cargo test --locked --workspace --manifest-path native/Cargo.toml");
    expect(verifyScript.indexOf("cargo test")).toBeLessThan(verifyScript.indexOf("npm run build"));
    expect(verifyScript).not.toMatch(/src-tauri|build-otter-release/);

    expect(readme).toContain("./docs/README.md");
    expect(docsIndex).toContain("guide/development.md");
    expect(developmentGuide).toContain("npm run verify:local");
    expect(developmentGuide).not.toContain("verify-otter-full.bat");
    expect(developmentGuide).not.toContain("start-otter-capture-test.bat");
    expect(developmentGuide).not.toContain("scripts\\run-vitest.cmd");
    expect(readme).not.toContain("verify-otter-full.bat");
    expect(readme).not.toContain("start-otter-capture-test.bat");
  });
});
