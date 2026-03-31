#!/usr/bin/env node
/**
 * Joshify npm/bun uninstall script
 */

const { execSync } = require('child_process');

console.log('⚡ Joshify Uninstaller ⚡');
console.log('=======================');
console.log('');

try {
    console.log('Removing cargo binary...');
    execSync('cargo uninstall joshify', { stdio: 'inherit' });
} catch (e) {
    console.log('Binary not found or already removed');
}

// Remove config and cache
const fs = require('fs');
const path = require('path');
const os = require('os');

const configDir = path.join(os.homedir(), '.config', 'joshify');
const cacheDir = path.join(os.homedir(), '.cache', 'joshify');

if (fs.existsSync(configDir)) {
    console.log('Removing config:', configDir);
    fs.rmSync(configDir, { recursive: true, force: true });
}

if (fs.existsSync(cacheDir)) {
    console.log('Removing cache:', cacheDir);
    fs.rmSync(cacheDir, { recursive: true, force: true });
}

console.log('');
console.log('✓ Joshify uninstalled successfully!');
