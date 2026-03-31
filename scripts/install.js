#!/usr/bin/env node
/**
 * Joshify npm/bun install script
 * Builds and installs the Rust binary from source
 */

const { execSync } = require('child_process');
const { existsSync } = require('fs');
const path = require('path');
const os = require('os');

const BIN_NAME = 'joshify';
const REPO_URL = 'https://github.com/bigknoxy/joshify.git';

console.log('⚡ Joshify Installer (npm/bun) ⚡');
console.log('================================');
console.log('');

// Check for Rust
try {
    execSync('cargo --version', { stdio: 'ignore' });
    console.log('✓ Rust found');
} catch (e) {
    console.error('✗ Rust not found. Please install Rust from https://rustup.rs');
    process.exit(1);
}

// Install from source
const tempDir = require('fs').mkdtempSync(path.join(os.tmpdir(), 'joshify-'));
console.log('Building Joshify from source...');

try {
    execSync(`git clone ${REPO_URL}`, { cwd: tempDir, stdio: 'inherit' });
    execSync('cargo install --path .', {
        cwd: path.join(tempDir, 'joshify'),
        stdio: 'inherit'
    });
    console.log('');
    console.log('✓ Joshify installed successfully!');
    console.log(`Run '${BIN_NAME}' to start the app.`);
} catch (e) {
    console.error('✗ Installation failed:', e.message);
    process.exit(1);
} finally {
    // Cleanup
    try {
        require('fs').rmSync(tempDir, { recursive: true, force: true });
    } catch (e) {}
}
