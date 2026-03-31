#!/usr/bin/env node
/**
 * Joshify CLI wrapper
 * This script is linked by npm/bun as the 'joshify' command
 */

const { spawn } = require('child_process');
const { execSync } = require('child_process');

// Check if joshify binary exists in cargo bin
const cargoBin = process.env.HOME + '/.cargo/bin/joshify';

try {
    execSync(`test -f ${cargoBin}`);
    const child = spawn(cargoBin, process.argv.slice(2), {
        stdio: 'inherit'
    });
    child.on('exit', (code) => {
        process.exit(code);
    });
} catch (e) {
    console.error('Joshify not found. Run: npm install -g joshify');
    process.exit(1);
}
